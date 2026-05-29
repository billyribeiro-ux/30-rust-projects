//! Tickets — the RLS demo. Every handler opens a tx, calls
//! `enter_tenant`, runs RLS-aware SQL.

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::rls::enter_tenant;
use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{tenant_slug}/tickets", get(list).post(create))
        .route("/{tenant_slug}/tickets/{id}", get(detail))
        .route("/{tenant_slug}/tickets/{id}/messages", post(post_message))
}

#[derive(Debug, Serialize)]
pub struct TicketSummary {
    pub id: Uuid,
    pub subject: String,
    pub status: String,
    pub priority: String,
    pub assignee_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

async fn resolve_tenant(
    pool: &sqlx::PgPool,
    slug: &str,
    user_id: Uuid,
) -> AppResult<(Uuid, String)> {
    let row = sqlx::query!(
        r#"
        SELECT t.id, m.role
        FROM tenants t JOIN memberships m ON m.tenant_id = t.id
        WHERE t.slug = $1 AND m.user_id = $2
        "#,
        slug,
        user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?; // don't leak tenant existence
    Ok((row.id, row.role))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

async fn list(
    State(s): State<AppState>,
    user: AuthUser,
    Path(tenant_slug): Path<String>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<TicketSummary>>> {
    let (tenant_id, role) = resolve_tenant(&s.pool, &tenant_slug, user.id).await?;
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
    let status = q.status.unwrap_or_else(|| "open".into());
    // Customers only see their own; agents/admins see everything in
    // the tenant. We use a single nullable-customer filter so sqlx
    // returns one anonymous type for both branches.
    let customer_filter = if role == "customer" {
        Some(user.id)
    } else {
        None
    };
    let rows = sqlx::query!(
        r#"SELECT id, subject, status, priority, assignee_user_id, created_at, updated_at
           FROM tickets
           WHERE status = $1
             AND ($2::uuid IS NULL OR customer_user_id = $2)
           ORDER BY created_at DESC LIMIT 200"#,
        status,
        customer_filter
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| TicketSummary {
                id: r.id,
                subject: r.subject,
                status: r.status,
                priority: r.priority,
                assignee_user_id: r.assignee_user_id,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateTicketInput {
    pub subject: String,
    pub body: String,
    pub priority: Option<String>,
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Path(tenant_slug): Path<String>,
    Json(input): Json<CreateTicketInput>,
) -> AppResult<impl IntoResponse> {
    let (tenant_id, _role) = resolve_tenant(&s.pool, &tenant_slug, user.id).await?;
    if input.subject.trim().is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "subject".into(),
            message: "subject required".into(),
        }]));
    }
    let priority = input.priority.unwrap_or_else(|| "normal".into());
    if !matches!(priority.as_str(), "low" | "normal" | "high" | "urgent") {
        return Err(AppError::Validation("invalid priority".into()));
    }
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO tickets (id, tenant_id, customer_user_id, subject, priority)
           VALUES ($1, $2, $3, $4, $5)"#,
        id,
        tenant_id,
        user.id,
        input.subject.trim(),
        priority
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        r#"INSERT INTO ticket_messages (tenant_id, ticket_id, author_user_id, body)
           VALUES ($1, $2, $3, $4)"#,
        tenant_id,
        id,
        user.id,
        input.body
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

#[derive(Debug, Serialize)]
pub struct MessageOut {
    pub id: Uuid,
    pub author_user_id: Uuid,
    pub body: String,
    pub internal: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TicketDetail {
    pub ticket: TicketSummary,
    pub messages: Vec<MessageOut>,
}

async fn detail(
    State(s): State<AppState>,
    user: AuthUser,
    Path((tenant_slug, id)): Path<(String, Uuid)>,
) -> AppResult<Json<TicketDetail>> {
    let (tenant_id, role) = resolve_tenant(&s.pool, &tenant_slug, user.id).await?;
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
    let t = sqlx::query!(
        r#"SELECT id, subject, status, priority, customer_user_id, assignee_user_id, created_at, updated_at
           FROM tickets WHERE id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if role == "customer" && t.customer_user_id != user.id {
        return Err(AppError::NotFound);
    }
    let messages = sqlx::query!(
        r#"SELECT id, author_user_id, body, internal, created_at
           FROM ticket_messages
           WHERE ticket_id = $1
           ORDER BY created_at"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(TicketDetail {
        ticket: TicketSummary {
            id: t.id,
            subject: t.subject,
            status: t.status,
            priority: t.priority,
            assignee_user_id: t.assignee_user_id,
            created_at: t.created_at,
            updated_at: t.updated_at,
        },
        messages: messages
            .into_iter()
            .filter(|m| role != "customer" || !m.internal)
            .map(|m| MessageOut {
                id: m.id,
                author_user_id: m.author_user_id,
                body: m.body,
                internal: m.internal,
                created_at: m.created_at,
            })
            .collect(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct PostMessageInput {
    pub body: String,
    pub internal: Option<bool>,
}

async fn post_message(
    State(s): State<AppState>,
    user: AuthUser,
    Path((tenant_slug, id)): Path<(String, Uuid)>,
    Json(input): Json<PostMessageInput>,
) -> AppResult<impl IntoResponse> {
    let (tenant_id, role) = resolve_tenant(&s.pool, &tenant_slug, user.id).await?;
    let internal = input.internal.unwrap_or(false);
    if internal && role == "customer" {
        return Err(AppError::Forbidden);
    }
    let mut tx = s.pool.begin().await?;
    enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
    let msg_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO ticket_messages (id, tenant_id, ticket_id, author_user_id, body, internal)
           VALUES ($1,$2,$3,$4,$5,$6)"#,
        msg_id,
        tenant_id,
        id,
        user.id,
        input.body,
        internal
    )
    .execute(&mut *tx)
    .await?;
    // First-response SLA bookkeeping for agents.
    if role != "customer" {
        sqlx::query!(
            r#"UPDATE tickets
               SET first_response_at = COALESCE(first_response_at, now()),
                   updated_at = now()
               WHERE id = $1"#,
            id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": msg_id })),
    ))
}
