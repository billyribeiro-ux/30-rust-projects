//! Board-level role lookup.
//!
//! Routes use these helpers (or the `BoardRole` extractor) to enforce
//! membership. The board owner is implicitly an admin even without a
//! `memberships` row, so `lookup_role` checks both tables.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Viewer,
    Editor,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Editor => "editor",
            Role::Admin => "admin",
        }
    }

    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "viewer" => Some(Role::Viewer),
            "editor" => Some(Role::Editor),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }

    /// Editors and admins can mutate domain rows.
    pub fn can_write(self) -> bool {
        matches!(self, Role::Editor | Role::Admin)
    }

    /// Only admins can manage memberships and delete the board.
    pub fn is_admin(self) -> bool {
        matches!(self, Role::Admin)
    }
}

/// Look up the effective role of `user_id` on `board_id`.
/// Owner is always admin. Returns Ok(None) if the user has no access.
pub async fn lookup_role(
    pool: &PgPool,
    board_id: Uuid,
    user_id: Uuid,
) -> AppResult<Option<Role>> {
    // Single round-trip: pull owner_id + (optional) membership role.
    let row = sqlx::query!(
        r#"
        SELECT b.owner_id AS "owner_id!",
               m.role     AS "role?"
        FROM boards b
        LEFT JOIN memberships m
          ON m.board_id = b.id AND m.user_id = $2
        WHERE b.id = $1
        "#,
        board_id,
        user_id,
    )
    .fetch_optional(pool)
    .await?;

    let Some(r) = row else { return Ok(None) };
    if r.owner_id == user_id {
        return Ok(Some(Role::Admin));
    }
    Ok(r.role.as_deref().and_then(Role::parse))
}

/// Require at least `minimum` role on `board_id`. NotFound for non-members so
/// we don't leak board existence; Forbidden if they're a member but underprivileged.
pub async fn require_role(
    pool: &PgPool,
    board_id: Uuid,
    user_id: Uuid,
    minimum: Role,
) -> AppResult<Role> {
    let role = lookup_role(pool, board_id, user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if role < minimum {
        return Err(AppError::Forbidden);
    }
    Ok(role)
}
