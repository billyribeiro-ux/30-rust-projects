//! Stripe webhook receiver. Raw-body extractor + signature verify +
//! idempotency on `stripe_events.event_id`. Dispatches by event type.

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::stripe::webhook;

pub fn router() -> Router<AppState> {
    Router::new().route("/webhook", post(webhook_handler))
}

#[derive(Debug, Deserialize)]
struct Event<'a> {
    id: &'a str,
    #[serde(rename = "type")]
    kind: &'a str,
    data: EventData,
}

#[derive(Debug, Deserialize)]
struct EventData {
    object: serde_json::Value,
}

async fn webhook_handler(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let now = chrono::Utc::now().timestamp();
    webhook::verify(&body, sig, &s.stripe_webhook_secret, now).map_err(|e| {
        tracing::warn!(?e, "webhook signature verify failed");
        AppError::Unauthorized
    })?;

    let event: Event = serde_json::from_slice(&body)
        .map_err(|e| AppError::Validation(format!("bad json: {e}")))?;

    // Idempotency — ON CONFLICT DO NOTHING. If the row already existed
    // the rows-affected count is 0 and we short-circuit.
    let inserted = sqlx::query!(
        r#"
        INSERT INTO stripe_events (event_id) VALUES ($1)
        ON CONFLICT (event_id) DO NOTHING
        "#,
        event.id
    )
    .execute(&s.pool)
    .await?
    .rows_affected();
    if inserted == 0 {
        tracing::info!(event_id = event.id, "duplicate stripe event ignored");
        return Ok(StatusCode::OK);
    }

    dispatch(&s, event.kind, &event.data.object).await?;

    Ok(StatusCode::OK)
}

async fn dispatch(s: &AppState, kind: &str, obj: &serde_json::Value) -> AppResult<()> {
    match kind {
        "checkout.session.completed" => on_checkout_completed(s, obj).await,
        "customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted" => on_subscription_change(s, obj).await,
        "invoice.payment_failed" => on_payment_failed(s, obj).await,
        "invoice.paid" => on_invoice_paid(s, obj).await,
        _ => {
            tracing::debug!(kind, "stripe event ignored (unhandled type)");
            Ok(())
        }
    }
}

async fn on_checkout_completed(s: &AppState, obj: &serde_json::Value) -> AppResult<()> {
    let customer = obj
        .get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let subscription = obj.get("subscription").and_then(|v| v.as_str());
    let email = obj
        .get("customer_details")
        .and_then(|v| v.get("email"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    if customer.is_empty() || email.is_empty() {
        tracing::warn!(?obj, "checkout.session.completed missing customer/email");
        return Ok(());
    }
    let sub_row = sqlx::query!("SELECT id FROM subscribers WHERE email = $1", email)
        .fetch_optional(&s.pool)
        .await?;
    let subscriber_id = match sub_row {
        Some(r) => r.id,
        None => {
            // Auto-create — Stripe Checkout collected the email; we should honour it.
            sqlx::query!(
                "INSERT INTO subscribers (email) VALUES ($1) RETURNING id",
                email
            )
            .fetch_one(&s.pool)
            .await?
            .id
        }
    };
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (subscriber_id, stripe_customer_id, stripe_subscription_id, plan, status)
        VALUES ($1, $2, $3, 'pro', 'active')
        ON CONFLICT (subscriber_id) DO UPDATE
        SET stripe_customer_id = EXCLUDED.stripe_customer_id,
            stripe_subscription_id = EXCLUDED.stripe_subscription_id,
            plan = 'pro',
            status = 'active',
            past_due_since = NULL,
            updated_at = now()
        "#,
        subscriber_id,
        customer,
        subscription,
    )
    .execute(&s.pool)
    .await?;
    tracing::info!(%email, "subscriber promoted to pro");
    Ok(())
}

async fn on_subscription_change(s: &AppState, obj: &serde_json::Value) -> AppResult<()> {
    let stripe_sub_id = obj.get("id").and_then(|v| v.as_str()).unwrap_or_default();
    let status = obj
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("incomplete");
    let cancel_at_period_end = obj
        .get("cancel_at_period_end")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let current_period_end = obj
        .get("current_period_end")
        .and_then(|v| v.as_i64())
        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0));
    let plan = if matches!(status, "canceled" | "incomplete_expired") {
        "free"
    } else {
        "pro"
    };
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = $2,
            cancel_at_period_end = $3,
            current_period_end = $4,
            plan = $5,
            updated_at = now()
        WHERE stripe_subscription_id = $1
        "#,
        stripe_sub_id,
        status,
        cancel_at_period_end,
        current_period_end,
        plan
    )
    .execute(&s.pool)
    .await?;
    Ok(())
}

async fn on_payment_failed(s: &AppState, obj: &serde_json::Value) -> AppResult<()> {
    let customer = obj
        .get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'past_due',
            past_due_since = COALESCE(past_due_since, now()),
            updated_at = now()
        WHERE stripe_customer_id = $1
        "#,
        customer
    )
    .execute(&s.pool)
    .await?;
    tracing::warn!(%customer, "subscription past_due");
    Ok(())
}

async fn on_invoice_paid(s: &AppState, obj: &serde_json::Value) -> AppResult<()> {
    let customer = obj
        .get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'active',
            past_due_since = NULL,
            updated_at = now()
        WHERE stripe_customer_id = $1
        "#,
        customer
    )
    .execute(&s.pool)
    .await?;
    Ok(())
}

/// One-off SQL: returns subscriber ids whose grace period ended, so the
/// background reaper can flip them to free. Used by the dunning task and
/// asserted in tests.
pub async fn find_past_due_to_downgrade(pool: &sqlx::PgPool) -> AppResult<Vec<uuid::Uuid>> {
    let rows = sqlx::query!(
        r#"
        SELECT subscriber_id
        FROM subscriptions
        WHERE status = 'past_due'
          AND past_due_since IS NOT NULL
          AND past_due_since < now() - INTERVAL '14 days'
        "#
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.subscriber_id).collect())
}

pub async fn downgrade_to_free(pool: &sqlx::PgPool, subscriber_id: uuid::Uuid) -> AppResult<()> {
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET plan = 'free', status = 'canceled', past_due_since = NULL, updated_at = now()
        WHERE subscriber_id = $1
        "#,
        subscriber_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
