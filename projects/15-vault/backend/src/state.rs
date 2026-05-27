use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;

use crate::storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub storage: Arc<Storage>,
    pub upload_tmp: PathBuf,
    pub secure_cookies: bool,
}
