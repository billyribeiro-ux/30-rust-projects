use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::markdown::{render, slugify};

#[derive(Debug, Serialize)]
pub struct NoteSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Note {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub body_md: String,
    pub body_html: String,
    pub excerpt: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNote {
    pub title: String,
    pub body_md: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNote {
    pub title: Option<String>,
    pub body_md: Option<String>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{slug}", get(read).patch(update).delete(delete))
}

async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<NoteSummary>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, slug, title, excerpt, updated_at
        FROM notes
        ORDER BY updated_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let notes = rows
        .into_iter()
        .map(|r| NoteSummary {
            id: r.id.expect("id is non-null primary key"),
            slug: r.slug,
            title: r.title,
            excerpt: r.excerpt,
            updated_at: parse_ts(&r.updated_at),
        })
        .collect();

    Ok(Json(notes))
}

async fn read(State(pool): State<SqlitePool>, Path(slug): Path<String>) -> AppResult<Json<Note>> {
    let row = sqlx::query!(
        r#"
        SELECT id, slug, title, body_md, body_html, excerpt, created_at, updated_at
        FROM notes
        WHERE slug = ?1
        "#,
        slug,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(Note {
        id: row.id.expect("id is non-null primary key"),
        slug: row.slug,
        title: row.title,
        body_md: row.body_md,
        body_html: row.body_html,
        excerpt: row.excerpt,
        created_at: parse_ts(&row.created_at),
        updated_at: parse_ts(&row.updated_at),
    }))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateNote>,
) -> AppResult<(StatusCode, Json<Note>)> {
    let title = normalize_title(&payload.title)?;
    let body_md = normalize_body(&payload.body_md)?;
    let rendered = render(&body_md);

    let id = Uuid::new_v4().to_string();
    let slug = unique_slug(&pool, &slugify(&title)).await?;
    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        r#"
        INSERT INTO notes (id, slug, title, body_md, body_html, excerpt, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
        "#,
        id,
        slug,
        title,
        body_md,
        rendered.html,
        rendered.excerpt,
        now_str,
    )
    .execute(&pool)
    .await?;

    let note = Note {
        id,
        slug,
        title,
        body_md,
        body_html: rendered.html,
        excerpt: rendered.excerpt,
        created_at: now,
        updated_at: now,
    };

    Ok((StatusCode::CREATED, Json(note)))
}

async fn update(
    State(pool): State<SqlitePool>,
    Path(slug): Path<String>,
    Json(payload): Json<UpdateNote>,
) -> AppResult<Json<Note>> {
    let existing = sqlx::query!(
        r#"
        SELECT id, slug, title, body_md, body_html, excerpt, created_at, updated_at
        FROM notes
        WHERE slug = ?1
        "#,
        slug,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let new_title = match payload.title.as_deref() {
        Some(t) => normalize_title(t)?,
        None => existing.title,
    };
    let new_body_md = match payload.body_md.as_deref() {
        Some(b) => normalize_body(b)?,
        None => existing.body_md,
    };
    let rendered = render(&new_body_md);

    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        r#"
        UPDATE notes
        SET title       = ?1,
            body_md     = ?2,
            body_html   = ?3,
            excerpt     = ?4,
            updated_at  = ?5
        WHERE slug = ?6
        "#,
        new_title,
        new_body_md,
        rendered.html,
        rendered.excerpt,
        now_str,
        slug,
    )
    .execute(&pool)
    .await?;

    Ok(Json(Note {
        id: existing.id.expect("id is non-null primary key"),
        slug,
        title: new_title,
        body_md: new_body_md,
        body_html: rendered.html,
        excerpt: rendered.excerpt,
        created_at: parse_ts(&existing.created_at),
        updated_at: now,
    }))
}

async fn delete(State(pool): State<SqlitePool>, Path(slug): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM notes WHERE slug = ?1", slug)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn unique_slug(pool: &SqlitePool, base: &str) -> AppResult<String> {
    for n in 0u32..1000 {
        let candidate = if n == 0 {
            base.to_string()
        } else {
            format!("{base}-{n}")
        };
        let row = sqlx::query!("SELECT id FROM notes WHERE slug = ?1", candidate)
            .fetch_optional(pool)
            .await?;
        if row.is_none() {
            return Ok(candidate);
        }
    }
    Err(AppError::Conflict("could not allocate unique slug".into()))
}

fn normalize_title(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("title must not be empty".into()));
    }
    if trimmed.chars().count() > 200 {
        return Err(AppError::Validation(
            "title must be 200 chars or fewer".into(),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_body(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim_end();
    if trimmed.chars().count() > 100_000 {
        return Err(AppError::Validation(
            "body must be 100,000 chars or fewer".into(),
        ));
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
        assert!(matches!(
            normalize_title("   "),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_title_trims() {
        assert_eq!(
            normalize_title("  Morning Notes  ").unwrap(),
            "Morning Notes"
        );
    }

    #[test]
    fn normalize_title_rejects_too_long() {
        let long = "a".repeat(201);
        assert!(matches!(
            normalize_title(&long),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_body_rejects_too_long() {
        let long = "a".repeat(100_001);
        assert!(matches!(
            normalize_body(&long),
            Err(AppError::Validation(_))
        ));
    }
}
