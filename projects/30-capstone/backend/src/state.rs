use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub public_url: String,
    pub stripe_webhook_secret: String,
    pub secure_cookies: bool,
}
