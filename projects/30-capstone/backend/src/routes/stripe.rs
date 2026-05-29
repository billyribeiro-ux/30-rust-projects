//! Stripe webhook stub — production wire-up reuses project 21's pattern.

use axum::Router;
use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;

pub fn router() -> Router<crate::state::AppState> {
    Router::new().route("/webhook", post(webhook))
}

async fn webhook(_h: HeaderMap, _b: Bytes) -> impl IntoResponse {
    // Production: copy `stripe/webhook.rs` from project 21. Verify signature,
    // INSERT INTO stripe_events ... ON CONFLICT DO NOTHING, dispatch.
    StatusCode::OK
}
