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
pub struct Book {
    pub id: String,
    pub isbn: Option<String>,
    pub title: String,
    pub author: String,
    pub cover_url: Option<String>,
    pub pages: Option<i64>,
    pub status: String,
    pub current_page: i64,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBook {
    pub isbn: Option<String>,
    pub title: String,
    pub author: Option<String>,
    pub cover_url: Option<String>,
    pub pages: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBook {
    pub status: Option<String>,
    pub current_page: Option<i64>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub pages: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(read).patch(update).delete(delete))
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<Book>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, isbn, title, author, cover_url, pages, status, current_page,
               started_at, finished_at, created_at, updated_at
        FROM books
        ORDER BY updated_at DESC
        "#
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Book {
                id: r.id.expect("id is non-null primary key"),
                isbn: r.isbn,
                title: r.title,
                author: r.author,
                cover_url: r.cover_url,
                pages: r.pages,
                status: r.status,
                current_page: r.current_page,
                started_at: r.started_at.as_deref().map(parse_ts),
                finished_at: r.finished_at.as_deref().map(parse_ts),
                created_at: parse_ts(&r.created_at),
                updated_at: parse_ts(&r.updated_at),
            })
            .collect(),
    ))
}

async fn read(State(s): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Book>> {
    let r = sqlx::query!(
        r#"
        SELECT id, isbn, title, author, cover_url, pages, status, current_page,
               started_at, finished_at, created_at, updated_at
        FROM books WHERE id = ?1
        "#,
        id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(Book {
        id: r.id.expect("id is non-null primary key"),
        isbn: r.isbn,
        title: r.title,
        author: r.author,
        cover_url: r.cover_url,
        pages: r.pages,
        status: r.status,
        current_page: r.current_page,
        started_at: r.started_at.as_deref().map(parse_ts),
        finished_at: r.finished_at.as_deref().map(parse_ts),
        created_at: parse_ts(&r.created_at),
        updated_at: parse_ts(&r.updated_at),
    }))
}

async fn create(
    State(s): State<AppState>,
    Json(payload): Json<CreateBook>,
) -> AppResult<(StatusCode, Json<Book>)> {
    let title = normalize_title(&payload.title)?;
    let author = payload
        .author
        .as_deref()
        .map(|a| a.trim().to_string())
        .unwrap_or_default();
    let status = normalize_status(payload.status.as_deref().unwrap_or("want_to_read"))?;
    let pages = payload.pages.filter(|p| *p > 0);
    let isbn = payload.isbn.as_deref().map(|s| s.trim().to_string());

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        r#"
        INSERT INTO books (id, isbn, title, author, cover_url, pages, status,
                           current_page, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8)
        "#,
        id,
        isbn,
        title,
        author,
        payload.cover_url,
        pages,
        status,
        now_str,
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Book {
            id,
            isbn,
            title,
            author,
            cover_url: payload.cover_url,
            pages,
            status,
            current_page: 0,
            started_at: None,
            finished_at: None,
            created_at: now,
            updated_at: now,
        }),
    ))
}

async fn update(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateBook>,
) -> AppResult<Json<Book>> {
    let row = sqlx::query!(
        r#"
        SELECT id, isbn, title, author, cover_url, pages, status, current_page,
               started_at, finished_at, created_at, updated_at
        FROM books WHERE id = ?1
        "#,
        id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let prev_status = row.status.clone();
    let new_status = if let Some(s) = payload.status.as_deref() {
        normalize_status(s)?
    } else {
        prev_status.clone()
    };
    let new_title = match payload.title.as_deref() {
        Some(t) => normalize_title(t)?,
        None => row.title.clone(),
    };
    let new_author = payload
        .author
        .map(|a| a.trim().to_string())
        .unwrap_or(row.author.clone());
    let new_pages = payload.pages.filter(|p| *p > 0).or(row.pages);
    let new_current_page = if let Some(cp) = payload.current_page {
        if cp < 0 {
            return Err(AppError::Fields(vec![FieldError {
                field: "current_page".into(),
                message: "must be >= 0".into(),
            }]));
        }
        cp
    } else {
        row.current_page
    };

    let now = Utc::now();
    let now_str = format_ts(now);
    // Auto-fill status timestamps on transitions:
    // - first time entering 'reading' → set started_at
    // - first time entering 'finished' → set finished_at
    // - leaving 'finished' clears finished_at (so flipping back-and-forth doesn't lie)
    let started_at = if prev_status != "reading" && new_status == "reading" {
        Some(now_str.clone())
    } else {
        row.started_at.clone()
    };
    let finished_at = if prev_status != "finished" && new_status == "finished" {
        Some(now_str.clone())
    } else if new_status != "finished" {
        None
    } else {
        row.finished_at.clone()
    };

    sqlx::query!(
        r#"
        UPDATE books
        SET status = ?1, current_page = ?2, title = ?3, author = ?4, pages = ?5,
            started_at = ?6, finished_at = ?7, updated_at = ?8
        WHERE id = ?9
        "#,
        new_status,
        new_current_page,
        new_title,
        new_author,
        new_pages,
        started_at,
        finished_at,
        now_str,
        id,
    )
    .execute(&s.pool)
    .await?;

    let id_str = row.id.expect("id is non-null primary key");
    Ok(Json(Book {
        id: id_str,
        isbn: row.isbn,
        title: new_title,
        author: new_author,
        cover_url: row.cover_url,
        pages: new_pages,
        status: new_status,
        current_page: new_current_page,
        started_at: started_at.as_deref().map(parse_ts),
        finished_at: finished_at.as_deref().map(parse_ts),
        created_at: parse_ts(&row.created_at),
        updated_at: now,
    }))
}

async fn delete(State(s): State<AppState>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM books WHERE id = ?1", id)
        .execute(&s.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn normalize_title(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "title".into(),
            message: "must not be empty".into(),
        }]));
    }
    if trimmed.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "title".into(),
            message: "must be 200 chars or fewer".into(),
        }]));
    }
    Ok(trimmed.to_string())
}

fn normalize_status(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().to_lowercase();
    match trimmed.as_str() {
        "want_to_read" | "reading" | "finished" => Ok(trimmed),
        _ => Err(AppError::Fields(vec![FieldError {
            field: "status".into(),
            message: "must be want_to_read, reading, or finished".into(),
        }])),
    }
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_must_not_be_empty() {
        assert!(matches!(normalize_title(""), Err(AppError::Fields(_))));
    }

    #[test]
    fn status_whitelist() {
        assert_eq!(normalize_status("reading").unwrap(), "reading");
        assert!(matches!(
            normalize_status("dreaming"),
            Err(AppError::Fields(_))
        ));
    }
}
