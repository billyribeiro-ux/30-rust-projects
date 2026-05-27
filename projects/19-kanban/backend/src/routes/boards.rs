//! Board CRUD + membership management + list-of-lists endpoints.
//!
//! - `GET    /api/boards`                       — boards the caller owns or is a member of
//! - `POST   /api/boards`                       — create
//! - `GET    /api/boards/{slug}`                — full board view (board + lists + cards)
//! - `PATCH  /api/boards/{id}`                  — rename (editor+)
//! - `DELETE /api/boards/{id}`                  — admin only
//! - `GET    /api/boards/{id}/memberships`      — list members
//! - `POST   /api/boards/{id}/memberships`      — add/upsert member  (admin)
//! - `DELETE /api/boards/{id}/memberships/{u}`  — remove member       (admin)
//! - `POST   /api/boards/{id}/lists`            — create list (editor+)

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get};
use axum::{Json, response::Json as JsonResp};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::rbac::{self, Role};
use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{slug}", get(read_by_slug))
        .route("/{id}/rename", axum::routing::patch(rename))
        .route("/{id}", axum::routing::delete(remove))
        .route(
            "/{id}/memberships",
            get(list_members).post(upsert_member),
        )
        .route(
            "/{id}/memberships/{user_id}",
            delete(remove_member),
        )
        .route("/{id}/lists", axum::routing::post(create_list))
}

// ---------- Board ----------

#[derive(Serialize)]
pub struct BoardSummary {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ListRow {
    pub id: Uuid,
    pub board_id: Uuid,
    pub name: String,
    pub position: f64,
}

#[derive(Serialize)]
pub struct CardRow {
    pub id: Uuid,
    pub list_id: Uuid,
    pub title: String,
    pub body: String,
    pub position: f64,
    pub due_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct BoardFull {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub slug: String,
    pub role: Role,
    pub lists: Vec<ListRow>,
    pub cards: Vec<CardRow>,
}

#[derive(Deserialize)]
pub struct CreateBoardInput {
    pub name: String,
    pub slug: Option<String>,
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<JsonResp<Vec<BoardSummary>>> {
    // Owned boards + boards where the caller is a member.
    // We compute role inline: owner → 'admin', else the membership role.
    let rows = sqlx::query!(
        r#"
        SELECT b.id, b.owner_id, b.name, b.slug, b.created_at, b.updated_at,
               CASE WHEN b.owner_id = $1 THEN 'admin' ELSE m.role END AS "role!"
        FROM boards b
        LEFT JOIN memberships m
          ON m.board_id = b.id AND m.user_id = $1
        WHERE b.owner_id = $1 OR m.user_id = $1
        ORDER BY b.created_at DESC
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    let out = rows
        .into_iter()
        .map(|r| BoardSummary {
            id: r.id,
            owner_id: r.owner_id,
            name: r.name,
            slug: r.slug,
            role: Role::parse(&r.role).unwrap_or(Role::Viewer),
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect();
    Ok(JsonResp(out))
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateBoardInput>,
) -> AppResult<impl IntoResponse> {
    let name = input.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "1-200 characters required".into(),
        }]));
    }
    let slug = match input.slug {
        Some(s) => normalize_slug(&s)?,
        None => slugify(&name),
    };
    if slug.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "slug".into(),
            message: "must contain at least one a-z, 0-9, or - character".into(),
        }]));
    }

    let id = Uuid::new_v4();
    let result = sqlx::query!(
        "INSERT INTO boards (id, owner_id, name, slug) VALUES ($1, $2, $3, $4)",
        id,
        user.id,
        name,
        slug,
    )
    .execute(&s.pool)
    .await;

    if let Err(sqlx::Error::Database(e)) = &result
        && e.constraint() == Some("boards_slug_key")
    {
        return Err(AppError::Conflict("slug already taken".into()));
    }
    result?;

    let row = sqlx::query!(
        r#"SELECT id, owner_id, name, slug, created_at, updated_at FROM boards WHERE id = $1"#,
        id,
    )
    .fetch_one(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(BoardSummary {
            id: row.id,
            owner_id: row.owner_id,
            name: row.name,
            slug: row.slug,
            role: Role::Admin,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }),
    ))
}

