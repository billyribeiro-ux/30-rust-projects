use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub public_url: String,
    pub smtp_url: String,
    pub smtp_from: String,
    pub secure_cookies: bool,
}
