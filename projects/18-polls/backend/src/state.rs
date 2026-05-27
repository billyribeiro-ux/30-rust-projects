use sqlx::PgPool;
use std::sync::Arc;

use crate::email::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub mailer: Arc<Mailer>,
    pub public_url: String,
    /// DATABASE_URL kept so per-connection LISTEN sockets (PgListener) can be
    /// opened on demand — see routes::polls::stream.
    pub db_url: String,
    pub secure_cookies: bool,
}
