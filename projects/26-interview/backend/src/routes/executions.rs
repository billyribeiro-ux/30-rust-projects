//! Code execution. This project ships a **safe mock** runner: it
//! reflects stdin and reports a zero exit code. The real implementation
//! is one of:
//!   * a `docker run --rm --network=none …` subprocess (the curriculum's spec)
//!   * a gVisor / firecracker sandbox
//!   * a cloud sandbox service (Judge0, Hephaestus, etc.)
//!
//! The contract is the same in all three. The trait keeps the route
//! testable without docker in CI.

use async_trait::async_trait;
use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/{interview_id}/execute", post(execute))
}

#[derive(Debug, Deserialize)]
pub struct ExecuteInput {
    pub language: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct ExecutionOut {
    pub id: Uuid,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: i32,
}

#[async_trait]
pub trait CodeRunner: Send + Sync {
    async fn run(&self, language: &str, code: &str) -> AppResult<(String, String, i32, i32)>;
}

pub struct MockRunner;

#[async_trait]
impl CodeRunner for MockRunner {
    async fn run(&self, language: &str, code: &str) -> AppResult<(String, String, i32, i32)> {
        // Pretend we executed. Reflect the first 1KB of the code.
        let stdout = format!("[mock-{language}] {} chars run\n", code.chars().count());
        Ok((stdout, String::new(), 0, 7))
    }
}

async fn execute(
    State(s): State<AppState>,
    user: AuthUser,
    Path(interview_id): Path<Uuid>,
    Json(input): Json<ExecuteInput>,
) -> AppResult<impl IntoResponse> {
    sqlx::query!(
        "SELECT id FROM interviews WHERE id = $1 AND interviewer_id = $2",
        interview_id,
        user.id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let runner = MockRunner;
    let (stdout, stderr, exit_code, duration_ms) = runner.run(&input.language, &input.code).await?;

    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO executions (id, interview_id, language, code, stdout, stderr, exit_code, duration_ms)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        id,
        interview_id,
        input.language,
        input.code,
        stdout,
        stderr,
        exit_code,
        duration_ms
    )
    .execute(&s.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(ExecutionOut {
            id,
            stdout,
            stderr,
            exit_code,
            duration_ms,
        }),
    ))
}
