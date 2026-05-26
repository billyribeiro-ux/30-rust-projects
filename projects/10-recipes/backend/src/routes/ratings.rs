use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct Rating {
    pub id: String,
    pub recipe_id: String,
    pub stars: i64,
    pub comment: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRating {
    pub stars: i64,
    pub comment: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/recipes/{id}/ratings",
        post(create_rating).get(list_ratings),
    )
}

async fn create_rating(
    State(s): State<AppState>,
    Path(recipe_id): Path<String>,
    Json(payload): Json<CreateRating>,
) -> AppResult<(StatusCode, Json<Rating>)> {
    sqlx::query!("SELECT id FROM recipes WHERE id = ?1", recipe_id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    if !(1..=5).contains(&payload.stars) {
        return Err(AppError::Fields(vec![FieldError {
            field: "stars".into(),
            message: "must be between 1 and 5".into(),
        }]));
    }
    let comment = payload.comment.unwrap_or_default();
    let comment = comment.trim().to_string();
    if comment.chars().count() > 1000 {
        return Err(AppError::Fields(vec![FieldError {
            field: "comment".into(),
            message: "must be 1000 chars or fewer".into(),
        }]));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    sqlx::query!(
        r#"
        INSERT INTO ratings (id, recipe_id, stars, comment, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        id,
        recipe_id,
        payload.stars,
        comment,
        now_str,
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Rating {
            id,
            recipe_id,
            stars: payload.stars,
            comment,
            created_at: now,
        }),
    ))
}

async fn list_ratings(
    State(s): State<AppState>,
    Path(recipe_id): Path<String>,
) -> AppResult<Json<Vec<Rating>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, recipe_id, stars, comment, created_at
        FROM ratings
        WHERE recipe_id = ?1
        ORDER BY created_at DESC
        "#,
        recipe_id,
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| Rating {
                id: r.id.expect("id is non-null primary key"),
                recipe_id: r.recipe_id,
                stars: r.stars,
                comment: r.comment,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
            .collect(),
    ))
}