async fn read_by_slug(
    State(s): State<AppState>,
    user: AuthUser,
    Path(slug): Path<String>,
) -> AppResult<Json<BoardFull>> {
    let slug = normalize_slug(&slug)?;
    let board = sqlx::query!(
        r#"SELECT id, owner_id, name, slug FROM boards WHERE slug = $1"#,
        slug,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let role = rbac::lookup_role(&s.pool, board.id, user.id)
        .await?
        .ok_or(AppError::NotFound)?;

    let lists = sqlx::query_as!(
        ListRow,
        r#"
        SELECT id AS "id!", board_id AS "board_id!", name AS "name!", position AS "position!"
        FROM lists WHERE board_id = $1
        ORDER BY position, id
        "#,
        board.id,
    )
    .fetch_all(&s.pool)
    .await?;

    let cards = sqlx::query_as!(
        CardRow,
        r#"
        SELECT c.id AS "id!", c.list_id AS "list_id!", c.title AS "title!",
               c.body AS "body!", c.position AS "position!", c.due_at,
               c.created_at AS "created_at!", c.updated_at AS "updated_at!"
        FROM cards c
        JOIN lists l ON l.id = c.list_id
        WHERE l.board_id = $1
        ORDER BY c.position, c.id
        "#,
        board.id,
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(BoardFull {
        id: board.id,
        owner_id: board.owner_id,
        name: board.name,
        slug: board.slug,
        role,
        lists,
        cards,
    }))
}

#[derive(Deserialize)]
pub struct RenameInput {
    pub name: String,
}

async fn rename(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<RenameInput>,
) -> AppResult<StatusCode> {
    rbac::require_role(&s.pool, id, user.id, Role::Editor).await?;
    let name = input.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "1-200 characters required".into(),
        }]));
    }
    sqlx::query!("UPDATE boards SET name = $1 WHERE id = $2", name, id)
        .execute(&s.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    rbac::require_role(&s.pool, id, user.id, Role::Admin).await?;
    let res = sqlx::query!("DELETE FROM boards WHERE id = $1", id)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Memberships ----------

#[derive(Serialize)]
pub struct MembershipRow {
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
    pub role: Role,
}

async fn list_members(
    State(s): State<AppState>,
    user: AuthUser,
    Path(board_id): Path<Uuid>,
) -> AppResult<Json<Vec<MembershipRow>>> {
    rbac::require_role(&s.pool, board_id, user.id, Role::Viewer).await?;
    // Include the implicit owner-admin row up front, then explicit memberships.
    let rows = sqlx::query!(
        r#"
        SELECT u.id AS "user_id!", u.email, u.name,
               CASE WHEN u.id = b.owner_id THEN 'admin' ELSE m.role END AS "role!"
        FROM boards b
        LEFT JOIN memberships m ON m.board_id = b.id
        JOIN users u ON u.id = COALESCE(m.user_id, b.owner_id)
        WHERE b.id = $1
        ORDER BY (u.id = b.owner_id) DESC, u.email
        "#,
        board_id,
    )
    .fetch_all(&s.pool)
    .await?;
    let out = rows
        .into_iter()
        .map(|r| MembershipRow {
            user_id: r.user_id,
            email: r.email,
            name: r.name,
            role: Role::parse(&r.role).unwrap_or(Role::Viewer),
        })
        .collect();
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct AddMemberInput {
    /// Email of the user to invite. Must already exist (no invite-flow in scope).
    pub email: String,
    pub role: Role,
}

async fn upsert_member(
    State(s): State<AppState>,
    user: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(input): Json<AddMemberInput>,
) -> AppResult<impl IntoResponse> {
    rbac::require_role(&s.pool, board_id, user.id, Role::Admin).await?;

    let email = input.email.trim().to_lowercase();
    let target = sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&s.pool)
        .await?
        .ok_or_else(|| {
            AppError::Fields(vec![FieldError {
                field: "email".into(),
                message: "no user with that email".into(),
            }])
        })?;

    // Don't let admins fiddle with the owner row — it's implicit.
    let owner_id = sqlx::query_scalar!("SELECT owner_id FROM boards WHERE id = $1", board_id)
        .fetch_one(&s.pool)
        .await?;
    if target.id == owner_id {
        return Err(AppError::Conflict("the owner is already an admin".into()));
    }

    let role_str = input.role.as_str();
    sqlx::query!(
        r#"
        INSERT INTO memberships (board_id, user_id, role) VALUES ($1, $2, $3)
        ON CONFLICT (board_id, user_id) DO UPDATE SET role = EXCLUDED.role
        "#,
        board_id,
        target.id,
        role_str,
    )
    .execute(&s.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn remove_member(
    State(s): State<AppState>,
    user: AuthUser,
    Path((board_id, user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    rbac::require_role(&s.pool, board_id, user.id, Role::Admin).await?;

    let res = sqlx::query!(
        "DELETE FROM memberships WHERE board_id = $1 AND user_id = $2",
        board_id,
        user_id,
    )
    .execute(&s.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Create list on a board ----------

#[derive(Deserialize)]
pub struct CreateListInput {
    pub name: String,
    /// Optional position; if omitted we append to the end.
    pub position: Option<f64>,
}

async fn create_list(
    State(s): State<AppState>,
    user: AuthUser,
    Path(board_id): Path<Uuid>,
    Json(input): Json<CreateListInput>,
) -> AppResult<impl IntoResponse> {
    rbac::require_role(&s.pool, board_id, user.id, Role::Editor).await?;

    let name = input.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "1-200 characters required".into(),
        }]));
    }

    let position = match input.position {
        Some(p) if p.is_finite() => p,
        _ => {
            let max: Option<f64> =
                sqlx::query_scalar!("SELECT MAX(position) FROM lists WHERE board_id = $1", board_id)
                    .fetch_one(&s.pool)
                    .await?;
            max.unwrap_or(0.0) + 1024.0
        }
    };

    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO lists (id, board_id, name, position) VALUES ($1, $2, $3, $4)",
        id,
        board_id,
        name,
        position,
    )
    .execute(&s.pool)
    .await?;

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
    Ok((StatusCode::CREATED, Json(row)))
}

// ---------- Helpers ----------

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for ch in s.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').chars().take(80).collect()
}

fn normalize_slug(s: &str) -> AppResult<String> {
    let trimmed = s.trim().to_ascii_lowercase();
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
        || trimmed.is_empty()
        || trimmed.len() > 80
    {
        return Err(AppError::Fields(vec![FieldError {
            field: "slug".into(),
            message: "1-80 lowercase letters, digits, or hyphens".into(),
        }]));
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("My Cool Board!"), "my-cool-board");
        assert_eq!(slugify("    "), "");
        assert_eq!(slugify("ABC—123"), "abc-123");
    }

    #[test]
    fn slug_validation_rejects_uppercase_and_spaces() {
        assert!(normalize_slug("Hello World").is_err());
        assert!(normalize_slug("hello-world").is_ok());
    }
}
