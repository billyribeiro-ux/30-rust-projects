//! Server-side admin sessions. Identical wire format to project 14:
//! 32-byte random token in cookie; SHA-256(token) stored in DB.

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

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    #[allow(dead_code)]
    pub email_verified: bool,
    pub session_id: Uuid,
}

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(raw: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    h.finalize().to_vec()
}

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

pub async fn destroy(pool: &PgPool, session_id: Uuid) -> AppResult<()> {
    sqlx::query!("DELETE FROM sessions WHERE id = $1", session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn session_cookie(raw_token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, raw_token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(SESSION_TTL_DAYS))
        .build()
}

pub fn clear_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, ""))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build()
}

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
                parts.extensions.insert(u.clone());
                Ok(u)
            }
            None => Err(AppError::Unauthorized),
        }
    }
}
