//! File list / read / delete / download routes.
//!
//! Files reference a `file_versions` row by id; that row holds the blob's
//! sha256, size, and sniffed MIME. Downloads stream the bytes back via
//! the storage backend with the original filename in Content-Disposition.

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, response::Json as JsonResp};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(read).delete(remove))
        .route("/{id}/download", get(download))
}

#[derive(Serialize)]
pub struct FileRow {
    pub id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub size: i64,
    pub mime: String,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub folder_id: Option<Uuid>,
}

async fn list(
    State(s): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<JsonResp<Vec<FileRow>>> {
    // `folder_id IS NOT DISTINCT FROM $2` handles the NULL == NULL case
    // for root files; a plain `=` would never match NULL.
    let rows = sqlx::query_as!(
        FileRow,
        r#"
        SELECT f.id AS "id!", f.folder_id, f.name AS "name!",
               v.size AS "size!", v.mime AS "mime!", v.sha256 AS "sha256!",
               f.created_at AS "created_at!", f.updated_at AS "updated_at!"
        FROM files f
        JOIN file_versions v ON v.id = f.version_id
        WHERE f.owner_id = $1
          AND f.folder_id IS NOT DISTINCT FROM $2
        ORDER BY f.name
        "#,
        user.id,
        q.folder_id,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(JsonResp(rows))
}

async fn read(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<FileRow>> {
    let row = sqlx::query_as!(
        FileRow,
        r#"
        SELECT f.id AS "id!", f.folder_id, f.name AS "name!",
               v.size AS "size!", v.mime AS "mime!", v.sha256 AS "sha256!",
               f.created_at AS "created_at!", f.updated_at AS "updated_at!"
        FROM files f
        JOIN file_versions v ON v.id = f.version_id
        WHERE f.id = $1 AND f.owner_id = $2
        "#,
        id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

async fn remove(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let res = sqlx::query!(
        "DELETE FROM files WHERE id = $1 AND owner_id = $2",
        id,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    // NOTE: we do NOT garbage-collect orphaned file_versions blobs here.
    // A production garbage collector would run `DELETE FROM file_versions
    // WHERE NOT EXISTS (SELECT 1 FROM files WHERE version_id = file_versions.id)`
    // on a schedule, and only THEN delete the storage object. For the demo
    // we keep dedupe immortal — simpler and still correct.
    Ok(StatusCode::NO_CONTENT)
}

async fn download(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    let row = sqlx::query!(
        r#"
        SELECT f.name, v.sha256, v.size, v.mime
        FROM files f
        JOIN file_versions v ON v.id = f.version_id
        WHERE f.id = $1 AND f.owner_id = $2
        "#,
        id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let bytes = s.storage.get(&row.sha256).await?;
    let disposition = format!(
        "attachment; filename=\"{}\"",
        // sanitise quotes; everything else is fine in RFC-6266-ish quoted-string
        row.name.replace('"', "_")
    );
    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_str(&row.mime).unwrap())
        .header(header::CONTENT_LENGTH, row.size)
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(format!("response build: {e}")))?;
    Ok(resp.into_response())
}
