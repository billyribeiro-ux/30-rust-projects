//! Test-only endpoints. Compiled in unconditionally so e2e can inject a
//! valid session without driving the real OAuth provider (which requires
//! browser interaction outside our control). Guarded by `TEST_ONLY_TOKEN`
//! — if the env var is unset, every request returns 404.
//!
//! Production deployments MUST leave `TEST_ONLY_TOKEN` unset.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::session;
use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/login-as", post(login_as))
}

#[derive(Debug, Deserialize)]
pub struct LoginAsInput {
    pub email: String,
    /// Optional display name — only used if the user is created on the fly.
    pub name: Option<String>,
}

/// Authenticates by *email only*. If `TEST_ONLY_TOKEN` is unset → 404.
/// If the header `x-test-token` doesn't match → 401.
/// If the email isn't registered → 404.
async fn login_as(
    State(s): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(input): Json<LoginAsInput>,
) -> AppResult<impl IntoResponse> {
    let Ok(expected) = std::env::var("TEST_ONLY_TOKEN") else {
        return Ok((StatusCode::NOT_FOUND, jar, "").into_response());
    };
    if headers
        .get("x-test-token")
        .map(|h| h.as_bytes())
        .unwrap_or_default()
        != expected.as_bytes()
    {
        return Ok((StatusCode::UNAUTHORIZED, jar, "").into_response());
    }
    let email = input.email.trim().to_lowercase();
    let row = sqlx::query!(
        r#"SELECT id, email, name FROM users WHERE email = $1"#,
        email
    )
    .fetch_optional(&s.pool)
    .await?;
    let (id, name): (Uuid, String) = if let Some(r) = row {
        (r.id, r.name)
    } else {
        // Auto-create — handy for fresh e2e runs.
        let id = Uuid::new_v4();
        let display = input.name.unwrap_or_else(|| email.clone());
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, password_hash, name)
            VALUES ($1, $2, NULL, $3)
            "#,
            id,
            email,
            display,
        )
        .execute(&s.pool)
        .await?;
        (id, display)
    };
    let (raw, _sid) = session::create(&s.pool, id, None, None).await?;
    let cookie = Cookie::build((session::SESSION_COOKIE_NAME, raw))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(s.secure_cookies)
        .build();
    let _ = (email, name);
    Ok((
        StatusCode::OK,
        jar.add(cookie),
        Json(serde_json::json!({ "ok": true })),
    )
        .into_response())
}
