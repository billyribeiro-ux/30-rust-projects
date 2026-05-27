//! POST /api/stripe/webhook
//!
//! The endpoint Stripe POSTs to when a checkout completes. Three layers:
//!  1) raw-body extraction (no JSON parsing yet — we need the literal
//!     bytes to verify HMAC).
//!  2) signature verification against STRIPE_WEBHOOK_SECRET, with a
//!     5-minute replay window.
//!  3) idempotent dispatch: INSERT INTO stripe_events ON CONFLICT DO
//!     NOTHING. If the row already existed (duplicate webhook), we
//!     return 200 without re-fulfilling.
//!
//! If anything inside the dispatch fails, we return 5xx so Stripe will
//! retry — but the event_id is still in stripe_events. On retry our
//! INSERT skips, so we won't double-fulfil. The lesson here: idempotency
//! key in the DB > "process exactly once" guarantees.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::email;
use crate::error::{AppError, AppResult};
use crate::signing;
use crate::state::AppState;
use crate::stripe::types::{CheckoutSession, RefundedCharge, WebhookEvent};
use crate::stripe::webhook;

pub fn router() -> Router<AppState> {
    Router::new().route("/webhook", post(handle_webhook))
}

const SIGNED_LINK_TTL_HOURS: i64 = 24;

