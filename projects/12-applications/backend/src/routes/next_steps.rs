//! Next-step todos tied to applications. The background reminder task
//! (`reminders::tick`) emails users about steps due in the next 24h.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct NextStep {
    pub id: Uuid,
    pub application_id: Uuid,
    pub body: String,
    pub due_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub reminded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInput {
    pub body: String,
    pub due_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_for_application).post(create))
        .route("/{step_id}/complete", post(complete))
        .route("/{step_id}", axum::routing::delete(delete))
}

async fn list_for_application(
    State(s): State<AppState>,
    user: AuthUser,
    Path(application_id): Path<Uuid>,
) -> AppResult<Json<Vec<NextStep>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, application_id, body, due_at, completed_at, reminded_at, created_at
        FROM next_steps
        WHERE application_id = $1 AND user_id = $2
        ORDER BY due_at ASC
        "#,
        application_id,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| NextStep {
                id: r.id,
                application_id: r.application_id,
                body: r.body,
                due_at: r.due_at,
                completed_at: r.completed_at,
                reminded_at: r.reminded_at,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Path(application_id): Path<Uuid>,
    Json(input): Json<CreateInput>,
) -> AppResult<(StatusCode, Json<NextStep>)> {
    let body = input.body.trim().to_string();
    if body.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "body".into(),
            message: "required".into(),
        }]));
    }
    if body.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "body".into(),
            message: "must be 200 chars or fewer".into(),
        }]));
    }

    let app_exists = sqlx::query!(
        "SELECT id FROM applications WHERE id = $1 AND user_id = $2",
        application_id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?;
    if app_exists.is_none() {
        return Err(AppError::NotFound);
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query!(
        r#"
        INSERT INTO next_steps (id, application_id, user_id, body, due_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        id,
        application_id,
        user.id,
        body,
        input.due_at,
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(NextStep {
            id,
            application_id,
            body,
            due_at: input.due_at,
            completed_at: None,
            reminded_at: None,
            created_at: now,
        }),
    ))
}

async fn complete(
    State(s): State<AppState>,
    user: AuthUser,
    Path((_application_id, step_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!(
        "UPDATE next_steps SET completed_at = now() WHERE id = $1 AND user_id = $2 AND completed_at IS NULL",
        step_id, user.id,
    )
    .execute(&s.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete(
    State(s): State<AppState>,
    user: AuthUser,
    Path((_application_id, step_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!(
        "DELETE FROM next_steps WHERE id = $1 AND user_id = $2",
        step_id,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
