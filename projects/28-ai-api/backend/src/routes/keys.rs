//! API key management. Plaintext shown once at creation, then only the
//! prefix is visible. The full secret is SHA-256-hashed at rest.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}/revoke", post(revoke))
}

#[derive(Debug, Serialize)]
pub struct KeyOut {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub rpm_limit: i32,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<KeyOut>>> {
    let rows = sqlx::query!(
        "SELECT id, name, prefix, scopes, rpm_limit, revoked_at, created_at, last_used_at
         FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
        user.id
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| KeyOut {
                id: r.id,
                name: r.name,
                prefix: r.prefix,
                scopes: r.scopes,
                rpm_limit: r.rpm_limit,
                revoked: r.revoked_at.is_some(),
                created_at: r.created_at,
                last_used_at: r.last_used_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateInput {
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub rpm_limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CreatedKey {
    pub id: Uuid,
    pub secret: String,
    pub prefix: String,
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(i): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    if i.name.trim().is_empty() {
        return Err(AppError::Validation("name required".into()));
    }
    // 32 random bytes → base64url-no-pad → 43 chars.
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    let secret_body = URL_SAFE_NO_PAD.encode(bytes);
    let secret = format!("sk_live_{secret_body}");
    let prefix = secret_body.chars().take(8).collect::<String>();
    let hash = Sha256::digest(secret.as_bytes()).to_vec();
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO api_keys (id, user_id, name, prefix, hash, scopes, rpm_limit)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        id,
        user.id,
        i.name.trim(),
        prefix,
        hash,
        &i.scopes.unwrap_or_default(),
        i.rpm_limit.unwrap_or(60)
    )
    .execute(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(CreatedKey { id, secret, prefix })))
}

async fn revoke(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let n = sqlx::query!(
        "UPDATE api_keys SET revoked_at = COALESCE(revoked_at, now())
         WHERE id = $1 AND user_id = $2",
        id,
        user.id
    )
    .execute(&s.pool)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Resolve a `Bearer <secret>` header into an api_key row, applying
/// revoked + rpm-limit checks.
pub async fn authenticate(pool: &sqlx::PgPool, raw_header: &str) -> AppResult<(Uuid, Uuid, i32)> {
    let secret = raw_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;
    let hash = Sha256::digest(secret.as_bytes()).to_vec();
    let row = sqlx::query!(
        r#"SELECT id, user_id, rpm_limit, revoked_at
           FROM api_keys WHERE hash = $1"#,
        hash
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    if row.revoked_at.is_some() {
        return Err(AppError::Unauthorized);
    }
    // RPM limit: count usage_events in the last 60s.
    let count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "n!" FROM usage_events
           WHERE api_key_id = $1 AND ts > now() - INTERVAL '1 minute'"#,
        row.id
    )
    .fetch_one(pool)
    .await?;
    if count >= row.rpm_limit as i64 {
        return Err(AppError::Forbidden);
    }
    Ok((row.id, row.user_id, row.rpm_limit))
}
