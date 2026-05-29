use sqlx::PgPool;
use std::sync::Arc;

use crate::oauth::OAuthRegistry;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub oauth: Arc<OAuthRegistry>,
    pub secure_cookies: bool,
    /// Where the backend redirects the browser after a successful OAuth
    /// callback. Must point at the SvelteKit origin so the new session
    /// cookie (set for the backend's domain) is available on the next
    /// same-origin call.
    pub oauth_success_redirect: String,
    pub oauth_failure_redirect: String,
}
