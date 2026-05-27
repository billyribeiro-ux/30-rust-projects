//! Card CRUD + move + comments-on-card.
//!
//! - `PATCH  /api/cards/{id}`        — update title/body/due_at OR move (list_id + position)
//! - `DELETE /api/cards/{id}`        — remove
//! - `GET    /api/cards/{id}/comments` — list comments
//! - `POST   /api/cards/{id}/comments` — create
//! - `DELETE /api/cards/{id}/comments/{cid}` — delete (admin or author)

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, patch};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::rbac::{self, Role};
use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

use crate::routes::boards::CardRow;
use crate::routes::comments::Comment;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}", patch(update).delete(remove))
        .route("/{id}/comments", get(list_comments).post(create_comment))
        .route(
            "/{id}/comments/{cid}",
            axum::routing::delete(delete_comment),
        )
}

#[derive(Deserialize)]
pub struct UpdateCardInput {
    pub title: Option<String>,
    pub body: Option<String>,
    pub list_id: Option<Uuid>,
    pub position: Option<f64>,
    /// Optional explicit `Some(None)` to clear. We model "clear" as a string "null".
    pub due_at: Option<Option<DateTime<Utc>>>,
}

async fn update(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCardInput>,
) -> AppResult<Json<CardRow>> {
    let board_id = board_id_for_card(&s, id).await?;
    rbac::require_role(&s.pool, board_id, user.id, Role::Editor).await?;

    if let Some(target_list) = input.list_id {
        // Moving across lists is only allowed within the same board.
        let target_board: Uuid =
            sqlx::query_scalar!("SELECT board_id FROM lists WHERE id = $1", target_list)
                .fetch_optional(&s.pool)
                .await?
                .ok_or_else(|| {
                    AppError::Fields(vec![FieldError {
                        field: "list_id".into(),
                        message: "list not found".into(),
                    }])
                })?;
        if target_board != board_id {
            return Err(AppError::Forbidden);
        }
        sqlx::query!("UPDATE cards SET list_id = $1 WHERE id = $2", target_list, id)
            .execute(&s.pool)
            .await?;
    }

    if let Some(title) = input.title.as_ref() {
        let trimmed = title.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 300 {
            return Err(AppError::Fields(vec![FieldError {
                field: "title".into(),
                message: "1-300 characters required".into(),
            }]));
        }
        sqlx::query!("UPDATE cards SET title = $1 WHERE id = $2", trimmed, id)
            .execute(&s.pool)
            .await?;
    }

    if let Some(body) = input.body.as_ref() {
        if body.chars().count() > 4000 {
            return Err(AppError::Fields(vec![FieldError {
                field: "body".into(),
                message: "4000 characters max".into(),
            }]));
        }
        sqlx::query!("UPDATE cards SET body = $1 WHERE id = $2", body, id)
            .execute(&s.pool)
            .await?;
    }

    if let Some(pos) = input.position {
        if !pos.is_finite() {
            return Err(AppError::Validation("position must be finite".into()));
        }
        sqlx::query!("UPDATE cards SET position = $1 WHERE id = $2", pos, id)
            .execute(&s.pool)
            .await?;
    }

    if let Some(due) = input.due_at {
        sqlx::query!("UPDATE cards SET due_at = $1 WHERE id = $2", due, id)
            .execute(&s.pool)
            .await?;
    }

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
    Ok(Json(row))
}

async fn remove(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let board_id = board_id_for_card(&s, id).await?;
    rbac::require_role(&s.pool, board_id, user.id, Role::Editor).await?;

    let res = sqlx::query!("DELETE FROM cards WHERE id = $1", id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------- comments inline (so we don't need a separate router with double-board lookups) ----------

async fn list_comments(
    State(s): State<AppState>,
    user: AuthUser,
    Path(card_id): Path<Uuid>,
) -> AppResult<Json<Vec<Comment>>> {
    let board_id = board_id_for_card(&s, card_id).await?;
    rbac::require_role(&s.pool, board_id, user.id, Role::Viewer).await?;

    let rows = sqlx::query_as!(
        Comment,
        r#"
        SELECT c.id AS "id!", c.card_id AS "card_id!", c.author_id AS "author_id!",
               u.email AS "author_email!", u.name AS "author_name!",
               c.body AS "body!", c.created_at AS "created_at!"
        FROM comments c
        JOIN users u ON u.id = c.author_id
        WHERE c.card_id = $1
        ORDER BY c.created_at
        "#,
        card_id,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateCommentInput {
    pub body: String,
}

async fn create_comment(
    State(s): State<AppState>,
    user: AuthUser,
    Path(card_id): Path<Uuid>,
    Json(input): Json<CreateCommentInput>,
) -> AppResult<impl IntoResponse> {
    let board_id = board_id_for_card(&s, card_id).await?;
    rbac::require_role(&s.pool, board_id, user.id, Role::Editor).await?;

    let body = input.body.trim().to_string();
    if body.is_empty() || body.chars().count() > 4000 {
        return Err(AppError::Fields(vec![FieldError {
            field: "body".into(),
            message: "1-4000 characters required".into(),
        }]));
    }

    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO comments (id, card_id, author_id, body)
        VALUES ($1, $2, $3, $4)
        "#,
        id,
        card_id,
        user.id,
        body,
    )
    .execute(&s.pool)
    .await?;

    let row = sqlx::query_as!(
        Comment,
        r#"
        SELECT c.id AS "id!", c.card_id AS "card_id!", c.author_id AS "author_id!",
               u.email AS "author_email!", u.name AS "author_name!",
               c.body AS "body!", c.created_at AS "created_at!"
        FROM comments c
        JOIN users u ON u.id = c.author_id
        WHERE c.id = $1
        "#,
        id,
    )
    .fetch_one(&s.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn delete_comment(
    State(s): State<AppState>,
    user: AuthUser,
    Path((card_id, comment_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let board_id = board_id_for_card(&s, card_id).await?;
    let role = rbac::require_role(&s.pool, board_id, user.id, Role::Editor).await?;

    // Editors can delete their own; admins can delete any.
    let author = sqlx::query_scalar!(
        "SELECT author_id FROM comments WHERE id = $1 AND card_id = $2",
        comment_id,
        card_id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    if author != user.id && !role.is_admin() {
        return Err(AppError::Forbidden);
    }

    sqlx::query!("DELETE FROM comments WHERE id = $1", comment_id)
        .execute(&s.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn board_id_for_card(s: &AppState, card_id: Uuid) -> AppResult<Uuid> {
    sqlx::query_scalar!(
        r#"
        SELECT l.board_id AS "board_id!"
        FROM cards c JOIN lists l ON l.id = c.list_id
        WHERE c.id = $1
        "#,
        card_id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)
}
