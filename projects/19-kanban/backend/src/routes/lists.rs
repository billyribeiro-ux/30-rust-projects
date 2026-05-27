//! List CRUD + reorder + card-create-on-list.
//!
//! - `PATCH  /api/lists/{id}`         — rename or move (new position)
//! - `DELETE /api/lists/{id}`         — remove (cascades to cards)
//! - `POST   /api/lists/{id}/cards`   — append a card

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{patch, post};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::rbac::{self, Role};
use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

use crate::routes::boards::{CardRow, ListRow};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}", patch(update).delete(remove))
        .route("/{id}/cards", post(create_card))
}

#[derive(Deserialize)]
pub struct UpdateListInput {
    pub name: Option<String>,
    pub position: Option<f64>,
}

async fn update(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateListInput>,
) -> AppResult<Json<ListRow>> {
    let board_id = board_id_for_list(&s, id).await?;
    rbac::require_role(&s.pool, board_id, user.id, Role::Editor).await?;

    if let Some(name) = input.name.as_ref() {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 200 {
            return Err(AppError::Fields(vec![FieldError {
                field: "name".into(),
                message: "1-200 characters required".into(),
            }]));
        }
        sqlx::query!("UPDATE lists SET name = $1 WHERE id = $2", trimmed, id)
            .execute(&s.pool)
            .await?;
    }
    if let Some(pos) = input.position {
        if !pos.is_finite() {
            return Err(AppError::Validation("position must be finite".into()));
        }
        sqlx::query!("UPDATE lists SET position = $1 WHERE id = $2", pos, id)
            .execute(&s.pool)
            .await?;
    }

    let row = sqlx::query_as!(
        ListRow,
        r#"
        SELECT id AS "id!", board_id AS "board_id!", name AS "name!", position AS "position!"
        FROM lists WHERE id = $1
        "#,
        id,
    )
    .fetch_one(&s.pool)
    .await?;
    Ok(Json(row))
}

async fn remove(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let board_id = board_id_for_list(&s, id).await?;
    rbac::require_role(&s.pool, board_id, user.id, Role::Editor).await?;

    let res = sqlx::query!("DELETE FROM lists WHERE id = $1", id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct CreateCardInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub position: Option<f64>,
}

async fn create_card(
    State(s): State<AppState>,
    user: AuthUser,
    Path(list_id): Path<Uuid>,
    Json(input): Json<CreateCardInput>,
) -> AppResult<impl IntoResponse> {
    let board_id = board_id_for_list(&s, list_id).await?;
    rbac::require_role(&s.pool, board_id, user.id, Role::Editor).await?;

    let title = input.title.trim().to_string();
    if title.is_empty() || title.chars().count() > 300 {
        return Err(AppError::Fields(vec![FieldError {
            field: "title".into(),
            message: "1-300 characters required".into(),
        }]));
    }
    let body = input.body;
    if body.chars().count() > 4000 {
        return Err(AppError::Fields(vec![FieldError {
            field: "body".into(),
            message: "4000 characters max".into(),
        }]));
    }

    let position = match input.position {
        Some(p) if p.is_finite() => p,
        _ => {
            let max: Option<f64> =
                sqlx::query_scalar!("SELECT MAX(position) FROM cards WHERE list_id = $1", list_id)
                    .fetch_one(&s.pool)
                    .await?;
            max.unwrap_or(0.0) + 1024.0
        }
    };

    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO cards (id, list_id, title, body, position)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        id,
        list_id,
        title,
        body,
        position,
    )
    .execute(&s.pool)
    .await?;

    let row = sqlx::query_as!(
        CardRow,
        r#"
        SELECT id AS "id!", list_id AS "list_id!", title AS "title!", body AS "body!",
               position AS "position!", due_at,
               created_at AS "created_at!", updated_at AS "updated_at!"
        FROM cards WHERE id = $1
        "#,
        id,
    )
    .fetch_one(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn board_id_for_list(s: &AppState, list_id: Uuid) -> AppResult<Uuid> {
    sqlx::query_scalar!("SELECT board_id FROM lists WHERE id = $1", list_id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)
}