async fn handle_webhook(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    // 1) Pull the signature header. Missing → 400, NOT 401 — this is the
    //    convention Stripe's own examples follow.
    let sig_header = headers
        .get("stripe-signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Validation("missing Stripe-Signature header".into()))?;

    // 2) Verify HMAC.
    webhook::verify(
        s.stripe_webhook_secret.as_slice(),
        sig_header,
        &body,
        webhook::now_unix(),
    )
    .map_err(|e| {
        tracing::warn!(?e, "stripe webhook signature rejected");
        // Always 400 on sig failures, never 401: 401 would imply credentials
        // are needed, but the signature IS the credential.
        AppError::Validation(format!("invalid signature: {e:?}"))
    })?;

    // 3) Parse JSON now that we trust the body.
    let event: WebhookEvent = serde_json::from_slice(&body)
        .map_err(|e| AppError::Validation(format!("invalid event JSON: {e}")))?;

    // 4) Idempotency. We attempt to INSERT first; if there was already a
    //    row for this event_id we short-circuit and return 200.
    let payload_value: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
    let inserted = sqlx::query!(
        r#"
        INSERT INTO stripe_events (event_id, type, payload)
        VALUES ($1, $2, $3)
        ON CONFLICT (event_id) DO NOTHING
        RETURNING event_id
        "#,
        event.id,
        event.event_type,
        payload_value,
    )
    .fetch_optional(&s.pool)
    .await?;

    if inserted.is_none() {
        tracing::info!(event_id = %event.id, "duplicate webhook delivery — skipping");
        return Ok(StatusCode::OK);
    }

    // 5) Dispatch on event type. Only the types we care about; anything
    //    else returns 200 (we've stored the event for audit).
    match event.event_type.as_str() {
        "checkout.session.completed" => {
            let session: CheckoutSession = serde_json::from_value(event.data.object.clone())
                .map_err(|e| AppError::Validation(format!("bad checkout session: {e}")))?;
            fulfil_checkout(&s, &session).await?;
        }
        "charge.refunded" => {
            let charge: RefundedCharge = serde_json::from_value(event.data.object.clone())
                .map_err(|e| AppError::Validation(format!("bad charge: {e}")))?;
            if let Some(pi) = charge.payment_intent {
                mark_refunded(&s, &pi).await?;
            }
        }
        other => {
            tracing::info!(event_type = other, "ignoring event type");
        }
    }

    Ok(StatusCode::OK)
}

/// Mark the order as fulfilled, mint a signed download link, send email.
///
/// Mutually exclusive with itself per session_id: the UPDATE only runs if
/// the order is still 'pending' or 'paid' (not already 'fulfilled'). So
/// even if idempotency at the events level somehow fails (e.g. a hand
/// re-POST), this gate also prevents double emails.
async fn fulfil_checkout(s: &AppState, session: &CheckoutSession) -> AppResult<()> {
    let email = session
        .customer_email
        .clone()
        .or_else(|| session.customer_details.as_ref().and_then(|c| c.email.clone()))
        .ok_or_else(|| AppError::Validation("no email on session".into()))?
        .to_lowercase();

    let mut tx = s.pool.begin().await?;

    // Find the order. Race-free: we lock the row.
    let order = sqlx::query!(
        r#"
        SELECT id, status
        FROM orders
        WHERE stripe_session_id = $1
        FOR UPDATE
        "#,
        session.id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        AppError::Validation(format!("no local order for session {}", session.id))
    })?;

    if order.status == "fulfilled" || order.status == "refunded" {
        tracing::info!(order_id = %order.id, "order already fulfilled; skipping");
        return Ok(());
    }

    let payment_status = session.payment_status.as_deref().unwrap_or("unpaid");
    let new_status = if payment_status == "paid" {
        "fulfilled"
    } else {
        "paid" // payment captured later (rare for one-time)
    };

    sqlx::query!(
        r#"
        UPDATE orders
        SET status = $1,
            stripe_payment_intent = $2,
            fulfilled_at = CASE WHEN $1 = 'fulfilled' THEN now() ELSE fulfilled_at END,
            customer_email = $3
        WHERE id = $4
        "#,
        new_status,
        session.payment_intent.as_deref(),
        email,
        order.id,
    )
    .execute(&mut *tx)
    .await?;

    // Generate a signed download link for each item in the order. (Today
    // every order has exactly one item, but we still iterate.)
    let items = sqlx::query!(
        r#"
        SELECT product_id, p.name AS product_name
        FROM order_items
        JOIN products p ON p.id = order_items.product_id
        WHERE order_id = $1
        "#,
        order.id,
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut links: Vec<(String, String)> = Vec::new(); // (product_name, full_url)
    let expires_at = Utc::now() + Duration::hours(SIGNED_LINK_TTL_HOURS);

    for item in items {
        let nonce = signing::random_nonce();
        let signed_token = signing::sign(&s.download_signing_secret, &nonce);
        let token_hash = signing::hash_token(&signed_token);

        sqlx::query!(
            r#"
            INSERT INTO download_links (id, order_id, product_id, token_hash, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            Uuid::new_v4(),
            order.id,
            item.product_id,
            token_hash,
            expires_at,
        )
        .execute(&mut *tx)
        .await?;

        let url = format!(
            "{}/d/{}",
            s.public_url.trim_end_matches('/'),
            signed_token
        );
        links.push((item.product_name, url));
    }

    tx.commit().await?;

    // Send the customer their links. We don't block fulfilment on this —
    // the email module logs failures and lets the user re-request from
    // the success page.
    for (name, url) in links {
        let (subj, body) = email::download_ready_body(
            &s.public_url,
            &name,
            &url,
            SIGNED_LINK_TTL_HOURS,
        );
        s.mailer.send(&email, &subj, body).await;
    }

    Ok(())
}

/// Flip the order to 'refunded' and revoke all active download links.
async fn mark_refunded(s: &AppState, payment_intent: &str) -> AppResult<()> {
    let mut tx = s.pool.begin().await?;
    let order = sqlx::query!(
        r#"
        SELECT id FROM orders WHERE stripe_payment_intent = $1 FOR UPDATE
        "#,
        payment_intent,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(order) = order else {
        tracing::warn!(payment_intent, "refund event with no matching order");
        return Ok(());
    };

    sqlx::query!(
        r#"
        UPDATE orders
        SET status = 'refunded', refunded_at = now()
        WHERE id = $1
        "#,
        order.id,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        UPDATE download_links
        SET revoked_at = now()
        WHERE order_id = $1 AND revoked_at IS NULL
        "#,
        order.id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
