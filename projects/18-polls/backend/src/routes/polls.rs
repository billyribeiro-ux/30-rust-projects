//! Polls + live results.
//!
//! The headline feature of project 18 is the Postgres LISTEN/NOTIFY -> SSE
//! bridge. The flow:
//!
//!   1. A voter POSTs /api/polls/{slug}/vote. We INSERT into `votes`, then
//!      `SELECT pg_notify('poll_<slug>', <counts-json>)`. The server crashes,
//!      restarts, scales horizontally — Postgres is the single source of
//!      truth, so any process subscribed to that channel sees the event.
//!   2. Stage page opens an EventSource against /api/polls/{slug}/stream.
//!      The handler opens a *dedicated* PgListener (NOT borrowing from the
//!      pool — LISTEN is a per-connection state), LISTEN's on the channel,
//!      and turns notifications into `Sse<Stream<Event>>` via tokio-stream.
//!   3. Heartbeat every 15s keeps reverse proxies (nginx default 60s idle)
//!      from killing the connection.
//!
//! This is strictly better than the project-13 broadcast channel because
//! it works across multiple backend processes. Trade-offs: NOTIFY payloads
//! cap at 8KB, and you pay one connection per subscriber.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use futures::stream::{Stream, StreamExt};
use qrcode::QrCode;
use qrcode::render::svg;
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::postgres::PgListener;
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_poll).get(list_my_polls))
        .route("/{slug}", get(get_poll))
        .route("/{slug}/qr", get(qr_code))
        .route("/{slug}/vote", post(vote))
        .route("/{slug}/close", post(close_poll))
        .route("/{slug}/stream", get(stream))
}

// ---------- DTOs ----------

#[derive(Debug, Deserialize)]
pub struct CreatePollInput {
    pub slug: String,
    pub question: String,
    pub options: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OptionPublic {
    pub id: Uuid,
    pub label: String,
    pub position: i32,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct PollPublic {
    pub id: Uuid,
    pub slug: String,
    pub question: String,
    pub is_open: bool,
    pub options: Vec<OptionPublic>,
    pub total_votes: i64,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PollSummary {
    pub id: Uuid,
    pub slug: String,
    pub question: String,
    pub is_open: bool,
    pub option_count: i64,
    pub vote_count: i64,
    pub created_at: DateTime<Utc>,
}

// ---------- POST /api/polls ----------

async fn create_poll(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreatePollInput>,
) -> AppResult<(StatusCode, Json<PollPublic>)> {
    validate_slug(&input.slug)?;
    let question = input.question.trim();
    if question.is_empty() || question.len() > 500 {
        return Err(AppError::Fields(vec![FieldError {
            field: "question".into(),
            message: "must be 1-500 chars".into(),
        }]));
    }
    let options: Vec<String> = input
        .options
        .iter()
        .map(|o| o.trim().to_string())
        .filter(|o| !o.is_empty())
        .collect();
    if options.len() < 2 || options.len() > 16 {
        return Err(AppError::Fields(vec![FieldError {
            field: "options".into(),
            message: "between 2 and 16 options required".into(),
        }]));
    }
    for o in &options {
        if o.len() > 200 {
            return Err(AppError::Fields(vec![FieldError {
                field: "options".into(),
                message: "each option must be 1-200 chars".into(),
            }]));
        }
    }

    let mut tx = s.pool.begin().await?;

    let poll_row = sqlx::query!(
        r#"
        INSERT INTO polls (owner_id, slug, question)
        VALUES ($1, $2, $3)
        RETURNING id, slug, question, is_open, created_at, closed_at
        "#,
        user.id,
        input.slug,
        question,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(dbe) if dbe.constraint() == Some("polls_slug_key") => {
            AppError::Conflict("slug already in use".into())
        }
        sqlx::Error::Database(dbe) if dbe.constraint() == Some("polls_slug_check") => {
            AppError::Fields(vec![FieldError {
                field: "slug".into(),
                message: "must be 4-32 chars of [a-zA-Z0-9_-]".into(),
            }])
        }
        _ => AppError::from(e),
    })?;

    let mut option_rows: Vec<OptionPublic> = Vec::with_capacity(options.len());
    for (i, label) in options.iter().enumerate() {
        let row = sqlx::query!(
            r#"
            INSERT INTO options (poll_id, label, position)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            poll_row.id,
            label,
            i as i32,
        )
        .fetch_one(&mut *tx)
        .await?;
        option_rows.push(OptionPublic {
            id: row.id,
            label: label.clone(),
            position: i as i32,
            count: 0,
        });
    }

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(PollPublic {
            id: poll_row.id,
            slug: poll_row.slug,
            question: poll_row.question,
            is_open: poll_row.is_open,
            options: option_rows,
            total_votes: 0,
            created_at: poll_row.created_at,
            closed_at: poll_row.closed_at,
        }),
    ))
}

