use sqlx::PgPool;
use std::sync::Arc;

use crate::email::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub mailer: Arc<Mailer>,
    pub public_url: String,
    pub secure_cookies: bool,
}
