//! GET /d/{token}
//!
//! Where the customer's email link points. We:
//!   1) HMAC-verify the signed token (no DB hit yet — invalid tokens
//!      bounce in microseconds).
//!   2) Look up the matching row in `download_links` by SHA-256 hash.
//!   3) Reject if expired / revoked / used (configurable).
//!   4) Stream the file from disk.

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio_util::io::ReaderStream;

use crate::error::{AppError, AppResult};
use crate::signing;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/{token}", get(download))
}

async fn download(State(s): State<AppState>, Path(token): Path<String>) -> AppResult<Response> {
    // 1) Fail fast on signature.
    if signing::verify(&s.download_signing_secret, &token).is_none() {
        return Err(AppError::NotFound);
    }

    // 2) DB lookup by hash.
    let token_hash = signing::hash_token(&token);
    let row = sqlx::query!(
        r#"
        SELECT dl.id, dl.expires_at, dl.used_at, dl.revoked_at,
               p.file_path, p.file_name, p.name AS product_name
        FROM download_links dl
        JOIN products p ON p.id = dl.product_id
        WHERE dl.token_hash = $1
        "#,
        token_hash,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.revoked_at.is_some() {
        return Err(AppError::Gone);
    }
    if row.expires_at < chrono::Utc::now() {
        return Err(AppError::Gone);
    }
    // Note: we intentionally allow re-download (multiple GETs) within the
    // 24h window. Some email clients pre-fetch URLs to scan them — first-
    // use would otherwise burn the link. We still RECORD `used_at` for
    // audit, but don't block re-use.
    let file_path = row.file_path.ok_or_else(|| {
        AppError::Internal("download_links row points to product with no file".into())
    })?;
    let display_name = row
        .file_name
        .unwrap_or_else(|| format!("{}.bin", row.product_name));

    // 3) Mark as used (best-effort; don't fail the download if this fails).
    let _ = sqlx::query!(
        "UPDATE download_links SET used_at = COALESCE(used_at, now()) WHERE id = $1",
        row.id,
    )
    .execute(&s.pool)
    .await;

    // 4) Stream. The file lives under PRODUCT_FILES_DIR; we join carefully
    //    to avoid traversal even though the path came from our own DB.
    let safe_path = s.product_files_dir.join(sanitise(&file_path));
    let file = tokio::fs::File::open(&safe_path).await.map_err(|e| {
        tracing::error!(?safe_path, err = %e, "failed to open product file");
        AppError::Internal("file unavailable".into())
    })?;

    let metadata = file.metadata().await.ok();
    let size = metadata.map(|m| m.len());

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut h = HeaderMap::new();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    let disposition = format!(
        "attachment; filename=\"{}\"",
        display_name.replace('"', "_")
    );
    if let Ok(hv) = HeaderValue::from_str(&disposition) {
        h.insert(header::CONTENT_DISPOSITION, hv);
    }
    if let Some(sz) = size
        && let Ok(hv) = HeaderValue::from_str(&sz.to_string())
    {
        h.insert(header::CONTENT_LENGTH, hv);
    }

    Ok((StatusCode::OK, h, body).into_response())
}

/// Last-line-of-defence path sanitiser. The file_path coming from our
/// DB is admin-controlled, but we still strip any ".." traversal and
/// leading slashes so a misconfigured admin can't escape the products
/// directory.
fn sanitise(p: &str) -> String {
    p.split(['/', '\\'])
        .filter(|c| !c.is_empty() && *c != "." && *c != "..")
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitise_strips_traversal() {
        assert_eq!(sanitise("../etc/passwd"), "etc/passwd");
        assert_eq!(sanitise("/a/../b"), "a/b");
        assert_eq!(sanitise("normal.pdf"), "normal.pdf");
        assert_eq!(sanitise("sub/file.zip"), "sub/file.zip");
    }
}
