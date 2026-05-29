//! `POST /v1/ingest` — open public endpoint. Events validate, insert,
//! return 204. Production would rate-limit (project 17 pattern).

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(ingest))
}

#[derive(Debug, Deserialize)]
pub struct IngestInput {
    pub kind: String,
    pub payload: Option<serde_json::Value>,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
}

async fn ingest(
    State(s): State<AppState>,
    Json(i): Json<IngestInput>,
) -> AppResult<impl IntoResponse> {
    if i.kind.is_empty() || i.kind.len() > 80 {
        return Err(AppError::Fields(vec![FieldError {
            field: "kind".into(),
            message: "1..=80 chars".into(),
        }]));
    }
    let payload = i.payload.unwrap_or_else(|| serde_json::json!({}));
    sqlx::query!(
        r#"INSERT INTO events (kind, payload, user_id, session_id) VALUES ($1, $2, $3, $4)"#,
        i.kind,
        payload,
        i.user_id,
        i.session_id
    )
    .execute(&s.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
