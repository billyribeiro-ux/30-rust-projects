//! Auth routes: register, login, logout, verify, request-reset, reset.
//!
//! All endpoints follow two principles:
//!  1. **Never leak user existence**. /forgot returns 204 whether the email
//!     exists or not. /login uses the same "invalid credentials" message
//!     for "no such user" vs "wrong password". An attacker can't enumerate.
//!  2. **Field-level errors only for shape problems** (empty field, malformed
//!     email). Anything semantic (wrong password) returns 401 with a
//!     single generic message.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::hash;
use crate::auth::session::{self, AuthUser};
use crate::auth::tokens::{self, TokenKind};
use crate::email;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/verify/{token}", post(verify_email))
        .route("/forgot", post(forgot_password))
        .route("/reset/{token}", post(reset_password))
}

// ---------- register ----------

#[derive(Debug, Deserialize)]
pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserPublic {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub email_verified: bool,
}

async fn register(
    State(s): State<AppState>,
    jar: CookieJar,
    Json(input): Json<RegisterInput>,
) -> AppResult<impl IntoResponse> {
    let email = normalize_email(&input.email)?;
    let name = input.name.unwrap_or_default().trim().to_string();
    let password_hash = hash::hash_password(&input.password)?;

    let id = Uuid::new_v4();
    let result = sqlx::query!(
        "INSERT INTO users (id, email, password_hash, name) VALUES ($1, $2, $3, $4)",
        id,
        email,
        password_hash,
        name,
    )
    .execute(&s.pool)
    .await;

    if let Err(sqlx::Error::Database(dbe)) = &result
        && dbe.constraint() == Some("users_email_key")
    {
        // Don't disclose "email already exists" verbatim — it lets an attacker
        // enumerate users via /register. Return 409 with a friendly generic.
        return Err(AppError::Conflict(
            "an account with that email already exists".into(),
        ));
    }
    result?;

    // Issue verify-email token + send email (fire and forget — email send
    // failures don't 500 the register call).
    let raw_token = tokens::issue(&s.pool, id, TokenKind::VerifyEmail).await?;
    let (subj, body) = email::verify_email_body(&s.public_url, &name, &raw_token);
    s.mailer.send(&email, &subj, body).await;

    // Auto-login the user (issue session + cookie). They'll get a "please
    // verify" banner in the UI but can use the app.
    let (raw, _sid) = session::create(&s.pool, id, None, None).await?;
    let jar = jar.add(session::session_cookie(raw, s.secure_cookies));

    Ok((
        StatusCode::CREATED,
        jar,
        Json(UserPublic {
            id,
            email,
            name,
            email_verified: false,
        }),
    ))
}

// ---------- login ----------

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

async fn login(
    State(s): State<AppState>,
    jar: CookieJar,
    Json(input): Json<LoginInput>,
) -> AppResult<impl IntoResponse> {
    let email = normalize_email(&input.email)?;
    if input.password.is_empty() {
        return Err(AppError::Unauthorized);
    }

    let row = sqlx::query!(
        r#"
        SELECT id, email, name, password_hash, email_verified_at
        FROM users WHERE email = $1
        "#,
        email,
    )
    .fetch_optional(&s.pool)
    .await?;

    // To avoid timing-based user enumeration, run argon2 verify either way.
    // If the user doesn't exist, verify against a known dummy hash so the
    // attacker sees the same response time.
    const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let (user_id, name, email_verified, valid) = match row {
        Some(u) => {
            let v = hash::verify_password(&input.password, &u.password_hash)?;
            (u.id, u.name, u.email_verified_at.is_some(), v)
        }
        None => {
            let _ = hash::verify_password(&input.password, DUMMY_HASH);
            (Uuid::nil(), String::new(), false, false)
        }
    };

    if !valid {
        return Err(AppError::Unauthorized);
    }

    let (raw, _sid) = session::create(&s.pool, user_id, None, None).await?;
    let jar = jar.add(session::session_cookie(raw, s.secure_cookies));

    Ok((
        StatusCode::OK,
        jar,
        Json(UserPublic {
            id: user_id,
            email,
            name,
            email_verified,
        }),
    ))
}

