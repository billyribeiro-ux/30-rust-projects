//! Friend recommendations — minimal surface so the schema isn't dead code.
//!
//! `friend_recs(user_id, friend_id, place_id)` lets a user mark "Alice
//! recommends Place X" without forcing a full social graph. The list
//! endpoint returns the places your friends have flagged.

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list).post(create))
}

#[derive(Debug, Serialize)]
pub struct RecRow {
    pub friend_id: Uuid,
    pub friend_email: String,
    pub place_id: Uuid,
    pub place_name: String,
    pub created_at: DateTime<Utc>,
}

async fn list(State(s): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<RecRow>>> {
    let rows = sqlx::query!(
        r#"
        SELECT fr.friend_id, u.email AS friend_email,
               fr.place_id, p.name AS place_name, fr.created_at
        FROM friend_recs fr
        JOIN users  u ON u.id = fr.friend_id
        JOIN places p ON p.id = fr.place_id
        WHERE fr.user_id = $1
        ORDER BY fr.created_at DESC
        LIMIT 200
        "#,
        user.id
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| RecRow {
                friend_id: r.friend_id,
                friend_email: r.friend_email,
                place_id: r.place_id,
                place_name: r.place_name,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateRecInput {
    pub friend_id: Uuid,
    pub place_id: Uuid,
}

async fn create(
    State(s): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateRecInput>,
) -> AppResult<impl IntoResponse> {
    if input.friend_id == user.id {
        return Err(AppError::Fields(vec![FieldError {
            field: "friend_id".into(),
            message: "cannot recommend to yourself".into(),
        }]));
    }
    sqlx::query!(
        r#"
        INSERT INTO friend_recs (user_id, friend_id, place_id)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, friend_id, place_id) DO NOTHING
        "#,
        user.id,
        input.friend_id,
        input.place_id,
    )
    .execute(&s.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
