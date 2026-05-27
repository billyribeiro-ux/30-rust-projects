//! Calendar CRUD + sharing.
//!
//! - `GET    /api/calendars`               — list owned + shared
//! - `POST   /api/calendars`               — create (owner = current user)
//! - `GET    /api/calendars/{id}`          — read (owner OR shared.view+)
//! - `PATCH  /api/calendars/{id}`          — update (owner only)
//! - `DELETE /api/calendars/{id}`          — delete (owner only)
//! - `POST   /api/calendars/{id}/shares`   — share with another user
//! - `DELETE /api/calendars/{id}/shares/{user_id}` — revoke
//!
//! Permission discipline: every read/write joins a CTE that includes both
//! `owner_id = me` and `user_id = me` from `calendar_shares`. The handler
//! never has to write `if owner == me || share.permission == 'edit'` —
//! the SQL does it.

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(read).patch(update).delete(delete))
        .route("/{id}/shares", post(create_share))
        .route(
            "/{id}/shares/{user_id}",
            axum::routing::delete(delete_share),
        )
}

#[derive(Serialize)]
pub struct Calendar {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub color: String,
    pub default_tz: String,
    pub permission: String, // 'owner', 'edit', 'view'
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateInput {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub default_tz: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateInput {
    pub name: Option<String>,
    pub color: Option<String>,
    pub default_tz: Option<String>,
}

#[derive(Deserialize)]
pub struct ShareInput {
    pub user_email: String,
    pub permission: String, // 'view' | 'edit'
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<Calendar>>> {
    let rows = sqlx::query_as!(
        Calendar,
        r#"
        SELECT id AS "id!", owner_id AS "owner_id!", name AS "name!",
               color AS "color!", default_tz AS "default_tz!",
               'owner'::text AS "permission!",
               created_at AS "created_at!", updated_at AS "updated_at!"
        FROM calendars WHERE owner_id = $1
        UNION ALL
        SELECT c.id AS "id!", c.owner_id AS "owner_id!", c.name AS "name!",
               c.color AS "color!", c.default_tz AS "default_tz!",
               cs.permission AS "permission!",
               c.created_at AS "created_at!", c.updated_at AS "updated_at!"
        FROM calendars c
        JOIN calendar_shares cs ON cs.calendar_id = c.id
        WHERE cs.user_id = $1
        ORDER BY "created_at!" ASC
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
    let name = input.name.trim().to_string();
    if name.is_empty() || name.len() > 200 {
        return Err(AppError::Validation("name must be 1..=200 chars".into()));
    }
    let color = input.color.unwrap_or_else(|| "#3b82f6".to_string());
    let default_tz = input.default_tz.unwrap_or_else(|| "UTC".to_string());
    // Validate the IANA zone — chrono-tz panics on Display of unknown zones,
    // so reject up front.
    if default_tz.parse::<chrono_tz::Tz>().is_err() {
        return Err(AppError::Validation(
            "default_tz must be a valid IANA zone (e.g. America/Los_Angeles)".into(),
        ));
    }

    let row = sqlx::query_as!(
        Calendar,
        r#"
        INSERT INTO calendars (owner_id, name, color, default_tz)
        VALUES ($1, $2, $3, $4)
        RETURNING id, owner_id, name, color, default_tz,
                  'owner'::text AS "permission!", created_at, updated_at
        "#,
        user.id,
        name,
        color,
        default_tz,
    )
    .fetch_one(&s.pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("calendars_color_check") => {
            AppError::Validation("color must match #RRGGBB".into())
        }
        _ => AppError::from(e),
    })?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn read(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Calendar>> {
    let row = sqlx::query_as!(
        Calendar,
        r#"
        SELECT c.id, c.owner_id, c.name, c.color, c.default_tz,
               CASE
                 WHEN c.owner_id = $1 THEN 'owner'
                 ELSE cs.permission
               END AS "permission!",
               c.created_at, c.updated_at
        FROM calendars c
        LEFT JOIN calendar_shares cs ON cs.calendar_id = c.id AND cs.user_id = $1
        WHERE c.id = $2 AND (c.owner_id = $1 OR cs.user_id = $1)
        "#,
        user.id,
        id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

async fn update(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateInput>,
) -> AppResult<Json<Calendar>> {
    if let Some(ref tz) = input.default_tz
        && tz.parse::<chrono_tz::Tz>().is_err()
    {
        return Err(AppError::Validation(
            "default_tz must be a valid IANA zone".into(),
        ));
    }
    // Owner-only update. We don't grant edit-shares the ability to rename
    // the calendar — only its owner can.
    let row = sqlx::query_as!(
        Calendar,
        r#"
        UPDATE calendars
        SET name       = COALESCE($3, name),
            color      = COALESCE($4, color),
            default_tz = COALESCE($5, default_tz)
        WHERE id = $1 AND owner_id = $2
        RETURNING id, owner_id, name, color, default_tz,
                  'owner'::text AS "permission!", created_at, updated_at
        "#,
        id,
        user.id,
        input.name,
        input.color,
        input.default_tz,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

async fn delete(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let res = sqlx::query!(
        "DELETE FROM calendars WHERE id = $1 AND owner_id = $2",
        id,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn create_share(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ShareInput>,
) -> AppResult<StatusCode> {
    if !matches!(input.permission.as_str(), "view" | "edit") {
        return Err(AppError::Validation(
            "permission must be 'view' or 'edit'".into(),
        ));
    }
    // Must be the owner.
    let owner = sqlx::query_scalar!(
        "SELECT 1 AS x FROM calendars WHERE id = $1 AND owner_id = $2",
        id,
        user.id,
    )
    .fetch_optional(&s.pool)
    .await?;
    if owner.is_none() {
        return Err(AppError::NotFound);
    }
    // Look up the share-target user by email.
    let target_email = input.user_email.trim().to_lowercase();
    let target = sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", target_email)
        .fetch_optional(&s.pool)
        .await?
        .ok_or_else(|| AppError::Validation("no user with that email".into()))?;
    if target == user.id {
        return Err(AppError::Validation(
            "can't share a calendar with yourself".into(),
        ));
    }
    sqlx::query!(
        r#"INSERT INTO calendar_shares (calendar_id, user_id, permission)
           VALUES ($1, $2, $3)
           ON CONFLICT (calendar_id, user_id)
           DO UPDATE SET permission = EXCLUDED.permission"#,
        id,
        target,
        input.permission,
    )
    .execute(&s.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_share(
    State(s): State<AppState>,
    user: AuthUser,
    Path((id, target)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let res = sqlx::query!(
        r#"DELETE FROM calendar_shares
           WHERE calendar_id = $1 AND user_id = $2
             AND calendar_id IN (SELECT id FROM calendars WHERE owner_id = $3)"#,
        id,
        target,
        user.id,
    )
    .execute(&s.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
