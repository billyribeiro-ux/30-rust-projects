use sqlx::PgPool;
use std::sync::Arc;
use webauthn_rs::Webauthn;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub webauthn: Arc<Webauthn>,
    pub free_tier_calls: i64,
    pub per_call_cents: f64,
    pub secure_cookies: bool,
}
