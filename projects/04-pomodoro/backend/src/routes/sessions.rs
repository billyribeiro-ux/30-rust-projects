use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const ALLOWED_KINDS: &[&str] = &["work", "short_break", "long_break"];

#[derive(Debug, Serialize)]
pub struct Session {
    pub id: String,
    pub kind: String,
    pub label: Option<String>,
    pub planned_seconds: i64,
    pub actual_seconds: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSession {
    pub kind: String,
    pub label: Option<String>,
    pub planned_seconds: i64,
    pub actual_seconds: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct Stats {
    pub focus_seconds_today: i64,
    pub focus_seconds_week: i64,
    pub sessions_today: i64,
    pub pomodoros_today: i64,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::delete(delete))
        .route("/stats", get(stats))
}

async fn list(
    State(pool): State<SqlitePool>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<Session>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let rows = sqlx::query!(
        r#"
        SELECT id, kind, label, planned_seconds, actual_seconds, started_at, ended_at
        FROM sessions
        ORDER BY started_at DESC
        LIMIT ?1
        "#,
        limit,
    )
    .fetch_all(&pool)
    .await?;

    let sessions = rows
        .into_iter()
        .map(|r| Session {
            id: r.id.expect("id is non-null primary key"),
            kind: r.kind,
            label: r.label,
            planned_seconds: r.planned_seconds,
            actual_seconds: r.actual_seconds,
            started_at: parse_ts(&r.started_at),
            ended_at: parse_ts(&r.ended_at),
        })
        .collect();

    Ok(Json(sessions))
}

async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateSession>,
) -> AppResult<(StatusCode, Json<Session>)> {
    let kind = normalize_kind(&payload.kind)?;
    let label = payload.label.as_deref().map(normalize_label).transpose()?;
    validate_seconds(payload.planned_seconds, "planned_seconds")?;
    validate_seconds(payload.actual_seconds, "actual_seconds")?;
    if payload.ended_at < payload.started_at {
        return Err(AppError::Validation(
            "ended_at must be >= started_at".into(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let started_str = format_ts(payload.started_at);
    let ended_str = format_ts(payload.ended_at);

    sqlx::query!(
        r#"
        INSERT INTO sessions (id, kind, label, planned_seconds, actual_seconds, started_at, ended_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        id,
        kind,
        label,
        payload.planned_seconds,
        payload.actual_seconds,
        started_str,
        ended_str,
    )
    .execute(&pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Session {
            id,
            kind,
            label,
            planned_seconds: payload.planned_seconds,
            actual_seconds: payload.actual_seconds,
            started_at: payload.started_at,
            ended_at: payload.ended_at,
        }),
    ))
}

async fn delete(State(pool): State<SqlitePool>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM sessions WHERE id = ?1", id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn stats(State(pool): State<SqlitePool>) -> AppResult<Json<Stats>> {
    // Today is computed in the user's local timezone client-side normally, but
    // since we have no auth/user, we use server-side UTC. Frontend can decide
    // to compute its own with the full session list if needed.
    let today_start = today_utc_iso();
    let week_start = week_ago_utc_iso();

    let today_focus = sqlx::query!(
        r#"
        SELECT COALESCE(SUM(actual_seconds), 0) AS "focus!: i64"
        FROM sessions
        WHERE kind = 'work' AND started_at >= ?1
        "#,
        today_start,
    )
    .fetch_one(&pool)
    .await?;

    let week_focus = sqlx::query!(
        r#"
        SELECT COALESCE(SUM(actual_seconds), 0) AS "focus!: i64"
        FROM sessions
        WHERE kind = 'work' AND started_at >= ?1
        "#,
        week_start,
    )
    .fetch_one(&pool)
    .await?;

    let today_counts = sqlx::query!(
        r#"
        SELECT
            COUNT(*) AS "all!: i64",
            COALESCE(SUM(CASE WHEN kind = 'work' THEN 1 ELSE 0 END), 0) AS "pomos!: i64"
        FROM sessions
        WHERE started_at >= ?1
        "#,
        today_start,
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(Stats {
        focus_seconds_today: today_focus.focus,
        focus_seconds_week: week_focus.focus,
        sessions_today: today_counts.all,
        pomodoros_today: today_counts.pomos,
    }))
}

fn normalize_kind(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().to_lowercase();
    if !ALLOWED_KINDS.contains(&trimmed.as_str()) {
        return Err(AppError::Validation(format!(
            "kind must be one of: {}",
            ALLOWED_KINDS.join(", ")
        )));
    }
    Ok(trimmed)
}

fn normalize_label(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > 120 {
        return Err(AppError::Validation(
            "label must be 120 chars or fewer".into(),
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_seconds(value: i64, name: &str) -> AppResult<()> {
    if value < 0 {
        return Err(AppError::Validation(format!("{name} must be >= 0")));
    }
    if value > 12 * 60 * 60 {
        return Err(AppError::Validation(format!("{name} must be <= 12 hours")));
    }
    Ok(())
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn today_utc_iso() -> String {
    let now = Utc::now();
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is always valid")
        .and_utc()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

fn week_ago_utc_iso() -> String {
    let ago = Utc::now() - chrono::Duration::days(7);
    ago.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_kind_accepts_work() {
        assert_eq!(normalize_kind("work").unwrap(), "work");
    }

    #[test]
    fn normalize_kind_lowercases() {
        assert_eq!(normalize_kind("WORK").unwrap(), "work");
    }

    #[test]
    fn normalize_kind_rejects_unknown() {
        assert!(matches!(
            normalize_kind("snack"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_label_trims() {
        assert_eq!(normalize_label("  read  ").unwrap(), "read");
    }

    #[test]
    fn normalize_label_rejects_too_long() {
        let long = "a".repeat(121);
        assert!(matches!(
            normalize_label(&long),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn validate_seconds_rejects_negative() {
        assert!(matches!(
            validate_seconds(-1, "x"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn validate_seconds_rejects_too_long() {
        assert!(matches!(
            validate_seconds(12 * 60 * 60 + 1, "x"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn validate_seconds_accepts_zero() {
        assert!(validate_seconds(0, "x").is_ok());
    }
}
