use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::rls::enter_tenant;
use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list).post(create))
}

#[derive(Debug, Serialize)]
pub struct TenantOut {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub role: String,
    pub plan: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<TenantOut>>> {
    let rows = sqlx::query!(
        r#"SELECT t.id, t.slug, t.name, m.role, t.created_at,
                  COALESCE(s.plan, 'free') AS "plan!",
                  COALESCE(s.status, 'active') AS "status!"
           FROM memberships m
           JOIN tenants t ON t.id = m.tenant_id
           LEFT JOIN subscriptions s ON s.tenant_id = t.id
           WHERE m.user_id = $1
           ORDER BY t.created_at DESC"#,
        user.id
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| TenantOut {
                id: r.id,
                slug: r.slug,
                name: r.name,
                role: r.role,
                plan: r.plan,
                status: r.status,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateInput {
    pub slug: String,
    pub name: String,
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(i): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    let slug = i.slug.trim().to_lowercase();
    let name = i.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "name required".into(),
        }]));
    }
    let mut tx = s.pool.begin().await?;
    let id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO tenants (id, slug, name) VALUES ($1,$2,$3)",
        id,
        slug,
        name
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("tenants_slug_key") => {
            AppError::Conflict("slug already in use".into())
        }
        _ => e.into(),
    })?;
    enter_tenant(&mut tx, id, Some(user.id)).await?;
    sqlx::query!(
        "INSERT INTO memberships (tenant_id, user_id, role) VALUES ($1,$2,'owner')",
        id,
        user.id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("INSERT INTO subscriptions (tenant_id) VALUES ($1)", id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(TenantOut {
            id,
            slug,
            name,
            role: "owner".into(),
            plan: "free".into(),
            status: "active".into(),
            created_at: Utc::now(),
        }),
    ))
}
