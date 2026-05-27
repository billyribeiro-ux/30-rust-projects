//! Link CRUD + stats endpoints.
//!
//! Owner scoping: every read/write is filtered by `WHERE user_id = $session_user`.
//! Looking up *someone else's* slug returns 404, never 403 — that's the lesson
//! point: stats are private; we don't even leak the existence of foreign slugs.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use rand::RngCore;
use rand::rngs::OsRng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create).get(list))
        .route("/{slug}/stats", get(stats))
}

fn slug_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[a-zA-Z0-9_-]{3,32}$").unwrap())
}

fn validate_url(u: &str) -> AppResult<()> {
    let trimmed = u.trim();
    if trimmed.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "target_url".into(),
            message: "required".into(),
        }]));
    }
    if trimmed.len() > 2048 {
        return Err(AppError::Fields(vec![FieldError {
            field: "target_url".into(),
            message: "too long (max 2048)".into(),
        }]));
    }
    // Cheap scheme check; not a full URL parse, but rejects the obvious cases
    // (`javascript:`, no scheme at all). Production: use a real URL parser.
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(AppError::Fields(vec![FieldError {
            field: "target_url".into(),
            message: "must start with http:// or https://".into(),
        }]));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateInput {
    pub target_url: String,
    /// Optional custom slug. If absent, a random base64url 7-char slug is
    /// generated. We retry up to 5 times on collision before giving up.
    pub slug: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LinkRow {
    pub id: Uuid,
    pub slug: String,
    pub target_url: String,
    pub created_at: DateTime<Utc>,
    /// Cached counter from Redis, if available, otherwise None. The /stats
    /// endpoint is the source-of-truth aggregate; this is a UX hint for the
    /// list view.
    pub click_count: Option<i64>,
}

fn random_slug() -> String {
    let mut bytes = [0u8; 6]; // 6 bytes → 8 chars b64url, trim to 7
    OsRng.fill_bytes(&mut bytes);
    let s = URL_SAFE_NO_PAD.encode(bytes);
    s.chars().take(7).collect()
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    validate_url(&input.target_url)?;
    let target_url = input.target_url.trim().to_string();

    let chosen_slug = if let Some(raw) = input.slug.as_ref() {
        let s = raw.trim().to_string();
        if !slug_re().is_match(&s) {
            return Err(AppError::Fields(vec![FieldError {
                field: "slug".into(),
                message: "3–32 chars: letters, digits, '-' or '_'".into(),
            }]));
        }
        Some(s)
    } else {
        None
    };

    // 5 attempts; with 6 bytes of entropy the collision rate is astronomical
    // for any realistic dataset, but the retry guards against unlucky ones.
    for attempt in 0..5u8 {
        let slug = chosen_slug.clone().unwrap_or_else(random_slug);
        let id = Uuid::new_v4();
        let res = sqlx::query!(
            r#"
            INSERT INTO links (id, user_id, slug, target_url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, slug, target_url, created_at
            "#,
            id,
            user.id,
            slug,
            target_url,
        )
        .fetch_one(&s.pool)
        .await;

        match res {
            Ok(r) => {
                return Ok((
                    StatusCode::CREATED,
                    Json(LinkRow {
                        id: r.id,
                        slug: r.slug,
                        target_url: r.target_url,
                        created_at: r.created_at,
                        click_count: Some(0),
                    }),
                ));
            }
            Err(sqlx::Error::Database(dbe)) if dbe.constraint() == Some("links_slug_key") => {
                if chosen_slug.is_some() {
                    // User-picked slug collided — that's a 409. No retry: they
                    // need to pick a different one.
                    return Err(AppError::Conflict("slug already taken".into()));
                }
                tracing::warn!(attempt, "random slug collision; retrying");
                continue;
            }
            Err(e) => return Err(AppError::from(e)),
        }
    }
    Err(AppError::Internal(
        "failed to allocate a unique slug after 5 tries".into(),
    ))
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<LinkRow>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, slug, target_url, created_at
        FROM links
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 500
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    // Pull the live counters from Redis if available, fall back to DB count.
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let count = read_counter(&s, &r.slug).await;
        out.push(LinkRow {
            id: r.id,
            slug: r.slug,
            target_url: r.target_url,
            created_at: r.created_at,
            click_count: count,
        });
    }
    Ok(Json(out))
}

/// Best-effort read of the Redis counter. None if Redis isn't configured or
/// if the read failed (treat as cache miss — UI will show "—" or fall back
/// to the DB count for that link).
async fn read_counter(s: &AppState, slug: &str) -> Option<i64> {
    if let Some(pool) = &s.redis {
        let mut conn = pool.get().await.ok()?;
        let key = format!("clicks:{slug}:total");
        let v: Result<Option<i64>, _> = deadpool_redis::redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await;
        v.ok().flatten()
    } else {
        None
    }
}

// ---------- stats ----------

#[derive(Debug, Serialize)]
pub struct Stats {
    pub link: LinkRow,
    pub total_clicks: i64,
    pub clicks_today: i64,
    pub clicks_7d: i64,
    pub clicks_30d: i64,
    pub redis_counter: Option<i64>,
    pub top_referers: Vec<NamedCount>,
    pub top_countries: Vec<NamedCount>,
    /// Last 7 daily click counts, oldest first, for the bar chart.
    pub daily: Vec<DailyBucket>,
}

#[derive(Debug, Serialize)]
pub struct NamedCount {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyBucket {
    pub day: String, // YYYY-MM-DD UTC
    pub count: i64,
}

async fn stats(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Stats>> {
    // Scoping: looking up someone else's slug = NotFound (we don't even
    // confirm the slug exists for a different owner — 404, never 403).
    let link = sqlx::query!(
        r#"
        SELECT id, slug, target_url, created_at
        FROM links
        WHERE slug = $1 AND user_id = $2
        "#,
        slug,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let link_id = link.id;

    let total = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "n!" FROM clicks WHERE link_id = $1 AND is_bot = FALSE"#,
        link_id,
    )
    .fetch_one(&s.pool)
    .await?;

    let today = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "n!"
        FROM clicks
        WHERE link_id = $1 AND is_bot = FALSE
          AND occurred_at >= date_trunc('day', now())
        "#,
        link_id,
    )
    .fetch_one(&s.pool)
    .await?;

    let week = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "n!"
        FROM clicks
        WHERE link_id = $1 AND is_bot = FALSE
          AND occurred_at >= now() - interval '7 days'
        "#,
        link_id,
    )
    .fetch_one(&s.pool)
    .await?;

    let month = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "n!"
        FROM clicks
        WHERE link_id = $1 AND is_bot = FALSE
          AND occurred_at >= now() - interval '30 days'
        "#,
        link_id,
    )
    .fetch_one(&s.pool)
    .await?;

    let top_ref_rows = sqlx::query!(
        r#"
        SELECT COALESCE(NULLIF(referer, ''), '(direct)') AS "name!", COUNT(*) AS "count!"
        FROM clicks
        WHERE link_id = $1 AND is_bot = FALSE
        GROUP BY 1
        ORDER BY 2 DESC, 1 ASC
        LIMIT 10
        "#,
        link_id,
    )
    .fetch_all(&s.pool)
    .await?;

    let top_country_rows = sqlx::query!(
        r#"
        SELECT COALESCE(ip_country, '??') AS "name!", COUNT(*) AS "count!"
        FROM clicks
        WHERE link_id = $1 AND is_bot = FALSE
        GROUP BY 1
        ORDER BY 2 DESC, 1 ASC
        LIMIT 10
        "#,
        link_id,
    )
    .fetch_all(&s.pool)
    .await?;

    let daily = sqlx::query!(
        r#"
        SELECT to_char(date_trunc('day', occurred_at), 'YYYY-MM-DD') AS "day!",
               COUNT(*) AS "count!"
        FROM clicks
        WHERE link_id = $1 AND is_bot = FALSE
          AND occurred_at >= now() - interval '7 days'
        GROUP BY 1
        ORDER BY 1 ASC
        "#,
        link_id,
    )
    .fetch_all(&s.pool)
    .await?;

    let redis_counter = read_counter(&s, &link.slug).await;

    Ok(Json(Stats {
        link: LinkRow {
            id: link.id,
            slug: link.slug,
            target_url: link.target_url,
            created_at: link.created_at,
            click_count: redis_counter,
        },
        total_clicks: total,
        clicks_today: today,
        clicks_7d: week,
        clicks_30d: month,
        redis_counter,
        top_referers: top_ref_rows
            .into_iter()
            .map(|r| NamedCount {
                name: r.name,
                count: r.count,
            })
            .collect(),
        top_countries: top_country_rows
            .into_iter()
            .map(|r| NamedCount {
                name: r.name,
                count: r.count,
            })
            .collect(),
        daily: daily
            .into_iter()
            .map(|r| DailyBucket {
                day: r.day,
                count: r.count,
            })
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_regex_matches_3_to_32_alnum_dash_underscore() {
        let r = slug_re();
        assert!(r.is_match("abc"));
        assert!(r.is_match("My-Slug_42"));
        assert!(r.is_match(&"x".repeat(32)));
        assert!(!r.is_match("ab"));
        assert!(!r.is_match(&"x".repeat(33)));
        assert!(!r.is_match("has space"));
        assert!(!r.is_match("has!"));
    }

    #[test]
    fn validate_url_accepts_https_and_http() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://example.com").is_ok());
    }

    #[test]
    fn validate_url_rejects_other_schemes() {
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("").is_err());
        assert!(validate_url(&"x".repeat(3000)).is_err());
    }

    #[test]
    fn random_slug_is_short_and_valid() {
        for _ in 0..50 {
            let s = random_slug();
            assert!(slug_re().is_match(&s), "{s} not valid");
        }
    }
}
