use sqlx::PgPool;
use std::sync::Arc;

use crate::stripe::client::StripeClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub stripe: Arc<StripeClient>,
    pub stripe_webhook_secret: String,
    pub stripe_price_id: String,
    pub public_url: String,
    pub smtp_url: String,
    pub smtp_from: String,
    pub secure_cookies: bool,
}
