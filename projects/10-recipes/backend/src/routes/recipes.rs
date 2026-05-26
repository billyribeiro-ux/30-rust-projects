use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct RecipeImage {
    pub id: String,
    pub recipe_id: String,
    pub mime_type: String,
    pub width: i64,
    pub height: i64,
    pub bytes: i64,
    pub url: String,
    pub thumb_url: String,
}

#[derive(Debug, Serialize)]
pub struct RecipeSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub prep_minutes: Option<i64>,
    pub cook_minutes: Option<i64>,
    pub servings: Option<i64>,
    pub cover_thumb_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Recipe {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub ingredients: Vec<String>,
    pub instructions: Vec<String>,
    pub prep_minutes: Option<i64>,
    pub cook_minutes: Option<i64>,
    pub servings: Option<i64>,
    pub cover_image_id: Option<String>,
    pub images: Vec<RecipeImage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecipe {
    pub title: String,
    pub description: Option<String>,
    pub ingredients: Option<Vec<String>>,
    pub instructions: Option<Vec<String>>,
    pub prep_minutes: Option<i64>,
    pub cook_minutes: Option<i64>,
    pub servings: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRecipe {
    pub title: Option<String>,
    pub description: Option<String>,
    pub ingredients: Option<Vec<String>>,
    pub instructions: Option<Vec<String>>,
    pub prep_minutes: Option<i64>,
    pub cook_minutes: Option<i64>,
    pub servings: Option<i64>,
    pub cover_image_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListQuery {
    pub page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ShareLink {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::patch(update).delete(delete))
        .route("/by-slug/{slug}", get(read_by_slug))
        .route("/{id}/share", post(share_link))
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<RecipeSummary>>> {
    // 50 max per page (spec). page indexed from 0.
    let page = q.page.unwrap_or(0).max(0);
    let limit: i64 = 50;
    let offset = page * limit;

    let rows = sqlx::query!(
        r#"
        SELECT r.id, r.slug, r.title, r.description, r.prep_minutes, r.cook_minutes,
               r.servings, r.created_at, r.updated_at,
               img.thumb_path AS "thumb_path?: String"
        FROM recipes r
        LEFT JOIN recipe_images img ON img.id = r.cover_image_id
        ORDER BY r.updated_at DESC
        LIMIT ?1 OFFSET ?2
        "#,
        limit,
        offset,
    )
    .fetch_all(&s.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| RecipeSummary {
                id: r.id.expect("id is non-null primary key"),
                slug: r.slug,
                title: r.title,
                description: r.description,
                prep_minutes: r.prep_minutes,
                cook_minutes: r.cook_minutes,
                servings: r.servings,
                cover_thumb_url: r.thumb_path.map(|p| format!("/uploads/{p}")),
                created_at: parse_ts(&r.created_at),
                updated_at: parse_ts(&r.updated_at),
            })
            .collect(),
    ))
}

async fn read_by_slug(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<Recipe>> {
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

async fn create(
    State(s): State<AppState>,
    Json(payload): Json<CreateRecipe>,
) -> AppResult<(StatusCode, Json<Recipe>)> {
    let title = normalize_title(&payload.title)?;
    let description = normalize_description(payload.description.as_deref().unwrap_or(""))?;
    let ingredients =
        normalize_string_list(payload.ingredients.unwrap_or_default(), "ingredients", 500)?;
    let instructions = normalize_string_list(
        payload.instructions.unwrap_or_default(),
        "instructions",
        2000,
    )?;
    let prep_minutes = validate_minutes(payload.prep_minutes, "prep_minutes")?;
    let cook_minutes = validate_minutes(payload.cook_minutes, "cook_minutes")?;
    let servings = validate_servings(payload.servings)?;

    let id = Uuid::new_v4().to_string();
    let slug = unique_slug(&s.pool, &slugify(&title)).await?;
    let now = Utc::now();
    let now_str = format_ts(now);
    let ingredients_json = serde_json::to_string(&ingredients).expect("json");
    let instructions_json = serde_json::to_string(&instructions).expect("json");

    sqlx::query!(
        r#"
        INSERT INTO recipes (id, slug, title, description, ingredients_json,
                             instructions_json, prep_minutes, cook_minutes, servings,
                             created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
        "#,
        id,
        slug,
        title,
        description,
        ingredients_json,
        instructions_json,
        prep_minutes,
        cook_minutes,
        servings,
        now_str,
    )
    .execute(&s.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Recipe {
            id,
            slug,
            title,
            description,
            ingredients,
            instructions,
            prep_minutes,
            cook_minutes,
            servings,
            cover_image_id: None,
            images: Vec::new(),
            created_at: now,
            updated_at: now,
        }),
    ))
}

async fn update(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateRecipe>,
) -> AppResult<Json<Recipe>> {
    let row = sqlx::query!(
        r#"
        SELECT id, slug, title, description, ingredients_json, instructions_json,
               prep_minutes, cook_minutes, servings, cover_image_id,
               created_at, updated_at
        FROM recipes WHERE id = ?1
        "#,
        id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let new_title = match payload.title.as_deref() {
        Some(t) => normalize_title(t)?,
        None => row.title.clone(),
    };
    let new_description = match payload.description.as_deref() {
        Some(d) => normalize_description(d)?,
        None => row.description.clone(),
    };
    let new_ingredients = match payload.ingredients {
        Some(v) => normalize_string_list(v, "ingredients", 500)?,
        None => serde_json::from_str(&row.ingredients_json).unwrap_or_default(),
    };
    let new_instructions = match payload.instructions {
        Some(v) => normalize_string_list(v, "instructions", 2000)?,
        None => serde_json::from_str(&row.instructions_json).unwrap_or_default(),
    };
    let new_prep = match payload.prep_minutes {
        Some(v) => validate_minutes(Some(v), "prep_minutes")?,
        None => row.prep_minutes,
    };
    let new_cook = match payload.cook_minutes {
        Some(v) => validate_minutes(Some(v), "cook_minutes")?,
        None => row.cook_minutes,
    };
    let new_servings = match payload.servings {
        Some(v) => validate_servings(Some(v))?,
        None => row.servings,
    };

    // cover_image_id may be set, cleared (null), or untouched. We can't
    // distinguish "absent" from "null" from a single Option, so we treat
    // a None payload as "keep". Use PATCH with explicit "" sentinel? No,
    // the simpler convention is: if you pass cover_image_id (any string),
    // it replaces. If you want to clear, DELETE the image (which sets
    // null via FK ON DELETE SET NULL).
    let new_cover_id = payload.cover_image_id.or(row.cover_image_id.clone());

    // Validate the cover belongs to this recipe.
    if let Some(cover_id) = new_cover_id.as_deref() {
        let belongs = sqlx::query!(
            "SELECT id FROM recipe_images WHERE id = ?1 AND recipe_id = ?2",
            cover_id,
            id,
        )
        .fetch_optional(&s.pool)
        .await?;
        if belongs.is_none() {
            return Err(AppError::Validation(
                "cover_image_id must reference an image belonging to this recipe".into(),
            ));
        }
    }

    let now = Utc::now();
    let now_str = format_ts(now);
    let ingredients_json = serde_json::to_string(&new_ingredients).expect("json");
    let instructions_json = serde_json::to_string(&new_instructions).expect("json");

    sqlx::query!(
        r#"
        UPDATE recipes
        SET title = ?1, description = ?2, ingredients_json = ?3, instructions_json = ?4,
            prep_minutes = ?5, cook_minutes = ?6, servings = ?7, cover_image_id = ?8,
            updated_at = ?9
        WHERE id = ?10
        "#,
        new_title,
        new_description,
        ingredients_json,
        instructions_json,
        new_prep,
        new_cook,
        new_servings,
        new_cover_id,
        now_str,
        id,
    )
    .execute(&s.pool)
    .await?;

    let images = fetch_images_for(&s.pool, &id).await?;

    Ok(Json(Recipe {
        id: row.id.expect("id is non-null primary key"),
        slug: row.slug,
        title: new_title,
        description: new_description,
        ingredients: new_ingredients,
        instructions: new_instructions,
        prep_minutes: new_prep,
        cook_minutes: new_cook,
        servings: new_servings,
        cover_image_id: new_cover_id,
        images,
        created_at: parse_ts(&row.created_at),
        updated_at: now,
    }))
}

async fn delete(State(s): State<AppState>, Path(id): Path<String>) -> AppResult<StatusCode> {
    // Best-effort cleanup of disk files before the FK cascade removes
    // the rows.
    let images = sqlx::query!(
        "SELECT path, thumb_path FROM recipe_images WHERE recipe_id = ?1",
        id,
    )
    .fetch_all(&s.pool)
    .await?;
    for img in images {
        let _ = crate::uploads::delete_from_disk(&s.upload_dir, &img.path).await;
        let _ = crate::uploads::delete_from_disk(&s.upload_dir, &img.thumb_path).await;
    }

    let result = sqlx::query!("DELETE FROM recipes WHERE id = ?1", id)
        .execute(&s.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    // The recipe's directory should now be empty — best-effort rmdir.
    let dir = s.upload_dir.join(&id);
    let _ = tokio::fs::remove_dir(&dir).await;

    Ok(StatusCode::NO_CONTENT)
}

async fn share_link(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ShareLink>> {
    let row = sqlx::query!("SELECT slug FROM recipes WHERE id = ?1", id)
        .fetch_optional(&s.pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let exp = Utc::now() + Duration::hours(24);
    let exp_unix = exp.timestamp();
    let sig = s.signer.sign(&row.slug, exp_unix);
    let url = format!("/share/{}?sig={sig}&exp={exp_unix}", row.slug);
    Ok(Json(ShareLink {
        url,
        expires_at: exp,
    }))
}

// ---------------- helpers ----------------

pub async fn fetch_images_for(pool: &SqlitePool, recipe_id: &str) -> AppResult<Vec<RecipeImage>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, recipe_id, mime_type, width, height, bytes, path, thumb_path
        FROM recipe_images
        WHERE recipe_id = ?1
        ORDER BY created_at ASC
        "#,
        recipe_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| RecipeImage {
            id: r.id.expect("id is non-null primary key"),
            recipe_id: r.recipe_id,
            mime_type: r.mime_type,
            width: r.width,
            height: r.height,
            bytes: r.bytes,
            url: format!("/uploads/{}", r.path),
            thumb_url: format!("/uploads/{}", r.thumb_path),
        })
        .collect())
}

pub async fn unique_slug(pool: &SqlitePool, base: &str) -> AppResult<String> {
    // Same pattern as project 02 — try `base`, then `base-1`, `base-2`...
    // up to 1000. This is a classic UNIQUE-on-conflict allocator. We do
    // it with a pre-check rather than insert-and-retry because slugs are
    // chosen BEFORE the INSERT (the caller wants the slug for the
    // response).
    for n in 0u32..1000 {
        let candidate = if n == 0 {
            base.to_string()
        } else {
            format!("{base}-{n}")
        };
        let row = sqlx::query!("SELECT id FROM recipes WHERE slug = ?1", candidate)
            .fetch_optional(pool)
            .await?;
        if row.is_none() {
            return Ok(candidate);
        }
    }
    Err(AppError::Conflict("could not allocate unique slug".into()))
}

pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_dash = true;
    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "recipe".to_string()
    } else {
        trimmed
    }
}

fn normalize_title(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "title".into(),
            message: "must not be empty".into(),
        }]));
    }
    if trimmed.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "title".into(),
            message: "must be 200 chars or fewer".into(),
        }]));
    }
    Ok(trimmed.to_string())
}

