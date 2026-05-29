//! KPI computation + live broadcast.
//!
//! Three primitives:
//!   * `compute_snapshot(pool)` — one shot. Returns the current KPI set.
//!   * `spawn_kpi_loop(state, interval_ms)` — background tick that
//!     re-computes and broadcasts via `state.kpis_tx`.
//!   * `GET /api/stream/kpis` — SSE of every broadcast event.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use futures_core::Stream;
use serde::Serialize;
use sqlx::PgPool;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::auth::session::AuthUser;
use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(snapshot))
        .route("/stream", get(stream))
}

#[derive(Debug, Serialize, Clone)]
pub struct KpiSnapshot {
    pub events_last_minute: i64,
    pub events_last_hour: i64,
    pub unique_sessions_last_hour: i64,
    pub top_kinds: Vec<TopKind>,
    pub ts: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TopKind {
    pub kind: String,
    pub count: i64,
}

pub async fn compute_snapshot(pool: &PgPool) -> AppResult<KpiSnapshot> {
    let row = sqlx::query!(
        r#"
        SELECT
            (SELECT COUNT(*)::bigint FROM events WHERE ts > now() - INTERVAL '1 minute') AS "events_last_minute!",
            (SELECT COUNT(*)::bigint FROM events WHERE ts > now() - INTERVAL '1 hour')   AS "events_last_hour!",
            (SELECT COUNT(DISTINCT session_id)::bigint FROM events
                  WHERE ts > now() - INTERVAL '1 hour' AND session_id IS NOT NULL) AS "unique_sessions_last_hour!"
        "#
    )
    .fetch_one(pool)
    .await?;

    let top = sqlx::query!(
        r#"
        SELECT kind, COUNT(*)::bigint AS "count!"
        FROM events
        WHERE ts > now() - INTERVAL '1 hour'
        GROUP BY kind
        ORDER BY COUNT(*) DESC
        LIMIT 5
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| TopKind {
        kind: r.kind,
        count: r.count,
    })
    .collect();

    Ok(KpiSnapshot {
        events_last_minute: row.events_last_minute,
        events_last_hour: row.events_last_hour,
        unique_sessions_last_hour: row.unique_sessions_last_hour,
        top_kinds: top,
        ts: chrono::Utc::now(),
    })
}

async fn snapshot(State(s): State<AppState>, _u: AuthUser) -> AppResult<Json<KpiSnapshot>> {
    Ok(Json(compute_snapshot(&s.pool).await?))
}

async fn stream(
    State(s): State<AppState>,
    _u: AuthUser,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = s.kpis_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|r| match r {
        Ok(json) => Some(Ok(Event::default().event("kpis").data(json))),
        Err(_) => Some(Ok(Event::default().event("lag").data("0"))),
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Background loop — computes a snapshot every `interval` and broadcasts.
pub fn spawn_kpi_loop(state: AppState, interval: Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            match compute_snapshot(&state.pool).await {
                Ok(snap) => {
                    if let Ok(json) = serde_json::to_string(&snap) {
                        let _ = state.kpis_tx.send(json);
                    }
                }
                Err(e) => tracing::warn!(?e, "kpi snapshot failed"),
            }
        }
    });
}
