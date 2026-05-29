//! WebAuthn enrolment + login. State is stored in `webauthn_states`
//! keyed by a random uuid and survives the round-trip; the client
//! echoes the id back on the `finish` step.
//!
//! The flow:
//!   POST /api/passkeys/register/start  → CreationChallengeResponse
//!   POST /api/passkeys/register/finish → consume state, persist credential
//!   POST /api/passkeys/login/start     → RequestChallengeResponse
//!   POST /api/passkeys/login/finish    → consume state, create session

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum_extra::extract::cookie::CookieJar;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::auth::session;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register/start", post(register_start))
        .route("/register/finish", post(register_finish))
        .route("/login/start", post(login_start))
        .route("/login/finish", post(login_finish))
}

#[derive(Debug, Deserialize)]
pub struct RegisterStartInput {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct StartResp<C> {
    pub state_id: Uuid,
    pub challenge: C,
}

async fn register_start(
    State(s): State<AppState>,
    Json(i): Json<RegisterStartInput>,
) -> AppResult<Json<StartResp<CreationChallengeResponse>>> {
    let email = i.email.trim().to_lowercase();
    if !email.contains('@') {
        return Err(AppError::Validation("invalid email".into()));
    }

    // Reuse the user if they exist (linking a second passkey), else stage
    // an account by creating a user row with NULL password_hash on finish.
    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&s.pool)
        .await?
        .unwrap_or_else(Uuid::new_v4);

    // Existing credentials for this user (so the IdP can dedupe enrolments).
    let exclude = sqlx::query!(
        "SELECT credential_id FROM webauthn_credentials WHERE user_id = $1",
        user_id
    )
    .fetch_all(&s.pool)
    .await?
    .into_iter()
    .map(|r| CredentialID::from(r.credential_id))
    .collect::<Vec<_>>();

    let (ccr, reg_state) = s
        .webauthn
        .start_passkey_registration(user_id, &email, &email, Some(exclude))
        .map_err(|e| AppError::Internal(format!("webauthn: {e}")))?;