fn normalize_description(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > 2000 {
        return Err(AppError::Fields(vec![FieldError {
            field: "description".into(),
            message: "must be 2000 chars or fewer".into(),
        }]));
    }
    Ok(trimmed.to_string())
}

fn normalize_string_list(raw: Vec<String>, field: &str, max_per: usize) -> AppResult<Vec<String>> {
    if raw.len() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: field.into(),
            message: "must contain 200 or fewer items".into(),
        }]));
    }
    let mut out = Vec::with_capacity(raw.len());
    for (i, s) in raw.into_iter().enumerate() {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.chars().count() > max_per {
            return Err(AppError::Fields(vec![FieldError {
                field: format!("{field}[{i}]"),
                message: format!("item must be {max_per} chars or fewer"),
            }]));
        }
        out.push(trimmed);
    }
    Ok(out)
}

fn validate_minutes(raw: Option<i64>, field: &str) -> AppResult<Option<i64>> {
    match raw {
        Some(v) if v < 0 => Err(AppError::Fields(vec![FieldError {
            field: field.into(),
            message: "must be >= 0".into(),
        }])),
        Some(v) if v > 24 * 60 * 7 => Err(AppError::Fields(vec![FieldError {
            field: field.into(),
            message: "must be ≤ 1 week (10080 minutes)".into(),
        }])),
        v => Ok(v),
    }
}

