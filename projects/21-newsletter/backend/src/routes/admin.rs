//! Author CRUD on posts. Sessions only.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, patch, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::routes::posts::render_markdown;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/posts", get(list).post(create))
        .route("/posts/{id}", patch(update))
        .route("/posts/{id}/publish", post(publish))
}

#[derive(Debug, Serialize)]
pub struct AdminPost {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub body_md: String,
    pub is_gated: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

async fn list(State(s): State<AppState>, _user: AuthUser) -> AppResult<Json<Vec<AdminPost>>> {
    let rows = sqlx::query!(
        r#"SELECT id, slug, title, summary, body_md, is_gated, published_at, updated_at
           FROM posts
           ORDER BY updated_at DESC
           LIMIT 200"#
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| AdminPost {
                id: r.id,
                slug: r.slug,
                title: r.title,
                summary: r.summary,
                body_md: r.body_md,
                is_gated: r.is_gated,
                published_at: r.published_at,
                updated_at: r.updated_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreatePostInput {
    pub slug: String,
    pub title: String,
    pub summary: Option<String>,
    pub body_md: String,
    pub is_gated: Option<bool>,
}

fn validate_slug(s: &str) -> AppResult<()> {
    let ok = !s.is_empty()
        && s.len() <= 120
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return Err(AppError::Fields(vec![FieldError {
            field: "slug".into(),
            message: "lowercase letters, digits, dashes (1-120 chars)".into(),
        }]));
    }
    Ok(())
}

async fn create(
    State(s): State<AppState>,
    _user: AuthUser,
    Json(input): Json<CreatePostInput>,
) -> AppResult<impl IntoResponse> {
    validate_slug(&input.slug)?;
    let body_html = render_markdown(&input.body_md);
    let id = Uuid::new_v4();
    let summary = input.summary.unwrap_or_default();
    let is_gated = input.is_gated.unwrap_or(false);
    sqlx::query!(
        r#"
        INSERT INTO posts (id, slug, title, summary, body_md, body_html, is_gated)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        id,
        input.slug,
        input.title,
        summary,
        input.body_md,
        body_html,
        is_gated,
    )
    .execute(&s.pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("posts_slug_key") => {
            AppError::Conflict("slug already in use".into())
        }
        _ => e.into(),
    })?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostInput {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub body_md: Option<String>,
    pub is_gated: Option<bool>,
}

async fn update(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdatePostInput>,
) -> AppResult<impl IntoResponse> {
    let existing = sqlx::query!("SELECT body_md FROM posts WHERE id = $1", id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let body_md = input.body_md.unwrap_or(existing.body_md);
    let body_html = render_markdown(&body_md);
    let summary = input.summary;
    sqlx::query!(
        r#"
        UPDATE posts
        SET title = COALESCE($2, title),
            summary = COALESCE($3, summary),
            body_md = $4,
            body_html = $5,
            is_gated = COALESCE($6, is_gated),
            updated_at = now()
        WHERE id = $1
        "#,
        id,
        input.title,
        summary,
        body_md,
        body_html,
        input.is_gated,
    )
    .execute(&s.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn publish(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let affected = sqlx::query!(
        r#"UPDATE posts SET published_at = COALESCE(published_at, now()), updated_at = now()
           WHERE id = $1"#,
        id
    )
    .execute(&s.pool)
    .await?
    .rows_affected();
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
