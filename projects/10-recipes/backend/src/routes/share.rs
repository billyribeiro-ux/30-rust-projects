use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::routes::recipes::{Recipe, fetch_images_for};
use crate::signed::SignError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SignedQuery {
    pub sig: String,
    pub exp: i64,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/share/{slug}", get(read_shared))
}

/// Public share view, accessed via signed URL.
/// On bad/expired sig → 404 (NOT 401/403, so the URL doesn't disclose
/// "this recipe exists but you can't see it").
async fn read_shared(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<SignedQuery>,
) -> AppResult<Json<Recipe>> {
    let now_unix = Utc::now().timestamp();
    match s.signer.verify(&slug, q.exp, &q.sig, now_unix) {
        Ok(()) => {}
        Err(SignError::Expired | SignError::Invalid) => {
            // Don't leak which one failed.
            return Err(AppError::NotFound);
        }
    }

    let row = sqlx::query!(
        r#"
        SELECT id, slug, title, description, ingredients_json, instructions_json,
               prep_minutes, cook_minutes, servings, cover_image_id,
               created_at, updated_at
        FROM recipes WHERE slug = ?1
        "#,
        slug,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let id = row.id.expect("id is non-null primary key");
    let images = fetch_images_for(&s.pool, &id).await?;

    Ok(Json(Recipe {
        id,
        slug: row.slug,
        title: row.title,
        description: row.description,
        ingredients: serde_json::from_str(&row.ingredients_json).unwrap_or_default(),
        instructions: serde_json::from_str(&row.instructions_json).unwrap_or_default(),
        prep_minutes: row.prep_minutes,
        cook_minutes: row.cook_minutes,
        servings: row.servings,
        cover_image_id: row.cover_image_id,
        images,
        created_at: parse_ts(&row.created_at),
        updated_at: parse_ts(&row.updated_at),
    }))
}

fn parse_ts(raw: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}
