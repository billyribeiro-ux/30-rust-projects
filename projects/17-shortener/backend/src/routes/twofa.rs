//! TOTP-based 2FA: setup, verify, disable, and the step-up login flow.
//!
//! Wire model:
//!  1. POST /api/2fa/setup
//!     Auth: requires a logged-in user.
//!     Effect: generates a fresh TOTP secret, stashes it on `users.totp_secret`
//!     but DOES NOT mark 2FA as enabled. Returns:
//!       - provisioning URI (otpauth://totp/...) for manual entry
//!       - QR code PNG (base64) for camera scan
//!  2. POST /api/2fa/verify  { code }
//!     Auth: requires a logged-in user with an unverified secret.
//!     Effect: validates the code against the stashed secret; on success
//!     sets `users.totp_enabled_at = now()` and issues 10 backup codes
//!     (returned ONCE to the user; only hashes survive in the DB).
//!  3. POST /api/2fa/disable  { code }
//!     Auth: requires a logged-in user with 2FA enabled.
//!     Effect: validates code, clears secret + enabled_at + backup codes.
//!  4. Login step-up: see routes/auth.rs login(). If the user has 2FA on,
//!     login returns { requires_2fa: true, intermediate: <raw-token> } instead
//!     of issuing a real session. The intermediate token is a 5-minute
//!     auth_tokens row of kind=totp_intermediate. The client then POSTs
//!     /api/auth/2fa/verify { intermediate, code } to consume it and receive
//!     a real session.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};

use crate::auth::hash;
use crate::auth::session::{self, AuthUser};
use crate::auth::tokens::{self, TokenKind};
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", post(setup))
        .route("/verify", post(verify_setup))
        .route("/disable", post(disable))
}

const ISSUER: &str = "Shortener";
const BACKUP_COUNT: usize = 10;

fn build_totp(secret: Vec<u8>, account: &str) -> AppResult<TOTP> {
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some(ISSUER.to_string()),
        account.to_string(),
    )
    .map_err(|e| AppError::Internal(format!("totp construction: {e}")))
}

#[derive(Debug, Serialize)]
struct SetupResponse {
    provisioning_uri: String,
    /// PNG bytes, base64-encoded (data: URI ready). Easier for the frontend
    /// than streaming binary.
    qr_png_base64: String,
}

