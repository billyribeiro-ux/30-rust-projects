//! Room CRUD + membership.
//!
//! - GET    /api/rooms          → rooms the current user is a member of
//! - POST   /api/rooms          → create + auto-join
//! - GET    /api/rooms/{slug}   → room metadata (must be a member)
//! - POST   /api/rooms/{slug}/join → join an existing room
//! - DELETE /api/rooms/{slug}/leave → leave

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{slug}", get(read))
        .route("/{slug}/join", post(join))
        .route("/{slug}/leave", delete(leave))
}

#[derive(Serialize)]
pub struct Room {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct CreateInput {
    pub slug: String,
    pub name: String,
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<Room>>> {
    let rows = sqlx::query_as!(
        Room,
        r#"
        SELECT r.id, r.slug, r.name, r.created_by, r.created_at
        FROM rooms r
        JOIN room_members m ON m.room_id = r.id
        WHERE m.user_id = $1
        ORDER BY r.created_at DESC
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    let slug = input.slug.trim().to_lowercase();
    let name = input.name.trim().to_string();
    if name.is_empty() || name.len() > 200 {
        return Err(AppError::Validation("name must be 1..=200 chars".into()));
    }
    // The CHECK constraint on the slug enforces format; we surface a clean
    // 400 instead of the raw constraint-violation 500.
    if !slug
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
        || slug.len() > 64
        || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(AppError::Validation(
            "slug must match [a-z0-9][a-z0-9-]{0,63}".into(),
        ));
    }

    let mut tx = s.pool.begin().await?;
    let room = sqlx::query_as!(
        Room,
        r#"
        INSERT INTO rooms (slug, name, created_by)
        VALUES ($1, $2, $3)
        RETURNING id, slug, name, created_by, created_at
        "#,
        slug,
        name,
        user.id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("rooms_slug_key") => {
            AppError::Validation("slug already taken".into())
        }
        _ => AppError::from(e),
    })?;

    sqlx::query!(
        "INSERT INTO room_members (room_id, user_id) VALUES ($1, $2)",
        room.id,
        user.id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(room)))
}

async fn read(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Room>> {
    let row = sqlx::query_as!(
        Room,
        r#"
        SELECT r.id, r.slug, r.name, r.created_by, r.created_at
        FROM rooms r
        JOIN room_members m ON m.room_id = r.id
        WHERE r.slug = $1 AND m.user_id = $2
        "#,
        slug,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

async fn join(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Room>> {
    let room = sqlx::query_as!(
        Room,
        "SELECT id, slug, name, created_by, created_at FROM rooms WHERE slug = $1",
        slug,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    sqlx::query!(
        "INSERT INTO room_members (room_id, user_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
        room.id,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    Ok(Json(room))
}

async fn leave(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<StatusCode> {
    let res = sqlx::query!(
        r#"DELETE FROM room_members
           WHERE user_id = $1
             AND room_id = (SELECT id FROM rooms WHERE slug = $2)"#,
        user.id,
        slug,
    )
    .execute(&s.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
