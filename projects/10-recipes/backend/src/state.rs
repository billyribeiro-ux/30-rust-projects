use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

use crate::signed::Signer;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub upload_dir: Arc<PathBuf>,
    pub signer: Signer,
}
