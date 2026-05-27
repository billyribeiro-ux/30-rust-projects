//! tus-style resumable upload protocol (simplified).
//!
//! - POST   /api/uploads        → create session, return { upload_id }
//! - HEAD   /api/uploads/{id}   → return current Upload-Offset for resume
//! - PATCH  /api/uploads/{id}   → append chunk at Upload-Offset, return new offset
//!
//! When the cumulative offset reaches total_size we finalise:
//!   1. Stream the temp file from disk and SHA-256 it.
//!   2. Sniff MIME from the first ~16 bytes (`infer`).
//!   3. If JPEG/PNG, decode-then-re-encode to strip EXIF/XMP/etc.
//!   4. If a file_versions row already exists for this hash, REUSE it
//!      (dedup — we never put the blob twice).
//!   5. Otherwise put the bytes via `object_store` and insert
//!      `file_versions`.
//!   6. Insert the user-visible `files` row.
//!   7. Delete the upload session + temp file.

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, response::Json as JsonResp};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::blob;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create))
        .route(
            "/{id}",
            axum::routing::patch(append).head(head).delete(cancel),
        )
}

// ---------------- create ----------------

#[derive(Deserialize)]
pub struct CreateInput {
    pub filename: String,
    pub size: i64,
    pub folder_id: Option<Uuid>,
    pub content_type: Option<String>,
}

#[derive(Serialize)]
pub struct CreateOutput {
    pub upload_id: Uuid,
}

const MAX_FILE_SIZE: i64 = 5 * 1024 * 1024 * 1024; // 5 GiB
const SESSION_TTL_HOURS: i64 = 24;

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    let filename = input.filename.trim().to_string();
    if filename.is_empty() || filename.chars().count() > 255 {
        return Err(AppError::Fields(vec![FieldError {
            field: "filename".into(),
            message: "1-255 characters required".into(),
        }]));
    }
    if input.size < 0 || input.size > MAX_FILE_SIZE {
        return Err(AppError::TooLarge);
    }
    if let Some(folder_id) = input.folder_id {
        let owns = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND owner_id = $2)",
            folder_id,
            user.id,
        )
        .fetch_one(&s.pool)
        .await?
        .unwrap_or(false);
        if !owns {
            return Err(AppError::NotFound);
        }
    }

    let id = Uuid::new_v4();
    let temp_path = s.upload_tmp.join(format!("{id}.part"));
    // Pre-create the file so a 0-byte upload still has the same code path.
    tokio::fs::File::create(&temp_path).await?;
    let expires_at = Utc::now() + Duration::hours(SESSION_TTL_HOURS);
    let content_type = input
        .content_type
        .unwrap_or_else(|| "application/octet-stream".into());
    let temp_path_s = temp_path.to_string_lossy().to_string();

    sqlx::query!(
        r#"
        INSERT INTO upload_sessions
            (id, owner_id, folder_id, filename, content_type,
             total_size, received, temp_path, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8)
        "#,
        id,
        user.id,
        input.folder_id,
        filename,
        content_type,
        input.size,
        temp_path_s,
        expires_at,
    )
    .execute(&s.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(CreateOutput { upload_id: id })))
}

// ---------------- head ----------------

