use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTodo {
    pub title: Option<String>,
    pub done: Option<bool>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", patch(update).delete(delete))
}

async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<Todo>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, title, done, created_at, updated_at
        FROM todos
        ORDER BY done ASC, created_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let todos = rows
        .into_iter()
        .map(|r| Todo {
            id: r.id.expect("id is non-null primary key"),
            title: r.title,
            done: r.done != 0,
            created_at: parse_ts(&r.created_at),
            updated_at: parse_ts(&r.updated_at),
        })
        .collect();

    Ok(Json(todos))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateTodo>,
) -> AppResult<(StatusCode, Json<Todo>)> {
    let title = normalize_title(&payload.title)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        r#"
        INSERT INTO todos (id, title, done, created_at, updated_at)
        VALUES (?1, ?2, 0, ?3, ?3)
        "#,
        id,
        title,
        now_str,
    )
    .execute(&pool)
    .await?;

    let todo = Todo {
        id,
        title,
        done: false,
        created_at: now,
        updated_at: now,
    };

    Ok((StatusCode::CREATED, Json(todo)))
}

async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTodo>,
) -> AppResult<Json<Todo>> {
    let now = Utc::now();
    let now_str = format_ts(now);

    let title = payload.title.as_deref().map(normalize_title).transpose()?;
    let done = payload.done.map(|d| if d { 1 } else { 0 });

    let row = sqlx::query!(
        r#"
        UPDATE todos
        SET title       = COALESCE(?1, title),
            done        = COALESCE(?2, done),
            updated_at  = ?3
        WHERE id = ?4
        RETURNING id, title, done, created_at, updated_at
        "#,
        title,
        done,
        now_str,
        id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(Todo {
        id: row.id.expect("id is non-null primary key"),
        title: row.title,
        done: row.done != 0,
        created_at: parse_ts(&row.created_at),
        updated_at: parse_ts(&row.updated_at),
    }))
}

async fn delete(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM todos WHERE id = ?1", id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn normalize_title(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("title must not be empty".into()));
    }
    if trimmed.chars().count() > 200 {
        return Err(AppError::Validation("title must be 200 chars or fewer".into()));
    }
    Ok(trimmed.to_string())
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
    fn normalize_title_rejects_empty() {
        assert!(matches!(normalize_title("   "), Err(AppError::Validation(_))));
    }

    #[test]
    fn normalize_title_trims() {
        assert_eq!(normalize_title("  buy milk  ").unwrap(), "buy milk");
    }

    #[test]
    fn normalize_title_rejects_too_long() {
        let long = "a".repeat(201);
        assert!(matches!(normalize_title(&long), Err(AppError::Validation(_))));
    }
}
