//! Reviews. Posting requires a session. One review per (place, user) —
//! UPSERT (UPDATE on conflict).

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, response::IntoResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/{place_id}", post(upsert))
}

#[derive(Debug, Deserialize)]
pub struct ReviewInput {
    pub rating: i16,
    pub body: Option<String>,
}

async fn upsert(
    State(s): State<AppState>,
    user: AuthUser,
    Path(place_id): Path<Uuid>,
    Json(input): Json<ReviewInput>,
) -> AppResult<impl IntoResponse> {
    if !(1..=5).contains(&input.rating) {
        return Err(AppError::Fields(vec![FieldError {
            field: "rating".into(),
            message: "must be 1-5".into(),
        }]));
    }
    let body = input.body.unwrap_or_default();
    if body.chars().count() > 2000 {
        return Err(AppError::Fields(vec![FieldError {
            field: "body".into(),
            message: "max 2000 characters".into(),
        }]));
    }

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM places WHERE id = $1)",
        place_id,
    )
    .fetch_one(&s.pool)
    .await?
    .unwrap_or(false);
    if !exists {
        return Err(AppError::NotFound);
    }

    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO reviews (id, place_id, user_id, rating, body)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (place_id, user_id) DO UPDATE
            SET rating = EXCLUDED.rating,
                body   = EXCLUDED.body
        "#,
        id,
        place_id,
        user.id,
        input.rating,
        body,
    )
    .execute(&s.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
