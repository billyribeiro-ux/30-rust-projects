use sqlx::PgPool;
use std::sync::Arc;

use crate::email::Mailer;

/// `redis` is `Option` to enforce graceful degradation: if REDIS_URL is unset
/// (or unreachable at startup), the app still runs — the redirect path falls
/// back to "just write the click row". The Redis counter is a cache, not the
/// source of truth.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: Option<deadpool_redis::Pool>,
    pub mailer: Arc<Mailer>,
    pub public_url: String,
    pub secure_cookies: bool,
}
