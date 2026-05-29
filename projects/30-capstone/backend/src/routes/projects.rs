//! Projects + tasks under a tenant. RLS-bounded.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
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
    Router::new()
        .route("/{tenant_slug}/projects", get(list).post(create))
        .route(
            "/{tenant_slug}/projects/{project_id}/tasks",
            get(tasks).post(create_task),
        )
}

async fn resolve_tenant(
    pool: &sqlx::PgPool,
    slug: &str,
    user_id: Uuid,
) -> AppResult<(Uuid, String)> {
    let row = sqlx::query!(
        r#"SELECT t.id, m.role
           FROM tenants t JOIN memberships m ON m.tenant_id = t.id
           WHERE t.slug = $1 AND m.user_id = $2"#,
        slug,
        user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok((row.id, row.role))
}

#[derive(Debug, Serialize)]
pub struct ProjectOut {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

async fn list(
    State(s): State<AppState>,
    user: AuthUser,
    Path(tenant_slug): Path<String>,
) -> AppResult<Json<Vec<ProjectOut>>> {
    let (tenant_id, _role) = resolve_tenant(&s.pool, &tenant_slug, user.id).await?;
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
    let rows = sqlx::query!(
        "SELECT id, slug, name, created_at FROM projects WHERE archived_at IS NULL
         ORDER BY created_at DESC"
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ProjectOut {
                id: r.id,
                slug: r.slug,
                name: r.name,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectInput {
    pub slug: String,
    pub name: String,
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Path(tenant_slug): Path<String>,
    Json(i): Json<CreateProjectInput>,
) -> AppResult<impl IntoResponse> {
    let (tenant_id, role) = resolve_tenant(&s.pool, &tenant_slug, user.id).await?;
    if role == "viewer" {
        return Err(AppError::Forbidden);
    }
    if i.name.trim().is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "name required".into(),
        }]));
    }
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO projects (id, tenant_id, slug, name) VALUES ($1,$2,$3,$4)"#,
        id,
        tenant_id,
        i.slug,
        i.name
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("projects_tenant_id_slug_key") => {
            AppError::Conflict("slug in use".into())
        }
        _ => e.into(),
    })?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

#[derive(Debug, Serialize)]
pub struct TaskOut {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub status: String,
    pub assignee_id: Option<Uuid>,
    pub position: f64,
    pub due_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

async fn tasks(
    State(s): State<AppState>,
    user: AuthUser,
    Path((tenant_slug, project_id)): Path<(String, Uuid)>,
) -> AppResult<Json<Vec<TaskOut>>> {
    let (tenant_id, _role) = resolve_tenant(&s.pool, &tenant_slug, user.id).await?;
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
    let rows = sqlx::query!(
        "SELECT id, title, body, status, assignee_id, position, due_at, created_at, updated_at
         FROM tasks WHERE project_id = $1
         ORDER BY status, position",
        project_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| TaskOut {
                id: r.id,
                title: r.title,
                body: r.body,
                status: r.status,
                assignee_id: r.assignee_id,
                position: r.position,
                due_at: r.due_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub title: String,
    pub body: Option<String>,
}

async fn create_task(
    State(s): State<AppState>,
    user: AuthUser,
    Path((tenant_slug, project_id)): Path<(String, Uuid)>,
    Json(i): Json<CreateTaskInput>,
) -> AppResult<impl IntoResponse> {
    let (tenant_id, role) = resolve_tenant(&s.pool, &tenant_slug, user.id).await?;
    if role == "viewer" {
        return Err(AppError::Forbidden);
    }
    if i.title.trim().is_empty() {
        return Err(AppError::Validation("title required".into()));
    }
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO tasks (id, tenant_id, project_id, title, body)
           VALUES ($1, $2, $3, $4, $5)"#,
        id,
        tenant_id,
        project_id,
        i.title,
        i.body.unwrap_or_default()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}