    let state_id = Uuid::new_v4();
    let state_json =
        serde_json::to_value(&reg_state).map_err(|e| AppError::Internal(e.to_string()))?;
    let pending = serde_json::json!({
        "email": email,
        "user_id": user_id,
        "reg_state": state_json
    });
    sqlx::query!(
        r#"INSERT INTO webauthn_states (id, user_id, state, kind, expires_at)
           VALUES ($1, $2, $3, 'register', $4)"#,
        state_id,
        user_id,
        pending,
        Utc::now() + Duration::minutes(10)
    )
    .execute(&s.pool)
    .await?;
    Ok(Json(StartResp {
        state_id,
        challenge: ccr,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RegisterFinishInput {
    pub state_id: Uuid,
    pub credential: RegisterPublicKeyCredential,
    pub nickname: Option<String>,
}

async fn register_finish(
    State(s): State<AppState>,
    jar: CookieJar,
    Json(i): Json<RegisterFinishInput>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query!(
        "DELETE FROM webauthn_states WHERE id = $1 AND kind = 'register' AND expires_at > now() RETURNING state, user_id",
        i.state_id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    let pending: serde_json::Value = row.state;
    let user_id: Uuid = row
        .user_id
        .ok_or(AppError::Internal("missing user_id".into()))?;
    let email = pending["email"].as_str().unwrap_or_default().to_string();
    let reg_state: PasskeyRegistration = serde_json::from_value(pending["reg_state"].clone())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let credential = s
        .webauthn
        .finish_passkey_registration(&i.credential, &reg_state)
        .map_err(|e| AppError::Validation(format!("webauthn finish: {e}")))?;

    // Upsert the user (no password) if first credential.
    sqlx::query!(
        r#"INSERT INTO users (id, email) VALUES ($1, $2)
           ON CONFLICT (id) DO UPDATE SET email = EXCLUDED.email"#,
        user_id,
        email
    )
    .execute(&s.pool)
    .await?;
    let cred_id = credential.cred_id().as_ref().to_vec();
    let cred_json =
        serde_json::to_value(&credential).map_err(|e| AppError::Internal(e.to_string()))?;
    sqlx::query!(
        r#"INSERT INTO webauthn_credentials (user_id, credential_id, credential, nickname)
           VALUES ($1, $2, $3, $4)"#,
        user_id,
        cred_id,
        cred_json,
        i.nickname.unwrap_or_default()
    )
    .execute(&s.pool)
    .await?;

    let raw = session::create(&s.pool, user_id).await?;
    let cookie = session::session_cookie(raw, s.secure_cookies);
    Ok((
        StatusCode::CREATED,
        jar.add(cookie),
        Json(serde_json::json!({ "user_id": user_id })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct LoginStartInput {
    pub email: String,
}

async fn login_start(
    State(s): State<AppState>,
    Json(i): Json<LoginStartInput>,
) -> AppResult<Json<StartResp<RequestChallengeResponse>>> {
    let email = i.email.trim().to_lowercase();
    let user = sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let creds: Vec<Passkey> = sqlx::query!(
        "SELECT credential FROM webauthn_credentials WHERE user_id = $1",
        user.id
    )
    .fetch_all(&s.pool)
    .await?
    .into_iter()
    .filter_map(|r| serde_json::from_value(r.credential).ok())
    .collect();
    if creds.is_empty() {
        return Err(AppError::Unauthorized);
    }
    let (req, auth_state) = s
        .webauthn
        .start_passkey_authentication(&creds)
        .map_err(|e| AppError::Internal(format!("webauthn: {e}")))?;
    let state_id = Uuid::new_v4();
    let state_json =
        serde_json::to_value(&auth_state).map_err(|e| AppError::Internal(e.to_string()))?;
    sqlx::query!(
        r#"INSERT INTO webauthn_states (id, user_id, state, kind, expires_at)
           VALUES ($1, $2, $3, 'authenticate', $4)"#,
        state_id,
        user.id,
        state_json,
        Utc::now() + Duration::minutes(5)
    )
    .execute(&s.pool)
    .await?;
    Ok(Json(StartResp {
        state_id,
        challenge: req,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LoginFinishInput {
    pub state_id: Uuid,
    pub credential: PublicKeyCredential,
}

async fn login_finish(
    State(s): State<AppState>,
    jar: CookieJar,
    Json(i): Json<LoginFinishInput>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query!(
        "DELETE FROM webauthn_states WHERE id = $1 AND kind = 'authenticate' AND expires_at > now()
         RETURNING state, user_id",
        i.state_id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    let user_id = row
        .user_id
        .ok_or(AppError::Internal("missing user_id".into()))?;
    let auth_state: PasskeyAuthentication =
        serde_json::from_value(row.state).map_err(|e| AppError::Internal(e.to_string()))?;
    let result = s
        .webauthn
        .finish_passkey_authentication(&i.credential, &auth_state)
        .map_err(|e| AppError::Unauthorized.tap(|_| tracing::warn!(?e, "passkey auth failed")))?;
    let _ = result;
    // Stamp last_used_at on the matching credential (best-effort).
    if let Some(cred_id_bytes) = i.credential.raw_id.as_ref().get(..).map(<[u8]>::to_vec) {
        let _ = sqlx::query!(
            "UPDATE webauthn_credentials SET last_used_at = now()
             WHERE user_id = $1 AND credential_id = $2",
            user_id,
            cred_id_bytes
        )
        .execute(&s.pool)
        .await;
    }
    let raw = session::create(&s.pool, user_id).await?;
    let cookie = session::session_cookie(raw, s.secure_cookies);
    Ok((
        StatusCode::OK,
        jar.add(cookie),
        Json(serde_json::json!({ "user_id": user_id })),
    ))
}

// A "tap"-style helper for the .map_err chain above — keeps the error and
// runs a side effect.
trait Tap {
    fn tap<F: FnOnce(&Self)>(self, f: F) -> Self;
}
impl<T> Tap for T {
    fn tap<F: FnOnce(&Self)>(self, f: F) -> Self {
        f(&self);
        self
    }
}
