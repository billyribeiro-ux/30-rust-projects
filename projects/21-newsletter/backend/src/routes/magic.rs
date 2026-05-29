//! Magic-link subscriber auth. POST email → sends single-use link. GET
//! verify → consumes token, creates subscriber session.

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum_extra::extract::cookie::CookieJar;
use chrono::{Duration, Utc};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;

use crate::auth::session;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/start", post(start))
        .route("/verify", get(verify))
}

#[derive(Debug, Deserialize)]
pub struct StartInput {
    pub email: String,
}

async fn start(
    State(s): State<AppState>,
    Json(input): Json<StartInput>,
) -> AppResult<impl IntoResponse> {
    let email = input.email.trim().to_lowercase();
    if !email.contains('@') {
        return Err(AppError::Fields(vec![FieldError {
            field: "email".into(),
            message: "invalid email address".into(),
        }]));
    }
    let subscriber = sqlx::query!("SELECT id FROM subscribers WHERE email = $1", email)
        .fetch_optional(&s.pool)
        .await?
        // Don't leak whether the email is subscribed — pretend success.
        .map(|r| r.id);
    if let Some(id) = subscriber {
        let raw = session::generate_token();
        let token_hash = session::hash_token(&raw);
        let expires = Utc::now() + Duration::minutes(10);
        sqlx::query!(
            r#"INSERT INTO magic_links (token_hash, subscriber_id, expires_at)
               VALUES ($1, $2, $3)"#,
            token_hash,
            id,
            expires
        )
        .execute(&s.pool)
        .await?;
        let link = format!("{}/api/magic/verify?token={raw}", s.public_url);
        if let Err(e) = send_magic_email(&s.smtp_url, &s.smtp_from, &email, &link).await {
            tracing::warn!(?e, "magic email send failed (link still valid)");
        }
    }
    Ok((StatusCode::NO_CONTENT, ()))
}

async fn send_magic_email(smtp_url: &str, from: &str, to: &str, link: &str) -> AppResult<()> {
    let from = from
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| AppError::Internal(format!("from addr: {e}")))?;
    let to = to
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| AppError::Internal(format!("to addr: {e}")))?;
    let body = format!("Tap to sign in (10-minute window): {link}");
    let email = Message::builder()
        .from(from)
        .to(to)
        .subject("Your sign-in link")
        .body(body)
        .map_err(|e| AppError::Internal(format!("message build: {e}")))?;
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::from_url(smtp_url)
            .map_err(|e| AppError::Internal(format!("smtp url: {e}")))?
            .build();
    mailer
        .send(email)
        .await
        .map_err(|e| AppError::Internal(format!("smtp send: {e}")))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct VerifyQuery {
    pub token: String,
}

async fn verify(
    State(s): State<AppState>,
    jar: CookieJar,
    Query(q): Query<VerifyQuery>,
) -> AppResult<impl IntoResponse> {
    let token_hash = session::hash_token(&q.token);
    let row = sqlx::query!(
        r#"
        UPDATE magic_links
        SET used_at = now()
        WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
        RETURNING subscriber_id
        "#,
        token_hash
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    let raw = session::create_subscriber_session(&s.pool, row.subscriber_id).await?;
    let cookie = session::subscriber_cookie(raw, s.secure_cookies);
    Ok((jar.add(cookie), Redirect::to(&format!("{}/", s.public_url))))
}
