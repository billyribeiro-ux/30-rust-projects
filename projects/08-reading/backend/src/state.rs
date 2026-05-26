use sqlx::SqlitePool;

use crate::openlibrary::OpenLibrary;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub openlibrary: OpenLibrary,
}
