//! Public + entitlement-gated post reads.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::session::{self, SUBSCRIBER_COOKIE_NAME};
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
    pub is_gated: bool,
    pub published_at: Option<DateTime<Utc>>,
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<PostSummary>>> {
    let rows = sqlx::query!(
        r#"SELECT id, slug, title, summary, is_gated, published_at
           FROM posts
           WHERE published_at IS NOT NULL
           ORDER BY published_at DESC
           LIMIT 100"#
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
                is_gated: r.is_gated,
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
    pub body_html: String,
    pub is_gated: bool,
    pub paywalled: bool,
    pub published_at: Option<DateTime<Utc>>,
}

async fn detail(
    State(s): State<AppState>,
    jar: CookieJar,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query!(
        r#"SELECT id, slug, title, summary, body_html, is_gated, published_at
           FROM posts
           WHERE slug = $1 AND published_at IS NOT NULL"#,
        slug
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    // Optionally extract subscriber from cookie. Pure read-through —
    // an unauthenticated visitor sees the teaser; an authenticated
    // free-tier subscriber also sees the teaser; only `plan='pro'`
    // unlocks gated bodies.
    let is_pro = if let Some(raw) = jar
        .get(SUBSCRIBER_COOKIE_NAME)
        .map(|c| c.value().to_string())
    {
        let token_hash = session::hash_token(&raw);
        sqlx::query_scalar!(
            r#"
            SELECT s.plan = 'pro'
            FROM subscriber_sessions ss
            JOIN subscriptions s ON s.subscriber_id = ss.subscriber_id
            WHERE ss.token_hash = $1 AND ss.expires_at > now()
            "#,
            token_hash
        )
        .fetch_optional(&s.pool)
        .await?
        .flatten()
        .unwrap_or(false)
    } else {
        false
    };

    let paywalled = row.is_gated && !is_pro;
    let body_html = if paywalled {
        // Truncate to a teaser. 600 chars matches the SEO-friendly "lead
        // paragraph" length crawlers expect to see.
        let teaser = row.body_html.chars().take(600).collect::<String>();
        format!("{teaser}<p><em>… subscribe to read the rest.</em></p>")
    } else {
        row.body_html
    };

    let status = if paywalled {
        StatusCode::PAYMENT_REQUIRED
    } else {
        StatusCode::OK
    };
    let post = PostFull {
        id: row.id,
        slug: row.slug,
        title: row.title,
        summary: row.summary,
        body_html,
        is_gated: row.is_gated,
        paywalled,
        published_at: row.published_at,
    };
    Ok((status, Json(post)))
}

pub fn render_markdown(md: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, opts);
    let mut raw = String::new();
    html::push_html(&mut raw, parser);
    ammonia::clean(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_strips_dangerous_html() {
        let html = render_markdown("# Hello\n<script>alert(1)</script>\n");
        assert!(html.contains("<h1>"));
        assert!(!html.contains("<script>"));
    }
}
