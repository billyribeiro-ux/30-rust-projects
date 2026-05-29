use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub sla_first_response_minutes: i32,
    pub sla_resolve_minutes: i32,
    pub created_at: DateTime<Utc>,
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<TenantOut>>> {
    let rows = sqlx::query!(
        r#"
        SELECT t.id, t.slug, t.name, m.role, t.sla_first_response_minutes,
               t.sla_resolve_minutes, t.created_at
        FROM memberships m JOIN tenants t ON t.id = m.tenant_id
        WHERE m.user_id = $1
        ORDER BY t.created_at DESC
        "#,
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
                sla_first_response_minutes: r.sla_first_response_minutes,
                sla_resolve_minutes: r.sla_resolve_minutes,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantInput {
    pub slug: String,
    pub name: String,
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateTenantInput>,
) -> AppResult<impl IntoResponse> {
    let slug = input.slug.trim().to_lowercase();
    let name = input.name.trim().to_string();
    if name.is_empty() || slug.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "name and slug required".into(),
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

    // Insert membership under tenant context — RLS-aware.
    crate::auth::rls::enter_tenant(&mut tx, id, Some(user.id)).await?;
    sqlx::query!(
        "INSERT INTO memberships (tenant_id, user_id, role) VALUES ($1,$2,'admin')",
        id,
        user.id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(TenantOut {
            id,
            slug,
            name,
            role: "admin".into(),
            sla_first_response_minutes: 60,
            sla_resolve_minutes: 1440,
            created_at: chrono::Utc::now(),
        }),
    ))
}
