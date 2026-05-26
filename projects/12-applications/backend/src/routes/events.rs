//! Application timeline events. Read-only list + create-note endpoint.
//! Status-change events are auto-emitted by `applications::update`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct Event {
    pub id: Uuid,
    pub kind: String,
    pub body: String,
    pub new_status: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteInput {
    pub body: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list).post(create_note))
}

async fn list(
    State(s): State<AppState>,
    user: AuthUser,
    Path(application_id): Path<Uuid>,
) -> AppResult<Json<Vec<Event>>> {
    // Confirm the application is the user's; 404 either way.
    let exists = sqlx::query!(
        "SELECT id FROM applications WHERE id = $1 AND user_id = $2",
        application_id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    let rows = sqlx::query!(
        r#"
        SELECT id, kind, body, new_status, occurred_at
        FROM application_events
        WHERE application_id = $1
        ORDER BY occurred_at DESC
        "#,
        application_id,
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| Event {
                id: r.id,
                kind: r.kind,
                body: r.body,
                new_status: r.new_status,
                occurred_at: r.occurred_at,
            })
            .collect(),
    ))
}

async fn create_note(
    State(s): State<AppState>,
    user: AuthUser,
    Path(application_id): Path<Uuid>,
    Json(input): Json<CreateNoteInput>,
) -> AppResult<(StatusCode, Json<Event>)> {
    let body = input.body.trim().to_string();
    if body.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "body".into(),
            message: "required".into(),
        }]));
    }
    if body.chars().count() > 2000 {
        return Err(AppError::Fields(vec![FieldError {
            field: "body".into(),
            message: "must be 2000 chars or fewer".into(),
        }]));
    }

    let exists = sqlx::query!(
        "SELECT id FROM applications WHERE id = $1 AND user_id = $2",
        application_id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query!(
        r#"
        INSERT INTO application_events (id, application_id, user_id, kind, body, occurred_at)
        VALUES ($1, $2, $3, 'note', $4, $5)
        "#,
        id,
        application_id,
        user.id,
        body,
        now,
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Event {
            id,
            kind: "note".into(),
            body,
            new_status: None,
            occurred_at: now,
        }),
    ))
}
