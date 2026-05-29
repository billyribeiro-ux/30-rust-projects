//! `POST /v1/inference` — the developer-facing endpoint. Authenticates
//! via api-key, runs a mocked inference, writes a `usage_events` row.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::routes::keys;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(infer))
}

#[derive(Debug, Deserialize)]
pub struct InferenceInput {
    pub prompt: String,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct InferenceOut {
    pub completion: String,
    pub tokens_used: i32,
    pub units_billed: i32,
}

async fn infer(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(i): Json<InferenceInput>,
) -> AppResult<impl IntoResponse> {
    let auth = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let (api_key_id, _user_id, _rpm) = keys::authenticate(&s.pool, auth).await?;

    let max = i.max_tokens.unwrap_or(64).clamp(1, 1024);
    let completion = mock_complete(&i.prompt, max);
    let tokens_used = completion.split_whitespace().count() as i32;

    sqlx::query!(
        "INSERT INTO usage_events (api_key_id, units) VALUES ($1, 1)",
        api_key_id
    )
    .execute(&s.pool)
    .await?;

    sqlx::query!(
        "UPDATE api_keys SET last_used_at = now() WHERE id = $1",
        api_key_id
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::OK,
        Json(InferenceOut {
            completion,
            tokens_used,
            units_billed: 1,
        }),
    ))
}

fn mock_complete(prompt: &str, max_tokens: i32) -> String {
    // The mock just echoes the prompt prefix and appends Lorem. Not exciting
    // — the point is to test billing/auth/rate-limit, not the LLM.
    let head = prompt.chars().take(40).collect::<String>();
    let tail = "Lorem ipsum dolor sit amet, consectetur adipiscing elit.";
    let n = (max_tokens / 4).max(1) as usize;
    format!("{head} … {}", tail.repeat(n.min(20)))
}