async fn setup(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<SetupResponse>> {
    // Generate a fresh 20-byte secret each time setup() is called — even if
    // an unverified secret exists, we overwrite it (cheap, and avoids the
    // failure mode where a user scanned the QR but their phone glitched).
    let secret_bytes = Secret::generate_secret()
        .to_bytes()
        .map_err(|e| AppError::Internal(format!("secret encoding: {e}")))?;

    sqlx::query!(
        r#"UPDATE users SET totp_secret = $1, totp_enabled_at = NULL WHERE id = $2"#,
        secret_bytes,
        user.id,
    )
    .execute(&s.pool)
    .await?;

    let totp = build_totp(secret_bytes, &user.email)?;
    let provisioning_uri = totp.get_url();
    let png_bytes = totp
        .get_qr_png()
        .map_err(|e| AppError::Internal(format!("qr render: {e}")))?;
    let qr_png_base64 = STANDARD.encode(&png_bytes);

    Ok(Json(SetupResponse {
        provisioning_uri,
        qr_png_base64,
    }))
}

#[derive(Debug, Deserialize)]
struct VerifyInput {
    code: String,
}

#[derive(Debug, Serialize)]
struct VerifyResponse {
    /// Plaintext backup codes, returned ONCE. After this response the server
    /// only has the Argon2 hashes.
    backup_codes: Vec<String>,
}

async fn verify_setup(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<VerifyInput>,
) -> AppResult<Json<VerifyResponse>> {
    let secret = sqlx::query_scalar!(r#"SELECT totp_secret FROM users WHERE id = $1"#, user.id,)
        .fetch_one(&s.pool)
        .await?
        .ok_or_else(|| {
            AppError::Validation("call /api/2fa/setup first to generate a secret".into())
        })?;

    let totp = build_totp(secret, &user.email)?;
    let ok = totp
        .check_current(&input.code)
        .map_err(|e| AppError::Internal(format!("totp check: {e}")))?;
    if !ok {
        return Err(AppError::Fields(vec![FieldError {
            field: "code".into(),
            message: "invalid code".into(),
        }]));
    }

    // Generate 10 backup codes (8-byte base32 each), hash them with Argon2.
    let mut plaintext: Vec<String> = Vec::with_capacity(BACKUP_COUNT);
    let mut hashes: Vec<String> = Vec::with_capacity(BACKUP_COUNT);
    for _ in 0..BACKUP_COUNT {
        let mut buf = [0u8; 6];
        rand::rng().fill(&mut buf);
        // Hex-encode for human-typeable codes: 12 chars, all 0-9/a-f.
        let code = hex::encode(buf);
        let h = hash::hash_password(&code)?;
        plaintext.push(format!("{}-{}", &code[..6], &code[6..]));
        hashes.push(h);
    }

    let mut tx = s.pool.begin().await?;
    sqlx::query!(
        r#"UPDATE users SET totp_enabled_at = now() WHERE id = $1"#,
        user.id,
    )
    .execute(&mut *tx)
    .await?;
    // Wipe any previously issued codes — fresh ten on every enable.
    sqlx::query!(r#"DELETE FROM backup_codes WHERE user_id = $1"#, user.id)
        .execute(&mut *tx)
        .await?;
    for h in &hashes {
        sqlx::query!(
            r#"INSERT INTO backup_codes (user_id, code_hash) VALUES ($1, $2)"#,
            user.id,
            h,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(Json(VerifyResponse {
        backup_codes: plaintext,
    }))
}

async fn disable(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<VerifyInput>,
) -> AppResult<StatusCode> {
    let row = sqlx::query!(
        r#"SELECT totp_secret, totp_enabled_at FROM users WHERE id = $1"#,
        user.id,
    )
    .fetch_one(&s.pool)
    .await?;
    if row.totp_enabled_at.is_none() {
        return Err(AppError::Validation("2FA is not enabled".into()));
    }
    let secret = row
        .totp_secret
        .ok_or_else(|| AppError::Internal("enabled without secret".into()))?;
    let totp = build_totp(secret, &user.email)?;
    let ok = totp
        .check_current(&input.code)
        .map_err(|e| AppError::Internal(format!("totp check: {e}")))?;
    if !ok {
        // Allow disable-via-backup-code too — a lost authenticator shouldn't
        // lock the user out of disabling.
        if !consume_backup_code(&s, user.id, &input.code).await? {
            return Err(AppError::Fields(vec![FieldError {
                field: "code".into(),
                message: "invalid code".into(),
            }]));
        }
    }

    let mut tx = s.pool.begin().await?;
    sqlx::query!(
        r#"UPDATE users SET totp_secret = NULL, totp_enabled_at = NULL WHERE id = $1"#,
        user.id,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(r#"DELETE FROM backup_codes WHERE user_id = $1"#, user.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------- step-up login: stage two ----------

#[derive(Debug, Deserialize)]
pub struct StepUpInput {
    pub intermediate: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct StepUpResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub email_verified: bool,
}

pub async fn step_up_verify(
    State(s): State<AppState>,
    jar: CookieJar,
    Json(input): Json<StepUpInput>,
) -> AppResult<impl IntoResponse> {
    // Consume the intermediate token (single-use, 5-minute TTL).
    let user_id = tokens::consume(&s.pool, &input.intermediate, TokenKind::TotpIntermediate)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    let row = sqlx::query!(
        r#"
        SELECT id, email, name, email_verified_at, totp_secret
        FROM users WHERE id = $1
        "#,
        user_id,
    )
    .fetch_one(&s.pool)
    .await?;

    let secret = row.totp_secret.ok_or(AppError::Unauthorized)?;
    let totp = build_totp(secret, &row.email)?;
    let ok = totp
        .check_current(&input.code)
        .map_err(|e| AppError::Internal(format!("totp check: {e}")))?;

    if !ok && !consume_backup_code(&s, user_id, &input.code).await? {
        return Err(AppError::Unauthorized);
    }

    let (raw, _sid) = session::create(&s.pool, user_id, None, None).await?;
    let jar = jar.add(session::session_cookie(raw, s.secure_cookies));
    Ok((
        StatusCode::OK,
        jar,
        Json(StepUpResponse {
            id: row.id,
            email: row.email,
            name: row.name,
            email_verified: row.email_verified_at.is_some(),
        }),
    ))
}

/// Try to consume one backup code for this user. Returns Ok(true) on success.
/// Since we Argon2-hash every code at issuance, we have to verify against
/// every stored hash for the user — a small N (10), so this is fine.
async fn consume_backup_code(s: &AppState, user_id: uuid::Uuid, code: &str) -> AppResult<bool> {
    // Normalize: strip dashes/whitespace, lowercase.
    let normalized: String = code
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if normalized.is_empty() {
        return Ok(false);
    }

    let codes = sqlx::query!(
        r#"SELECT code_hash FROM backup_codes WHERE user_id = $1 AND used_at IS NULL"#,
        user_id,
    )
    .fetch_all(&s.pool)
    .await?;

    for c in codes {
        if hash::verify_password(&normalized, &c.code_hash)? {
            sqlx::query!(
                r#"UPDATE backup_codes SET used_at = now() WHERE user_id = $1 AND code_hash = $2"#,
                user_id,
                c.code_hash,
            )
            .execute(&s.pool)
            .await?;
            return Ok(true);
        }
    }
    Ok(false)
}
