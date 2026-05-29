use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(detail))
        .route("/{id}/state", post(update_state))
}

#[derive(Debug, Serialize)]
pub struct InterviewOut {
    pub id: Uuid,
    pub interviewer_id: Uuid,
    pub candidate_email: String,
    pub status: String,
    pub code: String,
    pub language: String,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<InterviewOut>>> {
    let rows = sqlx::query!(
        r#"SELECT id, interviewer_id, candidate_email, status, code, language,
                  started_at, ended_at, created_at
           FROM interviews
           WHERE interviewer_id = $1
           ORDER BY created_at DESC LIMIT 100"#,
        user.id
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| InterviewOut {
                id: r.id,
                interviewer_id: r.interviewer_id,
                candidate_email: r.candidate_email,
                status: r.status,
                code: r.code,
                language: r.language,
                started_at: r.started_at,
                ended_at: r.ended_at,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateInput {
    pub candidate_email: String,
    pub language: Option<String>,
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(i): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    let email = i.candidate_email.trim().to_lowercase();
    if !email.contains('@') {
        return Err(AppError::Fields(vec![FieldError {
            field: "candidate_email".into(),
            message: "invalid email".into(),
        }]));
    }
    let id = Uuid::new_v4();
    let language = i.language.unwrap_or_else(|| "rust".into());
    sqlx::query!(
        r#"INSERT INTO interviews (id, interviewer_id, candidate_email, language)
           VALUES ($1, $2, $3, $4)"#,
        id,
        user.id,
        email,
        language
    )
    .execute(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

async fn detail(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<InterviewOut>> {
    let r = sqlx::query!(
        r#"SELECT id, interviewer_id, candidate_email, status, code, language,
                  started_at, ended_at, created_at
           FROM interviews WHERE id = $1 AND interviewer_id = $2"#,
        id,
        user.id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(InterviewOut {
        id: r.id,
        interviewer_id: r.interviewer_id,
        candidate_email: r.candidate_email,
        status: r.status,
        code: r.code,
        language: r.language,
        started_at: r.started_at,
        ended_at: r.ended_at,
        created_at: r.created_at,
    }))
}

#[derive(Debug, Deserialize)]
pub struct StateInput {
    pub status: String,
}

async fn update_state(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(i): Json<StateInput>,
) -> AppResult<impl IntoResponse> {
    if !matches!(i.status.as_str(), "live" | "ended") {
        return Err(AppError::Validation("status must be live|ended".into()));
    }
    let n = sqlx::query!(
        r#"UPDATE interviews
           SET status = $3,
               started_at = CASE WHEN $3 = 'live' AND started_at IS NULL THEN now() ELSE started_at END,
               ended_at   = CASE WHEN $3 = 'ended' THEN now() ELSE ended_at END,
               updated_at = now()
           WHERE id = $1 AND interviewer_id = $2"#,
        id,
        user.id,
        i.status
    )
    .execute(&s.pool)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
