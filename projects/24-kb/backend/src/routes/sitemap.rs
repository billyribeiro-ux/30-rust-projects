//! sitemap.xml + robots.txt for SEO.

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;

use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sitemap.xml", get(sitemap))
        .route("/robots.txt", get(robots))
}

async fn sitemap(State(s): State<AppState>) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query!(
        "SELECT slug, updated_at FROM articles WHERE published_at IS NOT NULL ORDER BY updated_at DESC"
    )
    .fetch_all(&s.pool)
    .await?;
    let mut xml = String::with_capacity(2048);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);
    xml.push_str(&format!(
        "<url><loc>{}</loc></url>",
        xml_escape(&s.public_url)
    ));
    for r in rows {
        xml.push_str(&format!(
            "<url><loc>{}/a/{}</loc><lastmod>{}</lastmod></url>",
            xml_escape(&s.public_url),
            xml_escape(&r.slug),
            r.updated_at.to_rfc3339()
        ));
    }
    xml.push_str("</urlset>");
    let mut h = HeaderMap::new();
    h.insert(
        header::CONTENT_TYPE,
        "application/xml; charset=utf-8".parse().unwrap(),
    );
    Ok((StatusCode::OK, h, xml))
}

async fn robots(State(s): State<AppState>) -> impl IntoResponse {
    let body = format!(
        "User-agent: *\nAllow: /\nSitemap: {}/sitemap.xml\n",
        s.public_url
    );
    let mut h = HeaderMap::new();
    h.insert(
        header::CONTENT_TYPE,
        "text/plain; charset=utf-8".parse().unwrap(),
    );
    (StatusCode::OK, h, body)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
