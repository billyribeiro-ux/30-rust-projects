use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{hash, session};
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
pub struct UserOut {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

fn validate_email(email: &str) -> AppResult<String> {
    let t = email.trim().to_lowercase();
    if !t.contains('@')
        || !t.contains('.')
        || t.starts_with('@')
        || t.ends_with('@')
        || t.len() > 320
    {
        return Err(AppError::Fields(vec![FieldError {
            field: "email".into(),
            message: "invalid email".into(),
        }]));
    }
    Ok(t)
}

async fn register(
    State(s): State<AppState>,
    jar: CookieJar,
    Json(input): Json<RegisterInput>,
) -> AppResult<impl IntoResponse> {
    let email = validate_email(&input.email)?;
    let h = hash::hash_password(&input.password)?;
    let name = input.name.unwrap_or_default();
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, password_hash, name) VALUES ($1,$2,$3,$4)",
        id,
        email,
        h,
        name
    )
    .execute(&s.pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("users_email_key") => {
            AppError::Conflict("email already registered".into())
        }
        _ => e.into(),
    })?;
    let raw = session::create(&s.pool, id).await?;
    let cookie = session::session_cookie(raw, s.secure_cookies);
    Ok((
        StatusCode::CREATED,
        jar.add(cookie),
        Json(UserOut { id, email, name }),
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
    let email = validate_email(&input.email)?;
    let row = sqlx::query!(
        "SELECT id, email, name, password_hash FROM users WHERE email = $1",
        email
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    if !hash::verify_password(&input.password, &row.password_hash)? {
        return Err(AppError::Unauthorized);
    }
    let raw = session::create(&s.pool, row.id).await?;
    let cookie = session::session_cookie(raw, s.secure_cookies);
    Ok((
        StatusCode::OK,
        jar.add(cookie),
        Json(UserOut {
            id: row.id,
            email: row.email,
            name: row.name,
        }),
    ))
}

async fn logout(State(s): State<AppState>, jar: CookieJar) -> AppResult<impl IntoResponse> {
    if let Some(c) = jar.get(session::SESSION_COOKIE_NAME) {
        session::revoke(&s.pool, c.value()).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        jar.add(session::clear_session_cookie(s.secure_cookies)),
    ))
}

async fn me(user: session::AuthUser) -> Json<UserOut> {
    Json(UserOut {
        id: user.id,
        email: user.email,
        name: user.name,
    })
}
