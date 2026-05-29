//! Magic-link auth for customers. Email-only.

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum_extra::extract::cookie::CookieJar;
use chrono::{Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

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
    pub tenant_slug: String,
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
    // Tenant lookup is a precondition — but we never reveal whether the
    // tenant exists or the email is on it. Always return 204.
    let tenant = sqlx::query!("SELECT id FROM tenants WHERE slug = $1", input.tenant_slug)
        .fetch_optional(&s.pool)
        .await?;
    if let Some(t) = tenant {
        let raw = session::generate_token();
        let token_hash = session::hash_token(&raw);
        let expires = Utc::now() + Duration::minutes(10);
        sqlx::query!(
            r#"INSERT INTO magic_links (token_hash, email, tenant_id, expires_at)
               VALUES ($1, $2, $3, $4)"#,
            token_hash,
            email,
            t.id,
            expires
        )
        .execute(&s.pool)
        .await?;
        let link = format!("{}/api/magic/verify?token={raw}", s.public_url);
        if let Err(e) = send_email(&s.smtp_url, &s.smtp_from, &email, &link).await {
            tracing::warn!(?e, "magic email send failed");
        }
    }
    Ok((StatusCode::NO_CONTENT, ()))
}

async fn send_email(smtp_url: &str, from: &str, to: &str, link: &str) -> AppResult<()> {
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
    let from = from
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| AppError::Internal(format!("from: {e}")))?;
    let to = to
        .parse::<lettre::message::Mailbox>()
        .map_err(|e| AppError::Internal(format!("to: {e}")))?;
    let email = Message::builder()
        .from(from)
        .to(to)
        .subject("Sign in")
        .body(format!("Tap to sign in (10-minute window): {link}"))
        .map_err(|e| AppError::Internal(format!("msg: {e}")))?;
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
        RETURNING email, tenant_id
        "#,
        token_hash
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    // Upsert user.
    let user_id = sqlx::query_scalar!(
        r#"
        INSERT INTO users (email) VALUES ($1)
        ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
        RETURNING id
        "#,
        row.email
    )
    .fetch_one(&s.pool)
    .await?;

    // Auto-add as customer if not yet a member.
    sqlx::query!(
        r#"INSERT INTO memberships (tenant_id, user_id, role)
           VALUES ($1, $2, 'customer')
           ON CONFLICT (tenant_id, user_id) DO NOTHING"#,
        row.tenant_id,
        user_id
    )
    .execute(&s.pool)
    .await?;

    let raw = session::create(&s.pool, user_id).await?;
    let cookie = session::session_cookie(raw, s.secure_cookies);
    let _: Uuid = user_id;
    Ok((jar.add(cookie), Redirect::to(&format!("{}/", s.public_url))))
}
