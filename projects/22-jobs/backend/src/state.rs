use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::jobs::Registry;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub registry: Arc<Registry>,
    /// SSE fanout — `routes/stream.rs` subscribes; workers + admin
    /// mutations publish here so the dashboard sees status changes
    /// near-instantly without polling.
    pub events: broadcast::Sender<JobEvent>,
    pub secure_cookies: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct JobEvent {
    pub job_id: uuid::Uuid,
    pub status: String,
    pub queue: String,
    pub kind: String,
    pub attempts: i32,
}
