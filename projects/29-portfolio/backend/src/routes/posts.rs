use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/{slug}", get(detail))
}

#[derive(Debug, Serialize)]
pub struct PostSummary {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub hero_image_url: Option<String>,
    pub locale: String,
    pub published_at: Option<DateTime<Utc>>,
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<PostSummary>>> {
    let rows = sqlx::query!(
        r#"SELECT id, slug, title, summary, hero_image_url, locale, published_at
           FROM posts WHERE status = 'published'
           ORDER BY published_at DESC LIMIT 100"#
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| PostSummary {
                id: r.id,
                slug: r.slug,
                title: r.title,
                summary: r.summary,
                hero_image_url: r.hero_image_url,
                locale: r.locale,
                published_at: r.published_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct PostFull {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub hero_image_url: Option<String>,
    pub body_html: String,
    pub locale: String,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

async fn detail(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    let r = sqlx::query!(
        r#"SELECT id, slug, title, summary, hero_image_url, body_html, locale,
                  published_at, updated_at
           FROM posts WHERE slug = $1 AND status = 'published'"#,
        slug
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok((
        StatusCode::OK,
        Json(PostFull {
            id: r.id,
            slug: r.slug,
            title: r.title,
            summary: r.summary,
            hero_image_url: r.hero_image_url,
            body_html: r.body_html,
            locale: r.locale,
            published_at: r.published_at,
            updated_at: r.updated_at,
        }),
    ))
}
