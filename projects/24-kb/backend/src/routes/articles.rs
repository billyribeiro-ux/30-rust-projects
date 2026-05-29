//! Public read of articles (and categories for breadcrumbs).

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
pub struct ArticleSummary {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub category_slug: Option<String>,
    pub locale: String,
    pub published_at: Option<DateTime<Utc>>,
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<ArticleSummary>>> {
    let rows = sqlx::query!(
        r#"SELECT id, slug, title, summary, category_slug, locale, published_at
           FROM articles
           WHERE published_at IS NOT NULL
           ORDER BY published_at DESC
           LIMIT 100"#
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ArticleSummary {
                id: r.id,
                slug: r.slug,
                title: r.title,
                summary: r.summary,
                category_slug: r.category_slug,
                locale: r.locale,
                published_at: r.published_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct Faq {
    pub q: String,
    pub a: String,
}

#[derive(Debug, Serialize)]
pub struct ArticleFull {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub body_html: String,
    pub category_slug: Option<String>,
    pub locale: String,
    pub faqs: Vec<Faq>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

async fn detail(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query!(
        r#"SELECT id, slug, title, summary, body_html, category_slug, locale,
                  published_at, updated_at
           FROM articles WHERE slug = $1 AND published_at IS NOT NULL"#,
        slug
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let faqs = sqlx::query!(
        "SELECT q, a FROM faqs WHERE article_id = $1 ORDER BY position",
        row.id
    )
    .fetch_all(&s.pool)
    .await?
    .into_iter()
    .map(|r| Faq { q: r.q, a: r.a })
    .collect();
    Ok((
        StatusCode::OK,
        Json(ArticleFull {
            id: row.id,
            slug: row.slug,
            title: row.title,
            summary: row.summary,
            body_html: row.body_html,
            category_slug: row.category_slug,
            locale: row.locale,
            faqs,
            published_at: row.published_at,
            updated_at: row.updated_at,
        }),
    ))
}
