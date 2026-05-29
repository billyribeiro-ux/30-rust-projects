//! Admin dashboard endpoints.

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::jobs::queue;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/queues", get(list_queues))
        .route("/jobs", get(list_jobs).post(enqueue_job))
        .route("/jobs/{id}", get(get_job))
        .route("/jobs/{id}/retry", post(retry_job))
}

#[derive(Debug, Serialize)]
pub struct QueueSummary {
    pub queue: String,
    pub pending: i64,
    pub running: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub dead: i64,
}

async fn list_queues(
    State(s): State<AppState>,
    _user: AuthUser,
) -> AppResult<Json<Vec<QueueSummary>>> {
    // Single query that pivots status counts per queue.
    let rows = sqlx::query!(
        r#"
        SELECT
            queue                                            AS "queue!",
            SUM((status = 'pending')::int)::bigint           AS "pending!",
            SUM((status = 'running')::int)::bigint           AS "running!",
            SUM((status = 'succeeded')::int)::bigint         AS "succeeded!",
            SUM((status = 'failed')::int)::bigint            AS "failed!",
            SUM((status = 'dead')::int)::bigint              AS "dead!"
        FROM jobs
        GROUP BY queue
        ORDER BY queue
        "#
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| QueueSummary {
                queue: r.queue,
                pending: r.pending,
                running: r.running,
                succeeded: r.succeeded,
                failed: r.failed,
                dead: r.dead,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub queue: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct JobOut {
    pub id: Uuid,
    pub queue: String,
    pub kind: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub status: String,
    pub run_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

async fn list_jobs(
    State(s): State<AppState>,
    _user: AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<JobOut>>> {
    let queue = q.queue.unwrap_or_else(|| "default".into());
    let status = q.status.unwrap_or_else(|| "pending".into());
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let rows = sqlx::query!(
        r#"
        SELECT id, queue, kind, attempts, max_attempts, status, run_at, last_error,
               created_at, updated_at
        FROM jobs
        WHERE queue = $1 AND status = $2
        ORDER BY created_at DESC
        LIMIT $3
        "#,
        queue,
        status,
        limit
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| JobOut {
                id: r.id,
                queue: r.queue,
                kind: r.kind,
                attempts: r.attempts,
                max_attempts: r.max_attempts,
                status: r.status,
                run_at: r.run_at,
                last_error: r.last_error,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect(),
    ))
}

async fn get_job(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<JobOut>> {
    let r = sqlx::query!(
        r#"
        SELECT id, queue, kind, attempts, max_attempts, status, run_at, last_error,
               created_at, updated_at
        FROM jobs WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(JobOut {
        id: r.id,
        queue: r.queue,
        kind: r.kind,
        attempts: r.attempts,
        max_attempts: r.max_attempts,
        status: r.status,
        run_at: r.run_at,
        last_error: r.last_error,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }))
}

async fn retry_job(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let affected = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'pending',
            run_at = now(),
            locked_until = NULL,
            locked_by = NULL,
            updated_at = now()
        WHERE id = $1 AND status IN ('failed', 'dead')
        "#,
        id
    )
    .execute(&s.pool)
    .await?
    .rows_affected();
    if affected == 0 {
        return Err(AppError::Conflict("job is not in a retryable state".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct EnqueueInput {
    pub queue: Option<String>,
    pub kind: String,
    pub payload: Option<serde_json::Value>,
    pub max_attempts: Option<i32>,
}

async fn enqueue_job(
    State(s): State<AppState>,
    _user: AuthUser,
    Json(input): Json<EnqueueInput>,
) -> AppResult<impl IntoResponse> {
    let queue = input.queue.unwrap_or_else(|| "default".into());
    let payload = input.payload.unwrap_or_else(|| serde_json::json!({}));
    let max_attempts = input.max_attempts.unwrap_or(5);
    let id = queue::enqueue(
        &s.pool,
        &queue,
        &input.kind,
        payload,
        max_attempts,
        Utc::now(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}
