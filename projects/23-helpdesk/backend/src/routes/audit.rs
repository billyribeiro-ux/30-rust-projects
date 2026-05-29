use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::rls::enter_tenant;
use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/{tenant_slug}/audit", get(list))
}

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub entity_kind: String,
    pub entity_id: Uuid,
    pub created_at: DateTime<Utc>,
}

async fn list(
    State(s): State<AppState>,
    user: AuthUser,
    Path(tenant_slug): Path<String>,
) -> AppResult<Json<Vec<AuditEntry>>> {
    let row = sqlx::query!(
        r#"SELECT t.id, m.role
           FROM tenants t JOIN memberships m ON m.tenant_id = t.id
           WHERE t.slug = $1 AND m.user_id = $2"#,
        tenant_slug,
        user.id
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    if row.role != "admin" {
        return Err(AppError::Forbidden);
    }
    // audit_log isn't RLS-gated, so we filter explicitly by tenant_id.
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, row.id, Some(user.id)).await?;
    let rows = sqlx::query!(
        r#"SELECT id, actor_user_id, action, entity_kind, entity_id, created_at
           FROM audit_log
           WHERE tenant_id = $1
           ORDER BY created_at DESC LIMIT 500"#,
        row.id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| AuditEntry {
                id: r.id,
                actor_user_id: r.actor_user_id,
                action: r.action,
                entity_kind: r.entity_kind,
                entity_id: r.entity_id,
                created_at: r.created_at,
            })
            .collect(),
    ))
}
