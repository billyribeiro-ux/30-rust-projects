//! Folder CRUD. Folders form a tree rooted at parent_id = NULL.
//!
//! - `GET    /api/folders`        — list all folders for the user
//! - `POST   /api/folders`        — create
//! - `DELETE /api/folders/{id}`   — delete (cascades to children + files)

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::delete(remove))
}

#[derive(Serialize)]
pub struct Folder {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateInput {
    pub parent_id: Option<Uuid>,
    pub name: String,
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<Folder>>> {
    let rows = sqlx::query_as!(
        Folder,
        r#"
        SELECT id AS "id!", parent_id, name AS "name!",
               created_at AS "created_at!", updated_at AS "updated_at!"
        FROM folders
        WHERE owner_id = $1
        ORDER BY name
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
    if name.is_empty() || name.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "1-200 characters required".into(),
        }]));
    }
    // If parent is set, it must belong to the same owner.
    if let Some(pid) = input.parent_id {
        let owns = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND owner_id = $2)",
            pid,
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
    let result = sqlx::query!(
        r#"
        INSERT INTO folders (id, parent_id, owner_id, name)
        VALUES ($1, $2, $3, $4)
        "#,
        id,
        input.parent_id,
        user.id,
        name,
    )
    .execute(&s.pool)
    .await;

    if let Err(sqlx::Error::Database(e)) = &result
        && e.constraint().is_some()
    {
        return Err(AppError::Conflict(
            "a folder with that name already exists here".into(),
        ));
    }
    result?;

    let row = sqlx::query_as!(
        Folder,
        r#"
        SELECT id AS "id!", parent_id, name AS "name!",
               created_at AS "created_at!", updated_at AS "updated_at!"
        FROM folders WHERE id = $1
        "#,
        id,
    )
    .fetch_one(&s.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn remove(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let res = sqlx::query!(
        "DELETE FROM folders WHERE id = $1 AND owner_id = $2",
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