fn validate_servings(raw: Option<i64>) -> AppResult<Option<i64>> {
    match raw {
        Some(v) if v < 1 => Err(AppError::Fields(vec![FieldError {
            field: "servings".into(),
            message: "must be >= 1".into(),
        }])),
        Some(v) if v > 1000 => Err(AppError::Fields(vec![FieldError {
            field: "servings".into(),
            message: "must be ≤ 1000".into(),
        }])),
        v => Ok(v),
    }
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Grandma's Apple Pie!"), "grandma-s-apple-pie");
    }

    #[test]
    fn slugify_fallback_when_empty() {
        assert_eq!(slugify("!!!"), "recipe");
    }

    #[test]
    fn title_must_not_be_empty() {
        assert!(matches!(normalize_title("   "), Err(AppError::Fields(_))));
    }

    #[test]
    fn title_rejects_too_long() {
        let long = "a".repeat(201);
        assert!(matches!(normalize_title(&long), Err(AppError::Fields(_))));
    }

    #[test]
    fn string_list_drops_empties() {
        let r = normalize_string_list(
            vec!["  ".into(), "salt".into(), "".into()],
            "ingredients",
            500,
        )
        .unwrap();
        assert_eq!(r, vec!["salt".to_string()]);
    }

    #[test]
    fn string_list_rejects_too_long_item() {
        let r = normalize_string_list(vec!["a".repeat(501)], "ingredients", 500);
        assert!(matches!(r, Err(AppError::Fields(_))));
    }

    #[test]
    fn minutes_reject_negative() {
        assert!(matches!(
            validate_minutes(Some(-1), "prep_minutes"),
            Err(AppError::Fields(_))
        ));
    }

    #[test]
    fn servings_reject_zero() {
        assert!(matches!(
            validate_servings(Some(0)),
            Err(AppError::Fields(_))
        ));
    }
}
