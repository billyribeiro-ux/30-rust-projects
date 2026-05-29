use sqlx::PgPool;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub kpis_tx: broadcast::Sender<String>,
    pub secure_cookies: bool,
}
