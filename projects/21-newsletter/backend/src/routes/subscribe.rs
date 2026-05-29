//! `POST /api/subscribe` — entrypoint for a reader. Creates a
//! `subscribers` row + starts a Stripe Checkout session for the Pro
//! plan. Free tier doesn't need Stripe at all; we just create the
//! subscribers row + a magic link to log them in.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(subscribe))
}

#[derive(Debug, Deserialize)]
pub struct SubscribeInput {
    pub email: String,
    /// `"free"` or `"pro"`.
    pub plan: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscribeOut {
    Free { subscriber_id: Uuid },
    Pro { checkout_url: String },
}

async fn subscribe(
    State(s): State<AppState>,
    Json(input): Json<SubscribeInput>,
) -> AppResult<impl IntoResponse> {
    let email = input.email.trim().to_lowercase();
    if !email.contains('@') {
        return Err(AppError::Fields(vec![FieldError {
            field: "email".into(),
            message: "invalid email address".into(),
        }]));
    }

    // Upsert subscriber.
    let row = sqlx::query!(
        r#"
        INSERT INTO subscribers (email)
        VALUES ($1)
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id
        "#,
        email
    )
    .fetch_one(&s.pool)
    .await?;
    let subscriber_id = row.id;

    match input.plan.as_str() {
        "free" => Ok((StatusCode::OK, Json(SubscribeOut::Free { subscriber_id })).into_response()),
        "pro" => {
            let success = format!("{}/subscribed?ok=1", s.public_url);
            let cancel = format!("{}/subscribed?cancelled=1", s.public_url);
            let session = s
                .stripe
                .create_subscription_checkout(&email, &s.stripe_price_id, &success, &cancel)
                .await?;
            Ok((
                StatusCode::OK,
                Json(SubscribeOut::Pro {
                    checkout_url: session.url,
                }),
            )
                .into_response())
        }
        _ => Err(AppError::Fields(vec![FieldError {
            field: "plan".into(),
            message: "must be 'free' or 'pro'".into(),
        }])),
    }
}
