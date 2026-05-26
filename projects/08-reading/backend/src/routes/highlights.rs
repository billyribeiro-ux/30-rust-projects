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
pub struct Highlight {
    pub id: String,
    pub book_id: String,
    pub quote: String,
    pub note: String,
    pub page: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateHighlight {
    pub quote: String,
    pub note: Option<String>,
    pub page: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{highlight_id}", axum::routing::delete(delete))
}

async fn list(
    State(s): State<AppState>,
    Path(book_id): Path<String>,
) -> AppResult<Json<Vec<Highlight>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, book_id, quote, note, page, created_at
        FROM highlights WHERE book_id = ?1
        ORDER BY created_at DESC
        "#,
        book_id,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Highlight {
                id: r.id.expect("id is non-null primary key"),
                book_id: r.book_id,
                quote: r.quote,
                note: r.note,
                page: r.page,
                created_at: parse_ts(&r.created_at),
            })
            .collect(),
    ))
}

async fn create(
    State(s): State<AppState>,
    Path(book_id): Path<String>,
    Json(payload): Json<CreateHighlight>,
) -> AppResult<(StatusCode, Json<Highlight>)> {
    let quote = payload.quote.trim().to_string();
    if quote.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "quote".into(),
            message: "must not be empty".into(),
        }]));
    }
    if quote.chars().count() > 2000 {
        return Err(AppError::Fields(vec![FieldError {
            field: "quote".into(),
            message: "must be 2000 chars or fewer".into(),
        }]));
    }
    let note = payload.note.as_deref().unwrap_or("").trim().to_string();

    let book_exists = sqlx::query!("SELECT id FROM books WHERE id = ?1", book_id)
        .fetch_optional(&s.pool)
        .await?;
    if book_exists.is_none() {
        return Err(AppError::NotFound);
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        "INSERT INTO highlights (id, book_id, quote, note, page, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        id,
        book_id,
        quote,
        note,
        payload.page,
        now_str,
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Highlight {
            id,
            book_id,
            quote,
            note,
            page: payload.page,
            created_at: now,
        }),
    ))
}

async fn delete(
    State(s): State<AppState>,
    Path((_book_id, highlight_id)): Path<(String, String)>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM highlights WHERE id = ?1", highlight_id)
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
