//! `GET /feed.xml` — RSS 2.0 of published posts. Plain text body (the
//! teaser when gated, full when not — RSS readers are public, no auth
//! context).

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;

use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/feed.xml", get(rss))
}

async fn rss(State(s): State<AppState>) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query!(
        r#"SELECT slug, title, summary, body_html, is_gated, published_at
           FROM posts
           WHERE published_at IS NOT NULL
           ORDER BY published_at DESC
           LIMIT 50"#
    )
    .fetch_all(&s.pool)
    .await?;

    let mut xml = String::with_capacity(4096);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(r#"<rss version="2.0"><channel>"#);
    xml.push_str(&format!(
        "<title>Newsletter</title><link>{}</link><description>Latest posts</description>",
        escape(&s.public_url)
    ));
    for r in rows {
        let body = if r.is_gated {
            format!(
                "{}<p><em>Subscribe to read the full post.</em></p>",
                r.summary
            )
        } else {
            r.body_html
        };
        xml.push_str("<item>");
        xml.push_str(&format!("<title>{}</title>", escape(&r.title)));
        xml.push_str(&format!(
            "<link>{}/p/{}</link>",
            escape(&s.public_url),
            escape(&r.slug)
        ));
        xml.push_str(&format!(
            "<guid isPermaLink=\"false\">{}</guid>",
            escape(&r.slug)
        ));
        if let Some(ts) = r.published_at {
            xml.push_str(&format!(
                "<pubDate>{}</pubDate>",
                ts.format("%a, %d %b %Y %H:%M:%S GMT")
            ));
        }
        xml.push_str(&format!("<description><![CDATA[{body}]]></description>"));
        xml.push_str("</item>");
    }
    xml.push_str("</channel></rss>");

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/rss+xml; charset=utf-8".parse().unwrap(),
    );
    Ok((StatusCode::OK, headers, xml))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
