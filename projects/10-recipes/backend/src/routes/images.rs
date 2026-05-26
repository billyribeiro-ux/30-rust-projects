use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::uploads::{MAX_UPLOAD_BYTES, delete_from_disk, process_image, write_to_disk};

#[derive(Debug, Serialize)]
pub struct UploadedImage {
    pub id: String,
    pub recipe_id: String,
    pub mime_type: String,
    pub width: i64,
    pub height: i64,
    pub bytes: i64,
    pub url: String,
    pub thumb_url: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/recipes/{id}/images", post(upload_image))
        .route("/images/{id}", axum::routing::delete(delete_image))
}

async fn upload_image(
    State(s): State<AppState>,
    Path(recipe_id): Path<String>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<UploadedImage>)> {
    // Confirm the recipe exists FIRST. Saves us reading the entire body
    // only to discard it.
    sqlx::query!("SELECT id FROM recipes WHERE id = ?1", recipe_id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    // Find a field named `file` (or `image`). We accumulate the field
    // bytes manually so we can enforce MAX_UPLOAD_BYTES *while* receiving
    // — not after the buffer has already ballooned. This is the canonical
    // way to bound multipart memory in axum.
    let mut file_bytes: Option<Vec<u8>> = None;
    while let Some(mut field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "image" {
            let mut buf: Vec<u8> = Vec::new();
            while let Some(chunk) = field.chunk().await? {
                if buf.len() + chunk.len() > MAX_UPLOAD_BYTES {
                    return Err(AppError::TooLarge(format!(
                        "image exceeds {} byte limit",
                        MAX_UPLOAD_BYTES
                    )));
                }
                buf.extend_from_slice(&chunk);
            }
            file_bytes = Some(buf);
            break;
        }
    }

    let bytes = file_bytes
        .ok_or_else(|| AppError::Validation("multipart body must contain a `file` field".into()))?;
    if bytes.is_empty() {
        return Err(AppError::Validation("`file` field is empty".into()));
    }

    // SERVER-SIDE mime sniff via `infer` (not the client's Content-Type
    // header, which is forgeable). + decode + re-encode (EXIF stripped)
    // + thumbnail.
    let processed = process_image(&bytes)?;

    let image_id = Uuid::new_v4().to_string();
    let (rel_path, rel_thumb_path) =
        write_to_disk(&s.upload_dir, &recipe_id, &image_id, &processed).await?;

    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let mime = processed.kind.mime().to_string();
    let original_len = processed.original.len() as i64;
    let width = processed.width as i64;
    let height = processed.height as i64;

    sqlx::query!(
        r#"
        INSERT INTO recipe_images (id, recipe_id, mime_type, width, height, bytes,
                                   path, thumb_path, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        image_id,
        recipe_id,
        mime,
        width,
        height,
        original_len,
        rel_path,
        rel_thumb_path,
        now,
    )
    .execute(&s.pool)
    .await?;

    // First image becomes cover automatically. UX: a freshly-uploaded
    // photo *probably* wants to be visible in the gallery card immediately.
    sqlx::query!(
        r#"
        UPDATE recipes
        SET cover_image_id = COALESCE(cover_image_id, ?1), updated_at = ?2
        WHERE id = ?3
        "#,
        image_id,
        now,
        recipe_id,
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(UploadedImage {
            id: image_id.clone(),
            recipe_id,
            mime_type: processed.kind.mime().to_string(),
            width: processed.width as i64,
            height: processed.height as i64,
            bytes: processed.original.len() as i64,
            url: format!("/uploads/{rel_path}"),
            thumb_url: format!("/uploads/{rel_thumb_path}"),
        }),
    ))
}

async fn delete_image(
    State(s): State<AppState>,
    Path(image_id): Path<String>,
) -> AppResult<StatusCode> {
    let row = sqlx::query!(
        "SELECT path, thumb_path FROM recipe_images WHERE id = ?1",
        image_id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    // FK is ON DELETE SET NULL for recipes.cover_image_id, so deleting
    // the row implicitly clears the cover if it was this one.
    sqlx::query!("DELETE FROM recipe_images WHERE id = ?1", image_id)
        .execute(&s.pool)
        .await?;

    // Best-effort disk cleanup. We swallow errors because the DB is the
    // source of truth — a missing file on disk is harmless.
    let _ = delete_from_disk(&s.upload_dir, &row.path).await;
    let _ = delete_from_disk(&s.upload_dir, &row.thumb_path).await;

    Ok(StatusCode::NO_CONTENT)
}
