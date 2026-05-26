use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::error::AppResult;
use crate::openlibrary::BookLookup;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/isbn/{isbn}", get(by_isbn))
}

async fn by_isbn(
    State(s): State<AppState>,
    Path(isbn): Path<String>,
) -> AppResult<Json<BookLookup>> {
    let result = s.openlibrary.lookup_isbn(&isbn).await?;
    Ok(Json(result))
}
