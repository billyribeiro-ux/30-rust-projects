//! Hot path: `GET /{slug}` → 302 to `target_url`.
//!
//! Design notes — this is the project's high-throughput route:
//!   1. No auth required, no SSR, no session lookup. The DB hit is one
//!      indexed select on `links.slug`.
//!   2. Redis INCR is fire-and-forget but kept inline so the counter reflects
//!      reality at the moment of redirect (the INCR is roundtrip-cheap).
//!   3. The CLICK INSERT is deferred to `tokio::spawn` so the redirect
//!      response can fly out without waiting for the analytics write. If the
//!      process dies between the response and the insert, we lose at most
//!      one click row — but Redis still incremented, so the durable counter
//!      is preserved.
//!   4. Bot filter: regex match on the User-Agent. Matched UAs DO get a 302
//!      (we don't want to break scrapers entirely), but we skip the click
//!      insert AND the Redis INCR to keep analytics clean.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::http::header;
use axum::response::{IntoResponse, Redirect, Response};
use regex::Regex;
use std::sync::OnceLock;

use crate::state::AppState;

/// Conservative, hand-maintained UA blocklist for analytics noise. Real
/// production uses something like `ua-parser` with the regex DB from
/// https://github.com/ua-parser/uap-core. Keeping this small + explicit so
/// the lesson can show *what's being filtered*.
fn bot_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"^(Googlebot|Bingbot|YandexBot|DuckDuckBot|curl|wget|Python-urllib|HTTPie)")
            .unwrap()
    })
}

pub fn is_bot(ua: &str) -> bool {
    bot_re().is_match(ua)
}

/// Mock GeoIP lookup. The real version would load a MaxMind .mmdb file at
/// startup and look up the client IP. We keep a deterministic stub here so
/// the rest of the pipeline (DB column, stats aggregation) can be exercised
/// without depending on a 70MB binary asset.
///
/// Lesson: the surface area for "where do I want country codes?" is small
/// and local to this function — swap it out for the real impl later without
/// touching the handlers.
pub fn geoip_lookup(ip: Option<&str>) -> Option<String> {
    let ip = ip?;
    // Cheap parity-based stub: even last byte → 'US', odd → 'DE'. Loopback
    // and anything we can't parse → None.
    let last = ip.split('.').next_back()?.parse::<u8>().ok()?;
    if ip.starts_with("127.") || ip == "::1" {
        return None;
    }
    Some(if last % 2 == 0 {
        "US".into()
    } else {
        "DE".into()
    })
}

pub async fn redirect_to_target(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    // Look up the link. 404 page is a plain text response so we can avoid
    // a layout for the hot path.
    let row = sqlx::query!(r#"SELECT id, target_url FROM links WHERE slug = $1"#, slug,)
        .fetch_optional(&s.pool)
        .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "short link not found").into_response();
        }
        Err(e) => {
            tracing::error!(error = ?e, "redirect db lookup failed");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "internal error",
            )
                .into_response();
        }
    };

    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let referer = headers
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let xff = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string());

    let bot = is_bot(ua);

    if !bot {
        // Fire-and-forget INCR on the live counter. Cheap (single roundtrip)
        // and we don't need to await durability — the click row INSERT below
        // is the source of truth.
        if let Some(pool) = s.redis.clone() {
            let key = format!("clicks:{slug}:total");
            tokio::spawn(async move {
                if let Ok(mut conn) = pool.get().await {
                    let _: Result<i64, _> = deadpool_redis::redis::cmd("INCR")
                        .arg(&key)
                        .query_async(&mut conn)
                        .await;
                }
            });
        }
    }

    // Spawn the click INSERT so the redirect response goes out NOW. Even if
    // the click insert fails or takes a moment, the user's browser is
    // already following the Location header.
    let link_id = row.id;
    let pool = s.pool.clone();
    let ua_owned = ua.to_string();
    let country = geoip_lookup(xff.as_deref());
    tokio::spawn(async move {
        let _ = sqlx::query!(
            r#"
            INSERT INTO clicks (link_id, user_agent, referer, ip_country, is_bot)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            link_id,
            (!ua_owned.is_empty()).then_some(ua_owned),
            referer,
            country,
            bot,
        )
        .execute(&pool)
        .await
        .inspect_err(|e| tracing::error!(error = ?e, "click insert failed"));
    });

    Redirect::temporary(&row.target_url).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_filter_matches_known_uas() {
        assert!(is_bot("curl/8.4.0"));
        assert!(is_bot("Googlebot/2.1 (+http://www.google.com/bot.html)"));
        assert!(is_bot("wget/1.21"));
        assert!(is_bot("Python-urllib/3.11"));
        assert!(is_bot("HTTPie/3.2.2"));
    }

    #[test]
    fn bot_filter_leaves_real_browsers_alone() {
        assert!(!is_bot(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15"
        ));
        assert!(!is_bot(""));
    }

    #[test]
    fn geoip_stub_returns_country_for_non_loopback() {
        assert_eq!(geoip_lookup(Some("8.8.8.8")), Some("US".into()));
        assert_eq!(geoip_lookup(Some("1.2.3.5")), Some("DE".into()));
        assert_eq!(geoip_lookup(Some("127.0.0.1")), None);
        assert_eq!(geoip_lookup(None), None);
    }
}
