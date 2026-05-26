//! Dashboard KPIs: counts, follow-ups due, recent interactions.
//!
//! All in a single round-trip — the frontend's `load` does one fetch and
//! gets every tile's data. For larger orgs we'd materialize this as a
//! periodically-refreshed view; for one-user-per-tenant it's fine to compute
//! per-request.

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
    pub total_contacts: i64,
    pub deleted_contacts: i64,
    pub stale_contacts_7d: i64, // last_contacted >7 days ago OR null
    pub interactions_this_week: i64,
    pub due_reminders: Vec<Reminder>,
    pub recent_contacts: Vec<RecentContact>,
}

#[derive(Debug, Serialize)]
pub struct Reminder {
    pub id: Uuid,
    pub contact_id: Uuid,
    pub contact_name: String,
    pub body: String,
    pub due_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RecentContact {
    pub id: Uuid,
    pub name: String,
    pub company: String,
    pub last_contacted_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(dashboard))
}

async fn dashboard(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<DashboardData>> {
    let counts = sqlx::query!(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE deleted_at IS NULL)  AS "total!: i64",
            COUNT(*) FILTER (WHERE deleted_at IS NOT NULL) AS "deleted!: i64",
            COUNT(*) FILTER (
                WHERE deleted_at IS NULL
                  AND (last_contacted_at IS NULL OR last_contacted_at < now() - INTERVAL '7 days')
            ) AS "stale!: i64"
        FROM contacts WHERE user_id = $1
        "#,
        user.id,
    )
    .fetch_one(&s.pool)
    .await?;

    let interactions = sqlx::query!(
        r#"
        SELECT COUNT(*) AS "n!: i64"
        FROM interactions
        WHERE user_id = $1 AND occurred_at >= now() - INTERVAL '7 days'
        "#,
        user.id,
    )
    .fetch_one(&s.pool)
    .await?;

    let due = sqlx::query!(
        r#"
        SELECT r.id, r.contact_id, c.name AS contact_name, r.body, r.due_at
        FROM reminders r
        JOIN contacts c ON c.id = r.contact_id
        WHERE r.user_id = $1
          AND r.completed_at IS NULL
          AND r.due_at <= now() + INTERVAL '7 days'
        ORDER BY r.due_at ASC
        LIMIT 10
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    let recent = sqlx::query!(
        r#"
        SELECT id, name, company, last_contacted_at
        FROM contacts
        WHERE user_id = $1 AND deleted_at IS NULL
        ORDER BY last_contacted_at DESC NULLS LAST, updated_at DESC
        LIMIT 10
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(DashboardData {
        total_contacts: counts.total,
        deleted_contacts: counts.deleted,
        stale_contacts_7d: counts.stale,
        interactions_this_week: interactions.n,
        due_reminders: due
            .into_iter()
            .map(|r| Reminder {
                id: r.id,
                contact_id: r.contact_id,
                contact_name: r.contact_name,
                body: r.body,
                due_at: r.due_at,
            })
            .collect(),
        recent_contacts: recent
            .into_iter()
            .map(|r| RecentContact {
                id: r.id,
                name: r.name,
                company: r.company,
                last_contacted_at: r.last_contacted_at,
            })
            .collect(),
    }))
}