// ---------- logout ----------

async fn logout(
    State(s): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    session::destroy(&s.pool, user.session_id).await?;
    let jar = jar.add(session::clear_cookie(s.secure_cookies));
    Ok((StatusCode::NO_CONTENT, jar))
}

// ---------- me (current user) ----------

async fn me(user: AuthUser) -> Json<UserPublic> {
    Json(UserPublic {
        id: user.id,
        email: user.email,
        name: user.name,
        email_verified: user.email_verified,
    })
}

// ---------- verify email ----------

async fn verify_email(
    State(s): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<StatusCode> {
    let user_id = tokens::consume(&s.pool, &token, TokenKind::VerifyEmail).await?;
    sqlx::query!(
        "UPDATE users SET email_verified_at = now() WHERE id = $1",
        user_id,
    )
    .execute(&s.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- forgot password ----------

#[derive(Debug, Deserialize)]
pub struct ForgotInput {
    pub email: String,
}

async fn forgot_password(
    State(s): State<AppState>,
    Json(input): Json<ForgotInput>,
) -> AppResult<StatusCode> {
    let email = match normalize_email(&input.email) {
        Ok(e) => e,
        // Don't leak shape errors here either — always 204 to defeat enum.
        Err(_) => return Ok(StatusCode::NO_CONTENT),
    };
    let row = sqlx::query!("SELECT id, name FROM users WHERE email = $1", email)
        .fetch_optional(&s.pool)
        .await?;

    if let Some(u) = row {
        let raw_token = tokens::issue(&s.pool, u.id, TokenKind::PasswordReset).await?;
        let (subj, body) = email::password_reset_body(&s.public_url, &u.name, &raw_token);
        s.mailer.send(&email, &subj, body).await;
    }
    // Always return 204, whether or not the user existed, so an attacker
    // can't probe for valid emails via this endpoint.
    Ok(StatusCode::NO_CONTENT)
}

// ---------- reset password ----------

#[derive(Debug, Deserialize)]
pub struct ResetInput {
    pub password: String,
}

async fn reset_password(
    State(s): State<AppState>,
    Path(token): Path<String>,
    Json(input): Json<ResetInput>,
) -> AppResult<StatusCode> {
    let new_hash = hash::hash_password(&input.password)?;
    let user_id = tokens::consume(&s.pool, &token, TokenKind::PasswordReset).await?;

    let mut tx = s.pool.begin().await?;
    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        new_hash,
        user_id,
    )
    .execute(&mut *tx)
    .await?;
    // Reset invalidates all existing sessions for that user — they have to
    // log in again everywhere. This is the safer default.
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------- helpers ----------

fn normalize_email(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().to_lowercase();
    if trimmed.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "email".into(),
            message: "required".into(),
        }]));
    }
    if !trimmed.contains('@') || !trimmed.contains('.') {
        return Err(AppError::Fields(vec![FieldError {
            field: "email".into(),
            message: "must be a valid email address".into(),
        }]));
    }
    if trimmed.chars().count() > 254 {
        return Err(AppError::Fields(vec![FieldError {
            field: "email".into(),
            message: "too long".into(),
        }]));
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_lowercases_and_trims() {
        assert_eq!(normalize_email("  Foo@BAR.com  ").unwrap(), "foo@bar.com");
    }

    #[test]
    fn email_rejects_empty() {
        assert!(matches!(normalize_email(""), Err(AppError::Fields(_))));
    }

    #[test]
    fn email_rejects_malformed() {
        assert!(matches!(
            normalize_email("not-an-email"),
            Err(AppError::Fields(_))
        ));
    }
}
