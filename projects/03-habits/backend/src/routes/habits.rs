use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::streaks::{StreakInfo, compute};

const ALLOWED_COLORS: &[&str] = &[
    "#4F46E5", // indigo
    "#059669", // emerald
    "#D97706", // amber
    "#DC2626", // red
    "#0284C7", // sky
    "#7C3AED", // violet
    "#DB2777", // pink
    "#0F766E", // teal
];

#[derive(Debug, Serialize)]
pub struct Habit {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub streak: StreakInfo,
    pub completions: Vec<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct StreakWindow {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub length: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateHabit {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHabit {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleCompletion {
    pub date: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct ToggleResult {
    pub completed: bool,
    pub streak: StreakInfo,
    pub completions: Vec<NaiveDate>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::patch(update).delete(delete))
        .route("/{id}/completions", axum::routing::post(toggle_completion))
        .route("/{id}/streak-windows", get(streak_windows))
}

async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<Habit>>> {
    let habit_rows = sqlx::query!(
        r#"
        SELECT id, name, color, created_at
        FROM habits
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let today = Utc::now().date_naive();
    let mut habits = Vec::with_capacity(habit_rows.len());

    for row in habit_rows {
        let id = row.id.expect("id is non-null primary key");
        let completions = fetch_completions(&pool, &id).await?;
        let streak = compute(&completions, today);
        habits.push(Habit {
            id,
            name: row.name,
            color: row.color,
            created_at: parse_ts(&row.created_at),
            streak,
            completions,
        });
    }

    Ok(Json(habits))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateHabit>,
) -> AppResult<(StatusCode, Json<Habit>)> {
    let name = normalize_name(&payload.name)?;
    let color = normalize_color(&payload.color)?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        "INSERT INTO habits (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
        id,
        name,
        color,
        now_str,
    )
    .execute(&pool)
    .await?;

    let habit = Habit {
        id,
        name,
        color,
        created_at: now,
        streak: StreakInfo {
            current: 0,
            longest: 0,
            total: 0,
            last_completion: None,
        },
        completions: Vec::new(),
    };

    Ok((StatusCode::CREATED, Json(habit)))
}

async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateHabit>,
) -> AppResult<Json<Habit>> {
    let name = payload.name.as_deref().map(normalize_name).transpose()?;
    let color = payload.color.as_deref().map(normalize_color).transpose()?;

    let row = sqlx::query!(
        r#"
        UPDATE habits
        SET name  = COALESCE(?1, name),
            color = COALESCE(?2, color)
        WHERE id = ?3
        RETURNING id, name, color, created_at
        "#,
        name,
        color,
        id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let row_id = row.id.expect("id is non-null primary key");
    let completions = fetch_completions(&pool, &row_id).await?;
    let today = Utc::now().date_naive();
    let streak = compute(&completions, today);

    Ok(Json(Habit {
        id: row_id,
        name: row.name,
        color: row.color,
        created_at: parse_ts(&row.created_at),
        streak,
        completions,
    }))
}

async fn delete(State(pool): State<SqlitePool>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM habits WHERE id = ?1", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_completion(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<ToggleCompletion>,
) -> AppResult<Json<ToggleResult>> {
    let exists = sqlx::query!("SELECT id FROM habits WHERE id = ?1", id)
        .fetch_optional(&pool)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    let date_str = payload.date.format("%Y-%m-%d").to_string();
    let existing = sqlx::query!(
        "SELECT habit_id FROM habit_completions WHERE habit_id = ?1 AND completion_date = ?2",
        id,
        date_str,
    )
    .fetch_optional(&pool)
    .await?;

    let completed = if existing.is_some() {
        sqlx::query!(
            "DELETE FROM habit_completions WHERE habit_id = ?1 AND completion_date = ?2",
            id,
            date_str,
        )
        .execute(&pool)
        .await?;
        false
    } else {
        sqlx::query!(
            "INSERT INTO habit_completions (habit_id, completion_date) VALUES (?1, ?2)",
            id,
            date_str,
        )
        .execute(&pool)
        .await?;
        true
    };

    let completions = fetch_completions(&pool, &id).await?;
    let today = Utc::now().date_naive();
    let streak = compute(&completions, today);

    Ok(Json(ToggleResult {
        completed,
        streak,
        completions,
    }))
}

/// SQL "islands and gaps" — groups consecutive completion dates into streak
/// windows using a window function trick. Each row gets a streak_key equal to
/// (completion_date - row_number_in_order_of_completion_date). Consecutive
/// dates share the same key; gaps reset it. Then GROUP BY key gives streaks.
async fn streak_windows(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<StreakWindow>>> {
    let exists = sqlx::query!("SELECT id FROM habits WHERE id = ?1", id)
        .fetch_optional(&pool)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    let rows = sqlx::query!(
        r#"
        WITH numbered AS (
            SELECT
                completion_date,
                ROW_NUMBER() OVER (ORDER BY completion_date) AS rn
            FROM habit_completions
            WHERE habit_id = ?1
        ),
        grouped AS (
            SELECT
                completion_date,
                DATE(completion_date, '-' || (rn - 1) || ' days') AS streak_key
            FROM numbered
        )
        SELECT
            MIN(completion_date) AS "start!: String",
            MAX(completion_date) AS "end!: String",
            CAST(COUNT(*) AS INTEGER) AS "length!: i64"
        FROM grouped
        GROUP BY streak_key
        ORDER BY MIN(completion_date) DESC
        "#,
        id,
    )
    .fetch_all(&pool)
    .await?;

    let windows = rows
        .into_iter()
        .map(|r| StreakWindow {
            start: parse_date(&r.start),
            end: parse_date(&r.end),
            length: r.length,
        })
        .collect();

    Ok(Json(windows))
}

async fn fetch_completions(pool: &SqlitePool, habit_id: &str) -> AppResult<Vec<NaiveDate>> {
    let rows = sqlx::query!(
        "SELECT completion_date FROM habit_completions WHERE habit_id = ?1 ORDER BY completion_date ASC",
        habit_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| parse_date(&r.completion_date))
        .collect())
}

fn normalize_name(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("name must not be empty".into()));
    }
    if trimmed.chars().count() > 80 {
        return Err(AppError::Validation(
            "name must be 80 chars or fewer".into(),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_color(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().to_uppercase();
    if !ALLOWED_COLORS.contains(&trimmed.as_str()) {
        return Err(AppError::Validation(format!(
            "color must be one of: {}",
            ALLOWED_COLORS.join(", ")
        )));
    }
    Ok(trimmed)
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_date(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").unwrap_or_else(|_| Utc::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_name_rejects_empty() {
        assert!(matches!(
            normalize_name("   "),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_name_trims() {
        assert_eq!(normalize_name("  read  ").unwrap(), "read");
    }

    #[test]
    fn normalize_name_rejects_too_long() {
        let long = "a".repeat(81);
        assert!(matches!(
            normalize_name(&long),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_color_accepts_allowed_uppercase() {
        assert_eq!(normalize_color("#4F46E5").unwrap(), "#4F46E5");
    }

    #[test]
    fn normalize_color_uppercases() {
        assert_eq!(normalize_color("#4f46e5").unwrap(), "#4F46E5");
    }

    #[test]
    fn normalize_color_rejects_unknown() {
        assert!(matches!(
            normalize_color("#123456"),
            Err(AppError::Validation(_))
        ));
    }
}
