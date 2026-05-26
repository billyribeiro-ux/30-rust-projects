//! Server-side sessions.
//!
//! Wire format:
//! - 32 random bytes, base64url-encoded (43 chars, no padding) → "raw token"
//! - SHA-256(raw_token) is what we store and look up by ("token_hash" in DB)
//! - The user gets the raw token in an HttpOnly cookie; the DB never sees it
//!   in plaintext.
//!
//! Why hash even the raw token? Because the cookie value travels with the
//! user. A DB leak alone shouldn't let an attacker make ad-hoc Set-Cookie
//! headers — they'd need the live cookie. Hashing is cheap (one SHA-256)
//! and pays for itself the first time someone's DB ends up on a forum.
//!
//! The cookie is HttpOnly + Secure + SameSite=Lax + 30-day Max-Age.
//! - HttpOnly: no JS access (no document.cookie reads)
//! - Secure: HTTPS only (browsers ignore on http://localhost as a convenience)
//! - SameSite=Lax: CSRF protection for state-changing requests
//! - 30-day sliding: we extend `expires_at` on each request

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Duration, Utc};
use rand::RngCore;
use rand::rngs::OsRng;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const SESSION_COOKIE_NAME: &str = "app_session";
pub const SESSION_TTL_DAYS: i64 = 30;

/// What we attach to authenticated requests via the AuthUser extractor.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub email_verified: bool,
    pub session_id: Uuid,
}

/// Generate a fresh opaque session token. 32 bytes of OS randomness →
/// base64url (43 chars). Match `Cookie` accepts. The HASH is stored;
/// the RAW value goes in the cookie.
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// SHA-256 of the raw token. We use sha2 indirectly via argon2's transitive
/// dep, but to avoid coupling we'll use a tiny inline impl from `subtle` —
/// actually, sha2 is widely available; we just import it inline here.
pub fn hash_token(raw: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    h.finalize().to_vec()
}

/// Issue a new session for the given user and write it to the DB. Returns
/// the raw token (for the cookie) and the session id (for logging).
pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> AppResult<(String, Uuid)> {
    let raw = generate_token();
    let token_hash = hash_token(&raw);
    let id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);

    sqlx::query!(
        r#"
        INSERT INTO sessions (id, user_id, token_hash, expires_at, ip, user_agent)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        id,
        user_id,
        token_hash,
        expires_at,
        ip,
        user_agent,
    )
    .execute(pool)
    .await?;

    Ok((raw, id))
}

/// Look up a session by its raw cookie token. Updates `last_seen_at` and
/// slides the expiry forward by SESSION_TTL_DAYS. Returns None if the token
/// is missing, expired, or doesn't match anything.
pub async fn lookup(pool: &PgPool, raw_token: &str) -> AppResult<Option<AuthUser>> {
    let token_hash = hash_token(raw_token);
    let row = sqlx::query!(
        r#"
        SELECT s.id AS session_id, s.user_id, s.expires_at,
               u.email, u.name, u.email_verified_at
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = $1
          AND s.expires_at > now()
        "#,
        token_hash,
    )
    .fetch_optional(pool)
    .await?;

    let Some(r) = row else { return Ok(None) };

    // Sliding expiry: bump last_seen + expires forward.
    let new_expires = Utc::now() + Duration::days(SESSION_TTL_DAYS);
    sqlx::query!(
        "UPDATE sessions SET last_seen_at = now(), expires_at = $1 WHERE id = $2",
        new_expires,
        r.session_id,
    )
    .execute(pool)
    .await?;

    Ok(Some(AuthUser {
        id: r.user_id,
        email: r.email,
        name: r.name,
        email_verified: r.email_verified_at.is_some(),
        session_id: r.session_id,
    }))
}

/// Revoke (delete) a session by id. Used on logout.
pub async fn destroy(pool: &PgPool, session_id: Uuid) -> AppResult<()> {
    sqlx::query!("DELETE FROM sessions WHERE id = $1", session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Build the Set-Cookie header for a fresh session.
pub fn session_cookie(raw_token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, raw_token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(SESSION_TTL_DAYS))
        .build()
}

/// Build the Set-Cookie header that clears the session cookie (Max-Age=0).
pub fn clear_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, ""))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build()
}

/// Helper for handlers that want the *optional* user (e.g., a route that
/// renders differently for signed-in vs signed-out users).
#[allow(dead_code)] // Public surface; used by routes added in later projects.
pub async fn maybe_user(pool: &PgPool, jar: &CookieJar) -> AppResult<Option<AuthUser>> {
    let Some(cookie) = jar.get(SESSION_COOKIE_NAME) else {
        return Ok(None);
    };
    lookup(pool, cookie.value()).await
}

// ---- Axum extractor: required-auth ----
//
// Handler signature: `async fn h(user: AuthUser, State(s): State<AppState>)`.
// If the cookie is missing/expired/invalid, the request 401s before the
// handler runs.

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state: AppState = axum::extract::FromRef::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(cookie) = jar.get(SESSION_COOKIE_NAME) else {
            return Err(AppError::Unauthorized);
        };
        match lookup(&app_state.pool, cookie.value()).await? {
            Some(u) => {
                // Touch parts.extensions so downstream layers can reach the
                // user without re-extracting.
                parts.extensions.insert(u.clone());
                Ok(u)
            }
            None => Err(AppError::Unauthorized),
        }
    }
}
