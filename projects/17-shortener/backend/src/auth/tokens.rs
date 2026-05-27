//! Single-use email verification + password reset tokens.
//!
//! Same wire format as session tokens (32 random bytes, base64url-encoded;
//! SHA-256 hash stored). Distinguished by `kind`. TTL is short:
//! - verify_email: 24h
//! - password_reset: 1h (the shorter, the safer)
//!
//! Tokens are consumed exactly once (via `consume_token`): the row's
//! consumed_at is set in the same UPDATE that returns it, so two
//! concurrent verify attempts can't both succeed.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::session;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub enum TokenKind {
    VerifyEmail,
    PasswordReset,
    /// Step-up 2FA intermediate token: 5-minute window between the
    /// password-stage success and the TOTP-stage verify. The user does NOT
    /// have a real session yet — this is just proof that they completed
    /// stage one. See routes/twofa.rs for the flow.
    TotpIntermediate,
}

impl TokenKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TokenKind::VerifyEmail => "verify_email",
            TokenKind::PasswordReset => "password_reset",
            TokenKind::TotpIntermediate => "totp_intermediate",
        }
    }
    pub fn ttl(self) -> Duration {
        match self {
            TokenKind::VerifyEmail => Duration::hours(24),
            TokenKind::PasswordReset => Duration::hours(1),
            // Short on purpose — if the user can't get a code out of their
            // authenticator in 5 minutes, they should re-enter their password.
            TokenKind::TotpIntermediate => Duration::minutes(5),
        }
    }
}

/// Issue a fresh token. Returns the RAW token to embed in the email link.
/// The DB never sees the raw value.
pub async fn issue(pool: &PgPool, user_id: Uuid, kind: TokenKind) -> AppResult<String> {
    let raw = session::generate_token();
    let token_hash = session::hash_token(&raw);
    let id = Uuid::new_v4();
    let expires_at = Utc::now() + kind.ttl();
    let kind_str = kind.as_str();

    sqlx::query!(
        r#"
        INSERT INTO auth_tokens (id, user_id, kind, token_hash, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        id,
        user_id,
        kind_str,
        token_hash,
        expires_at,
    )
    .execute(pool)
    .await?;

    Ok(raw)
}

/// Consume a token: look up by raw value, mark consumed_at = now() in the
/// same UPDATE so it's single-use, return the associated user_id.
///
/// Returns AppError::NotFound if the token is unknown, expired, or already
/// consumed. We treat all three the same to avoid disclosing which.
pub async fn consume(pool: &PgPool, raw_token: &str, kind: TokenKind) -> AppResult<Uuid> {
    let token_hash = session::hash_token(raw_token);
    let kind_str = kind.as_str();

    let row = sqlx::query!(
        r#"
        UPDATE auth_tokens
        SET consumed_at = now()
        WHERE token_hash = $1
          AND kind = $2
          AND expires_at > now()
          AND consumed_at IS NULL
        RETURNING user_id
        "#,
        token_hash,
        kind_str,
    )
    .fetch_optional(pool)
    .await?;

    row.map(|r| r.user_id).ok_or(AppError::NotFound)
}
