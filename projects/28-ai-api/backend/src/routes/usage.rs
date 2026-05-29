use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::auth::session::AuthUser;
use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(summary))
}

#[derive(Debug, Serialize)]
pub struct UsageSummary {
    pub calls_this_month: i64,
    pub estimated_cents: f64,
    pub free_tier_calls: i64,
    pub per_call_cents: f64,
    pub last_call_at: Option<DateTime<Utc>>,
}

async fn summary(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<UsageSummary>> {
    let row = sqlx::query!(
        r#"
        SELECT
            COUNT(*) AS "calls!",
            MAX(u.ts) AS last_call_at
        FROM usage_events u
        JOIN api_keys k ON k.id = u.api_key_id
        WHERE k.user_id = $1
          AND u.ts >= date_trunc('month', now())
        "#,
        user.id
    )
    .fetch_one(&s.pool)
    .await?;

    let calls = row.calls;
    let billable = (calls - s.free_tier_calls).max(0);
    let estimated_cents = (billable as f64) * s.per_call_cents;
    Ok(Json(UsageSummary {
        calls_this_month: calls,
        estimated_cents,
        free_tier_calls: s.free_tier_calls,
        per_call_cents: s.per_call_cents,
        last_call_at: row.last_call_at,
    }))
}