async fn head(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query!(
        r#"
        SELECT received, total_size FROM upload_sessions
        WHERE id = $1 AND owner_id = $2 AND expires_at > now()
        "#,
        id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let mut headers = HeaderMap::new();
    headers.insert("upload-offset", HeaderValue::from(row.received));
    headers.insert("upload-length", HeaderValue::from(row.total_size));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((StatusCode::OK, headers))
}

// ---------------- append (PATCH) ----------------

async fn append(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    let offset_hdr = headers
        .get("upload-offset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .ok_or_else(|| AppError::Validation("Upload-Offset header required".into()))?;

    let session = sqlx::query!(
        r#"
        SELECT received, total_size, temp_path, folder_id, filename, content_type
        FROM upload_sessions
        WHERE id = $1 AND owner_id = $2 AND expires_at > now()
        "#,
        id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    if offset_hdr != session.received {
        // 409 Conflict in tus-speak.
        return Err(AppError::Conflict(format!(
            "Upload-Offset {offset_hdr} does not match server offset {}",
            session.received
        )));
    }

    let body_len = body.len() as i64;
    let new_offset = session.received + body_len;
    if new_offset > session.total_size {
        return Err(AppError::TooLarge);
    }

    // Append to the temp file. We open-write-close per chunk: simpler than
    // holding an open handle in state, and the OS page cache keeps the
    // per-chunk overhead low. Production with very-many-tiny-chunks
    // workloads would batch.
    let mut f = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&session.temp_path)
        .await?;
    f.write_all(&body).await?;
    f.flush().await?;
    drop(f);

    sqlx::query!(
        "UPDATE upload_sessions SET received = $1 WHERE id = $2",
        new_offset,
        id,
    )
    .execute(&s.pool)
    .await?;

    // Final chunk → assemble.
    if new_offset == session.total_size {
        let bytes = tokio::fs::read(&session.temp_path).await?;
        let bytes = Bytes::from(bytes);
        let sha = blob::sha256_hex(&bytes);
        let sniffed = blob::sniff_mime(&bytes);
        let final_bytes = if blob::is_strippable_image(&sniffed) {
            blob::strip_exif(&bytes, &sniffed)
        } else {
            bytes
        };
        // After EXIF strip the bytes may have shifted, so re-hash the stored
        // blob. The "dedup" key is the *stored* hash, not the pre-strip hash —
        // otherwise two clients uploading the same JPEG with different EXIF
        // would end up writing the blob twice.
        let final_sha = if blob::is_strippable_image(&sniffed) {
            blob::sha256_hex(&final_bytes)
        } else {
            sha
        };
        let final_size = final_bytes.len() as i64;

        // dedup check
        let existing = sqlx::query!(
            "SELECT id FROM file_versions WHERE sha256 = $1",
            final_sha,
        )
        .fetch_optional(&s.pool)
        .await?;

        let version_id = if let Some(v) = existing {
            v.id
        } else {
            // belt-and-braces: the DB may not have a row yet but the blob
            // store could already (a half-finished previous attempt). The
            // `put` is idempotent for both LocalFS and S3.
            s.storage.put(&final_sha, final_bytes).await?;
            let vid = Uuid::new_v4();
            sqlx::query!(
                r#"
                INSERT INTO file_versions (id, sha256, size, mime)
                VALUES ($1, $2, $3, $4)
                "#,
                vid,
                final_sha,
                final_size,
                sniffed,
            )
            .execute(&s.pool)
            .await?;
            vid
        };

        let file_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO files (id, folder_id, owner_id, name, version_id)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            file_id,
            session.folder_id,
            user.id,
            session.filename,
            version_id,
        )
        .execute(&s.pool)
        .await?;

        // Clean up: drop the upload session row and the temp file.
        sqlx::query!("DELETE FROM upload_sessions WHERE id = $1", id)
            .execute(&s.pool)
            .await?;
        let _ = tokio::fs::remove_file(&session.temp_path).await;

        let mut response_headers = HeaderMap::new();
        response_headers.insert("upload-offset", HeaderValue::from(new_offset));
        response_headers.insert(
            "x-file-id",
            HeaderValue::from_str(&file_id.to_string()).unwrap(),
        );
        return Ok((StatusCode::CREATED, response_headers).into_response());
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert("upload-offset", HeaderValue::from(new_offset));
    Ok((StatusCode::NO_CONTENT, response_headers).into_response())
}

async fn cancel(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let row = sqlx::query!(
        "DELETE FROM upload_sessions WHERE id = $1 AND owner_id = $2 RETURNING temp_path",
        id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?;
    if let Some(r) = row {
        let _ = tokio::fs::remove_file(&r.temp_path).await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

// ---------------- background reaper ----------------

/// Delete upload sessions that are past `expires_at`, plus their temp files.
/// Called from a tokio task in main.rs on a 1-hour cadence.
pub async fn reap_stale(pool: &PgPool) -> AppResult<u64> {
    let rows = sqlx::query!(
        "DELETE FROM upload_sessions WHERE expires_at < now() RETURNING temp_path",
    )
    .fetch_all(pool)
    .await?;
    let n = rows.len() as u64;
    for r in rows {
        let _ = tokio::fs::remove_file(&r.temp_path).await;
    }
    if n > 0 {
        tracing::info!(reaped = n, "stale upload sessions removed");
    }
    Ok(n)
}

// Re-export JsonResp for symmetry with files.rs (some IDEs warn on it).
#[allow(dead_code)]
type _Json<T> = JsonResp<T>;