// ---------- GET /api/polls ----------

async fn list_my_polls(
    State(s): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<PollSummary>>> {
    let rows = sqlx::query!(
        r#"
        SELECT p.id, p.slug, p.question, p.is_open, p.created_at,
               (SELECT COUNT(*) FROM options o WHERE o.poll_id = p.id)            AS "option_count!",
               (SELECT COUNT(*) FROM votes v
                JOIN options o ON o.id = v.option_id
                WHERE o.poll_id = p.id)                                            AS "vote_count!"
        FROM polls p
        WHERE p.owner_id = $1
        ORDER BY p.created_at DESC
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| PollSummary {
                id: r.id,
                slug: r.slug,
                question: r.question,
                is_open: r.is_open,
                option_count: r.option_count,
                vote_count: r.vote_count,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

// ---------- GET /api/polls/{slug} ----------
// Public — anyone can read a poll (this is what the join page hits).

async fn get_poll(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<PollPublic>> {
    let p = load_poll_public(&s.pool, &slug).await?;
    Ok(Json(p))
}

async fn load_poll_public(pool: &PgPool, slug: &str) -> AppResult<PollPublic> {
    let poll = sqlx::query!(
        r#"
        SELECT id, slug, question, is_open, created_at, closed_at
        FROM polls WHERE slug = $1
        "#,
        slug,
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let options = sqlx::query!(
        r#"
        SELECT o.id, o.label, o.position,
               (SELECT COUNT(*) FROM votes v WHERE v.option_id = o.id) AS "count!"
        FROM options o
        WHERE o.poll_id = $1
        ORDER BY o.position ASC
        "#,
        poll.id,
    )
    .fetch_all(pool)
    .await?;

    let total: i64 = options.iter().map(|r| r.count).sum();
    let options = options
        .into_iter()
        .map(|r| OptionPublic {
            id: r.id,
            label: r.label,
            position: r.position,
            count: r.count,
        })
        .collect();

    Ok(PollPublic {
        id: poll.id,
        slug: poll.slug,
        question: poll.question,
        is_open: poll.is_open,
        options,
        total_votes: total,
        created_at: poll.created_at,
        closed_at: poll.closed_at,
    })
}

// ---------- GET /api/polls/{slug}/qr ----------

async fn qr_code(State(s): State<AppState>, Path(slug): Path<String>) -> AppResult<Response> {
    // 404 if no such poll (don't leak that a slug is valid via the QR endpoint).
    let exists = sqlx::query_scalar!("SELECT 1 AS x FROM polls WHERE slug = $1", slug)
        .fetch_optional(&s.pool)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    let join_url = format!("{}/p/{}", s.public_url, slug);
    let code =
        QrCode::new(join_url.as_bytes()).map_err(|e| AppError::Internal(format!("qrcode: {e}")))?;
    // SVG is good enough for our use (small, scales perfectly on projector).
    // The lesson note about PNG via the `image` crate is also valid but adds
    // a heavyweight dep we can avoid.
    let svg = code
        .render::<svg::Color<'_>>()
        .min_dimensions(256, 256)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();

    Response::builder()
        .header(header::CONTENT_TYPE, "image/svg+xml")
        .header(header::CACHE_CONTROL, "public, max-age=300")
        .body(Body::from(svg))
        .map_err(|e| AppError::Internal(e.to_string()))
}

// ---------- POST /api/polls/{slug}/vote ----------

#[derive(Debug, Deserialize)]
pub struct VoteInput {
    pub option_id: Uuid,
}

const VOTER_COOKIE: &str = "voter";

async fn vote(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(input): Json<VoteInput>,
) -> AppResult<impl IntoResponse> {
    let poll = sqlx::query!("SELECT id, is_open FROM polls WHERE slug = $1", slug,)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    if !poll.is_open {
        return Err(AppError::Conflict("poll is closed".into()));
    }

    // Verify the option belongs to this poll — else a vandal could submit
    // an option_id from a different poll and corrupt counts.
    let owns = sqlx::query_scalar!(
        "SELECT 1 AS x FROM options WHERE id = $1 AND poll_id = $2",
        input.option_id,
        poll.id,
    )
    .fetch_optional(&s.pool)
    .await?;
    if owns.is_none() {
        return Err(AppError::Validation(
            "option does not belong to poll".into(),
        ));
    }

    // Mint a voter cookie if there isn't one. The cookie is per-browser, so
    // a returning user gets the same key — that's intentional, the goal is
    // "one vote per browser-with-this-network-fingerprint per poll".
    let voter_cookie = jar.get(VOTER_COOKIE).map(|c| c.value().to_string());
    let voter_cookie_val = match voter_cookie {
        Some(v) => v,
        None => generate_voter_token(),
    };

    let ip = client_ip(&headers).unwrap_or_else(|| "unknown".to_string());
    let voter_key = derive_voter_key(&slug, &ip, &voter_cookie_val);

    // Single INSERT. The unique index (option_id, voter_key) catches a vote
    // *for the same option*. To catch "voted for option A, now trying B in
    // the same poll", we additionally check the whole poll up-front via the
    // EXISTS query — cheap, racy in theory but enforced by us treating any
    // PK violation as 409.
    let existing = sqlx::query_scalar!(
        r#"
        SELECT 1 AS x FROM votes v
        JOIN options o ON o.id = v.option_id
        WHERE o.poll_id = $1 AND v.voter_key = $2
        "#,
        poll.id,
        voter_key,
    )
    .fetch_optional(&s.pool)
    .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("already voted in this poll".into()));
    }

    let res = sqlx::query!(
        "INSERT INTO votes (option_id, voter_key) VALUES ($1, $2)",
        input.option_id,
        voter_key,
    )
    .execute(&s.pool)
    .await;

    if let Err(sqlx::Error::Database(dbe)) = &res
        && dbe.constraint() == Some("idx_votes_unique_per_poll")
    {
        return Err(AppError::Conflict("already voted in this poll".into()));
    }
    res?;

    // Notify all SSE subscribers. Payload is the new counts JSON.
    let counts = compute_counts_json(&s.pool, &slug).await?;
    let channel = notify_channel_for(&slug);
    // We use sqlx::query with format!() because pg_notify needs a literal
    // channel name OR we use the function form. The function form is safer.
    let payload_text = counts.to_string();
    sqlx::query("SELECT pg_notify($1, $2)")
        .bind(&channel)
        .bind(&payload_text)
        .execute(&s.pool)
        .await?;

    // Set the voter cookie (Lax, 30 days). On a fresh voter it ensures
    // they keep the same identity across page reloads / option pages.
    let cookie = Cookie::build((VOTER_COOKIE, voter_cookie_val))
        .http_only(true)
        .secure(s.secure_cookies)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(30))
        .build();
    let jar = jar.add(cookie);

    Ok((StatusCode::CREATED, jar, Json(counts)))
}

// ---------- POST /api/polls/{slug}/close ----------

async fn close_poll(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<StatusCode> {
    let res = sqlx::query!(
        r#"
        UPDATE polls SET is_open = FALSE, closed_at = now()
        WHERE slug = $1 AND owner_id = $2
        "#,
        slug,
        user.id,
    )
    .execute(&s.pool)
    .await?;

    if res.rows_affected() == 0 {
        // Either no such slug, or it's owned by someone else. We treat both
        // as 404 so we don't leak existence to non-owners.
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------- GET /api/polls/{slug}/stream  (SSE) ----------
//
// Public — no auth. The browser opens an EventSource which auto-reconnects.

async fn stream(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    // 404 fast if no such slug. We don't want to keep an SSE socket open
    // forever for non-existent polls.
    let exists = sqlx::query_scalar!("SELECT 1 AS x FROM polls WHERE slug = $1", slug)
        .fetch_optional(&s.pool)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    // Snapshot the current counts so subscribers don't have to wait for the
    // first vote to see anything.
    let initial = compute_counts_json(&s.pool, &slug).await?;

    let channel = notify_channel_for(&slug);
    let db_url = s.db_url.clone();

    // PgListener owns its own connection — LISTEN is per-connection state, so
    // we deliberately don't borrow from the pool (which would scatter LISTENs
    // across whichever connection happened to be checked out).
    let mut listener = PgListener::connect(&db_url)
        .await
        .map_err(|e| AppError::Internal(format!("PgListener::connect: {e}")))?;
    listener
        .listen(&channel)
        .await
        .map_err(|e| AppError::Internal(format!("LISTEN {channel}: {e}")))?;

    // Stream: emit the snapshot first, then turn notifications into Events.
    let init_stream = futures::stream::once(async move {
        Ok::<_, Infallible>(Event::default().event("counts").data(initial.to_string()))
    });

    let notif_stream = listener.into_stream().filter_map(|res| async move {
        match res {
            Ok(n) => Some(Ok::<_, Infallible>(
                Event::default().event("counts").data(n.payload()),
            )),
            Err(err) => {
                tracing::warn!(?err, "pg listener stream error");
                None
            }
        }
    });

    let combined = init_stream.chain(notif_stream);

    Ok(Sse::new(combined).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}

// ---------- helpers ----------

fn validate_slug(slug: &str) -> AppResult<()> {
    if slug.len() < 4 || slug.len() > 32 {
        return Err(AppError::Fields(vec![FieldError {
            field: "slug".into(),
            message: "must be 4-32 chars".into(),
        }]));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::Fields(vec![FieldError {
            field: "slug".into(),
            message: "only letters, digits, -, _".into(),
        }]));
    }
    Ok(())
}

fn notify_channel_for(slug: &str) -> String {
    // Channel names must be valid Postgres identifiers (no quoting). Our slug
    // regex already restricts to [a-zA-Z0-9_-] but '-' isn't legal in an
    // unquoted identifier — so we just feed the channel via pg_notify (which
    // takes a TEXT, not an identifier) and skip the quoting issue entirely.
    format!("poll_{slug}")
}

fn derive_voter_key(slug: &str, ip: &str, cookie: &str) -> String {
    let mut h = Sha256::new();
    h.update(slug.as_bytes());
    h.update(b"|");
    h.update(ip.as_bytes());
    h.update(b"|");
    h.update(cookie.as_bytes());
    URL_SAFE_NO_PAD.encode(h.finalize())
}

fn generate_voter_token() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

async fn compute_counts_json(pool: &PgPool, slug: &str) -> AppResult<serde_json::Value> {
    let rows = sqlx::query!(
        r#"
        SELECT o.id, o.label, o.position,
               (SELECT COUNT(*) FROM votes v WHERE v.option_id = o.id) AS "count!"
        FROM options o
        JOIN polls p ON p.id = o.poll_id
        WHERE p.slug = $1
        ORDER BY o.position ASC
        "#,
        slug,
    )
    .fetch_all(pool)
    .await?;
    let total: i64 = rows.iter().map(|r| r.count).sum();
    let arr = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "option_id": r.id,
                "label": r.label,
                "position": r.position,
                "count": r.count,
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "slug": slug,
        "total": total,
        "counts": arr,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_rules() {
        assert!(validate_slug("abcd").is_ok());
        assert!(validate_slug("a_b-1234").is_ok());
        assert!(validate_slug("ab").is_err());
        assert!(validate_slug("ab cd").is_err());
        assert!(validate_slug(&"x".repeat(33)).is_err());
    }

    #[test]
    fn voter_key_differs_per_poll() {
        let a = derive_voter_key("poll-a", "1.2.3.4", "cookie-xyz");
        let b = derive_voter_key("poll-b", "1.2.3.4", "cookie-xyz");
        assert_ne!(a, b);
    }

    #[test]
    fn voter_key_stable_for_same_inputs() {
        let a = derive_voter_key("poll-a", "1.2.3.4", "cookie-xyz");
        let b = derive_voter_key("poll-a", "1.2.3.4", "cookie-xyz");
        assert_eq!(a, b);
    }

    #[test]
    fn notify_channel_includes_slug() {
        assert_eq!(notify_channel_for("hello"), "poll_hello");
    }
}
