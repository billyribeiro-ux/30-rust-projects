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
        .route("/articles", get(list).post(create))
        .route("/articles/{id}", patch(update))
        .route("/articles/{id}/publish", post(publish))
}

pub fn render_markdown(md: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(md, opts);
    let mut raw = String::new();
    html::push_html(&mut raw, parser);
    ammonia::clean(&raw)
}

#[derive(Debug, Serialize)]
pub struct AdminArticle {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

async fn list(State(s): State<AppState>, _u: AuthUser) -> AppResult<Json<Vec<AdminArticle>>> {
    let rows = sqlx::query!(
        "SELECT id, slug, title, published_at, updated_at FROM articles ORDER BY updated_at DESC LIMIT 200"
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| AdminArticle {
                id: r.id,
                slug: r.slug,
                title: r.title,
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
    pub body_md: String,
    pub category_slug: Option<String>,
    pub locale: Option<String>,
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
        || i.slug.len() > 120
    {
        return Err(AppError::Fields(vec![FieldError {
            field: "slug".into(),
            message: "lowercase letters, digits, dashes (1-120 chars)".into(),
        }]));
    }
    let body_html = render_markdown(&i.body_md);
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO articles (id, slug, title, summary, body_md, body_html, category_slug, locale)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"#,
        id,
        i.slug,
        i.title,
        i.summary.unwrap_or_default(),
        i.body_md,
        body_html,
        i.category_slug,
        i.locale.unwrap_or_else(|| "en".into())
    )
    .execute(&s.pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("articles_slug_key") => {
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
    pub body_md: Option<String>,
}

async fn update(
    State(s): State<AppState>,
    _u: AuthUser,
    Path(id): Path<Uuid>,
    Json(i): Json<UpdateInput>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query!("SELECT body_md FROM articles WHERE id = $1", id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let body_md = i.body_md.unwrap_or(row.body_md);
    let body_html = render_markdown(&body_md);
    sqlx::query!(
        r#"UPDATE articles SET
             title     = COALESCE($2, title),
             summary   = COALESCE($3, summary),
             body_md   = $4,
             body_html = $5,
             updated_at = now()
           WHERE id = $1"#,
        id,
        i.title,
        i.summary,
        body_md,
        body_html
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
        "UPDATE articles SET published_at = COALESCE(published_at, now()), updated_at = now() WHERE id = $1",
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
