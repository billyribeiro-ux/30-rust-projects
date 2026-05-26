use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct Session {
    pub id: String,
    pub book_id: String,
    pub pages_read: i64,
    pub duration_minutes: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSession {
    pub pages_read: i64,
    pub duration_minutes: i64,
    pub occurred_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{session_id}", axum::routing::delete(delete))
}

async fn list(
    State(s): State<AppState>,
    Path(book_id): Path<String>,
) -> AppResult<Json<Vec<Session>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, book_id, pages_read, duration_minutes, occurred_at
        FROM reading_sessions
        WHERE book_id = ?1
        ORDER BY occurred_at DESC
        "#,
        book_id,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Session {
                id: r.id.expect("id is non-null primary key"),
                book_id: r.book_id,
                pages_read: r.pages_read,
                duration_minutes: r.duration_minutes,
                occurred_at: parse_ts(&r.occurred_at),
            })
            .collect(),
    ))
}

async fn create(
    State(s): State<AppState>,
    Path(book_id): Path<String>,
    Json(payload): Json<CreateSession>,
) -> AppResult<(StatusCode, Json<Session>)> {
    let mut errors = Vec::new();
    if payload.pages_read <= 0 {
        errors.push(FieldError {
            field: "pages_read".into(),
            message: "must be > 0".into(),
        });
    }
    if payload.duration_minutes <= 0 {
        errors.push(FieldError {
            field: "duration_minutes".into(),
            message: "must be > 0".into(),
        });
    }
    if !errors.is_empty() {
        return Err(AppError::Fields(errors));
    }

    let book_exists = sqlx::query!("SELECT id FROM books WHERE id = ?1", book_id)
        .fetch_optional(&s.pool)
        .await?;
    if book_exists.is_none() {
        return Err(AppError::NotFound);
    }

    let id = Uuid::new_v4().to_string();
    let occurred_at = payload.occurred_at.unwrap_or_else(Utc::now);
    let occurred_str = format_ts(occurred_at);

    sqlx::query!(
        r#"
        INSERT INTO reading_sessions (id, book_id, pages_read, duration_minutes, occurred_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        id,
        book_id,
        payload.pages_read,
        payload.duration_minutes,
        occurred_str,
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Session {
            id,
            book_id,
            pages_read: payload.pages_read,
            duration_minutes: payload.duration_minutes,
            occurred_at,
        }),
    ))
}

async fn delete(
    State(s): State<AppState>,
    Path((_book_id, session_id)): Path<(String, String)>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM reading_sessions WHERE id = ?1", session_id)
        .execute(&s.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
