//! Auth routes — port of project 14/15.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::hash;
use crate::auth::session::{self, AuthUser};
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

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
        return Err(AppError::Conflict(
            "an account with that email already exists".into(),
        ));
    }
    result?;

    let (raw, _sid) = session::create(&s.pool, id, None, None).await?;
    let jar = jar.add(session::session_cookie(raw, s.secure_cookies));

    Ok((
        StatusCode::CREATED,
        jar,
        Json(UserPublic { id, email, name }),
    ))
}

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
        r#"SELECT id, email, name, password_hash FROM users WHERE email = $1"#,
        email,
    )
    .fetch_optional(&s.pool)
    .await?;

    const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    let (user_id, name, valid) = match row {
        Some(u) => {
            let ok = hash::verify_password(&input.password, &u.password_hash)?;
            (u.id, u.name, ok)
        }
        None => {
            let _ = hash::verify_password(&input.password, DUMMY_HASH);
            (Uuid::nil(), String::new(), false)
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
        }),
    ))
}

async fn logout(
    State(s): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    session::destroy(&s.pool, user.session_id).await?;
    let jar = jar.add(session::clear_cookie(s.secure_cookies));
    Ok((StatusCode::NO_CONTENT, jar))
}

async fn me(user: AuthUser) -> Json<UserPublic> {
    Json(UserPublic {
        id: user.id,
        email: user.email,
        name: user.name,
    })
}

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
    fn email_rejects_malformed() {
        assert!(matches!(
            normalize_email("not-an-email"),
            Err(AppError::Fields(_))
        ));
    }
}
