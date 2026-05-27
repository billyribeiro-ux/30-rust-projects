//! Single-use email verification + password reset tokens. Same pattern
//! as project 14 (reused verbatim — see that LESSON for the rationale).

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::session;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // PasswordReset is wired but we don't ship a UI for it here.
pub enum TokenKind {
    VerifyEmail,
    PasswordReset,
}

impl TokenKind {
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            TokenKind::VerifyEmail => "verify_email",
            TokenKind::PasswordReset => "password_reset",
        }
    }
    #[allow(dead_code)]
    pub fn ttl(self) -> Duration {
        match self {
            TokenKind::VerifyEmail => Duration::hours(24),
            TokenKind::PasswordReset => Duration::hours(1),
        }
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
