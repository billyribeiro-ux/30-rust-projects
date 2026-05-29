use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub public_url: String,
    pub meili_url: Option<String>,
    pub meili_key: Option<String>,
    pub secure_cookies: bool,
}
