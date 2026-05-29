//! Single-user CMS — list + create + update + publish.

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
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/posts", get(list).post(create))
        .route("/posts/{id}", patch(update))
        .route("/posts/{id}/publish", post(publish))
}

pub fn render_mdx(body: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(body, opts);
    let mut raw = String::new();
    html::push_html(&mut raw, parser);
    ammonia::clean(&raw)
}

#[derive(Debug, Serialize)]
pub struct AdminPost {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

async fn list(State(s): State<AppState>, _u: AuthUser) -> AppResult<Json<Vec<AdminPost>>> {
    let rows = sqlx::query!(
        "SELECT id, slug, title, status, published_at, updated_at FROM posts ORDER BY updated_at DESC LIMIT 200"
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| AdminPost {
                id: r.id,
                slug: r.slug,
                title: r.title,
                status: r.status,
                published_at: r.published_at,
                updated_at: r.updated_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateInput {
    pub slug: String,
    pub title: String,
    pub summary: Option<String>,
    pub hero_image_url: Option<String>,
    pub body_mdx: String,
}

async fn create(
    State(s): State<AppState>,
    _u: AuthUser,
    Json(i): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    if !i
        .slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || i.slug.is_empty()
    {
        return Err(AppError::Fields(vec![FieldError {
            field: "slug".into(),
            message: "lowercase-dash slug only".into(),
        }]));
    }
    let html = render_mdx(&i.body_mdx);
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO posts (id, slug, title, summary, hero_image_url, body_mdx, body_html)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        id,
        i.slug,
        i.title,
        i.summary.unwrap_or_default(),
        i.hero_image_url,
        i.body_mdx,
        html
    )
    .execute(&s.pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("posts_slug_key") => {
            AppError::Conflict("slug in use".into())
        }
        _ => e.into(),
    })?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

#[derive(Debug, Deserialize)]
pub struct UpdateInput {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub hero_image_url: Option<String>,
    pub body_mdx: Option<String>,
}

async fn update(
    State(s): State<AppState>,
    _u: AuthUser,
    Path(id): Path<Uuid>,
    Json(i): Json<UpdateInput>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query!("SELECT body_mdx FROM posts WHERE id = $1", id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let body = i.body_mdx.unwrap_or(row.body_mdx);
    let html = render_mdx(&body);
    sqlx::query!(
        r#"UPDATE posts
           SET title = COALESCE($2, title),
               summary = COALESCE($3, summary),
               hero_image_url = COALESCE($4, hero_image_url),
               body_mdx = $5,
               body_html = $6,
               updated_at = now()
           WHERE id = $1"#,
        id,
        i.title,
        i.summary,
        i.hero_image_url,
        body,
        html
    )
    .execute(&s.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn publish(
    State(s): State<AppState>,
    _u: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let n = sqlx::query!(
        "UPDATE posts SET status = 'published',
                          published_at = COALESCE(published_at, now()),
                          updated_at = now()
         WHERE id = $1",
        id
    )
    .execute(&s.pool)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
