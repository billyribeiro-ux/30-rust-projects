use sqlx::PgPool;
use std::sync::Arc;

use crate::stripe::client::StripeClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub stripe: Arc<StripeClient>,
    pub stripe_webhook_secret: String,
    pub platform_fee_bps: i64,
    pub public_url: String,
    pub download_signing_key: String,
    pub secure_cookies: bool,
}
