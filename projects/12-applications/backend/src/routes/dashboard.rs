//! Dashboard KPIs for the job-application domain.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct DashboardData {
    pub total_applications: i64,
    pub by_status: Vec<StatusCount>,
    pub active_pipeline: i64,
    pub due_this_week: Vec<DueStep>,
    pub recent_events: Vec<RecentEvent>,
}

#[derive(Debug, Serialize)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DueStep {
    pub id: Uuid,
    pub application_id: Uuid,
    pub company: String,
    pub body: String,
    pub due_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RecentEvent {
    pub application_id: Uuid,
    pub company: String,
    pub role: String,
    pub kind: String,
    pub body: String,
    pub new_status: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(dashboard))
}

async fn dashboard(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<DashboardData>> {
    let total = sqlx::query!(
        r#"SELECT COUNT(*) AS "n!: i64" FROM applications WHERE user_id = $1"#,
        user.id,
    )
    .fetch_one(&s.pool)
    .await?;

    let by_status = sqlx::query!(
        r#"
        SELECT status, COUNT(*) AS "count!: i64"
        FROM applications
        WHERE user_id = $1
        GROUP BY status
        ORDER BY status ASC
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    let active = sqlx::query!(
        r#"
        SELECT COUNT(*) AS "n!: i64"
        FROM applications
        WHERE user_id = $1
          AND status NOT IN ('rejected', 'withdrawn', 'accepted')
        "#,
        user.id,
    )
    .fetch_one(&s.pool)
    .await?;

    let due = sqlx::query!(
        r#"
        SELECT ns.id, ns.application_id, a.company, ns.body, ns.due_at
        FROM next_steps ns
        JOIN applications a ON a.id = ns.application_id
        WHERE ns.user_id = $1
          AND ns.completed_at IS NULL
          AND ns.due_at <= now() + INTERVAL '7 days'
        ORDER BY ns.due_at ASC
        LIMIT 20
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    let recent = sqlx::query!(
        r#"
        SELECT e.application_id, a.company, a.role, e.kind, e.body, e.new_status, e.occurred_at
        FROM application_events e
        JOIN applications a ON a.id = e.application_id
        WHERE e.user_id = $1
        ORDER BY e.occurred_at DESC
        LIMIT 10
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(DashboardData {
        total_applications: total.n,
        by_status: by_status
            .into_iter()
            .map(|r| StatusCount {
                status: r.status,
                count: r.count,
            })
            .collect(),
        active_pipeline: active.n,
        due_this_week: due
            .into_iter()
            .map(|r| DueStep {
                id: r.id,
                application_id: r.application_id,
                company: r.company,
                body: r.body,
                due_at: r.due_at,
            })
            .collect(),
        recent_events: recent
            .into_iter()
            .map(|r| RecentEvent {
                application_id: r.application_id,
                company: r.company,
                role: r.role,
                kind: r.kind,
                body: r.body,
                new_status: r.new_status,
                occurred_at: r.occurred_at,
            })
            .collect(),
    }))
}
