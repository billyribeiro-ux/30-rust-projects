use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::prs::{SetInput, detect_prs};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct WorkoutSummary {
    pub id: String,
    pub name: String,
    pub performed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub set_count: i64,
    pub total_volume_minor: i64,
}

#[derive(Debug, Serialize)]
pub struct Workout {
    pub id: String,
    pub name: String,
    pub performed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub sets: Vec<WorkoutSet>,
}

#[derive(Debug, Serialize)]
pub struct WorkoutSet {
    pub id: String,
    pub exercise_id: String,
    pub exercise_name: String,
    pub weight_minor: i64,
    pub reps: i64,
    pub rir: i64,
    pub set_order: i64,
    pub volume_minor: i64,
    pub is_pr: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkout {
    pub name: Option<String>,
    pub performed_at: Option<DateTime<Utc>>,
    pub sets: Vec<CreateSet>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSet {
    pub exercise_id: String,
    pub weight_minor: i64,
    pub reps: i64,
    pub rir: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(read).delete(delete))
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<WorkoutSummary>>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            w.id, w.name, w.performed_at, w.created_at,
            COUNT(s.id) AS "set_count!: i64",
            COALESCE(SUM(s.weight_minor * s.reps), 0) AS "total_volume_minor!: i64"
        FROM workouts w
        LEFT JOIN sets s ON s.workout_id = w.id
        GROUP BY w.id
        ORDER BY w.performed_at DESC
        "#
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| WorkoutSummary {
                id: r.id.expect("id is non-null primary key"),
                name: r.name,
                performed_at: parse_ts(&r.performed_at),
                created_at: parse_ts(&r.created_at),
                set_count: r.set_count,
                total_volume_minor: r.total_volume_minor,
            })
            .collect(),
    ))
}

async fn read(State(s): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Workout>> {
    let w = sqlx::query!(
        r#"
        SELECT id, name, performed_at, created_at
        FROM workouts WHERE id = ?1
        "#,
        id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let set_rows = sqlx::query!(
        r#"
        SELECT s.id, s.exercise_id, e.name AS exercise_name,
               s.weight_minor, s.reps, s.rir, s.set_order
        FROM sets s
        JOIN exercises e ON e.id = s.exercise_id
        WHERE s.workout_id = ?1
        ORDER BY s.set_order ASC
        "#,
        id,
    )
    .fetch_all(&s.pool)
    .await?;

    // PR flags are computed PER EXERCISE across THE ENTIRE HISTORY UP TO AND
    // INCLUDING this workout. We fetch all sets for every exercise present
    // in this workout, run the pure detector, then mark just the sets that
    // belong to this workout.
    let workout_performed_at = w.performed_at.clone();
    let exercise_ids: std::collections::HashSet<String> =
        set_rows.iter().map(|r| r.exercise_id.clone()).collect();

    let mut pr_set_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ex_id in &exercise_ids {
        let history = sqlx::query!(
            r#"
            SELECT s.id, s.weight_minor, s.reps, s.workout_id, w.performed_at
            FROM sets s
            JOIN workouts w ON w.id = s.workout_id
            WHERE s.exercise_id = ?1 AND w.performed_at <= ?2
            ORDER BY w.performed_at ASC, s.set_order ASC
            "#,
            ex_id,
            workout_performed_at,
        )
        .fetch_all(&s.pool)
        .await?;

        let inputs: Vec<SetInput> = history
            .iter()
            .map(|r| SetInput {
                weight_minor: r.weight_minor,
                reps: r.reps,
                timestamp: parse_ts(&r.performed_at),
            })
            .collect();
        let prs = detect_prs(&inputs);
        for (h, p) in history.iter().zip(prs.iter()) {
            if p.is_pr && h.workout_id == id {
                pr_set_ids.insert(h.id.clone().expect("id is non-null primary key"));
            }
        }
    }

    let sets = set_rows
        .into_iter()
        .map(|r| {
            let id = r.id.expect("id is non-null primary key");
            let volume = r.weight_minor.saturating_mul(r.reps);
            let is_pr = pr_set_ids.contains(&id);
            WorkoutSet {
                id,
                exercise_id: r.exercise_id,
                exercise_name: r.exercise_name,
                weight_minor: r.weight_minor,
                reps: r.reps,
                rir: r.rir,
                set_order: r.set_order,
                volume_minor: volume,
                is_pr,
            }
        })
        .collect();

    Ok(Json(Workout {
        id: w.id.expect("id is non-null primary key"),
        name: w.name,
        performed_at: parse_ts(&w.performed_at),
        created_at: parse_ts(&w.created_at),
        sets,
    }))
}

async fn create(
    State(s): State<AppState>,
    Json(payload): Json<CreateWorkout>,
) -> AppResult<(StatusCode, Json<Workout>)> {
    let mut errors = Vec::new();
    let name = payload
        .name
        .as_deref()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if name.chars().count() > 200 {
        errors.push(FieldError {
            field: "name".into(),
            message: "must be 200 chars or fewer".into(),
        });
    }
    if payload.sets.is_empty() {
        errors.push(FieldError {
            field: "sets".into(),
            message: "must include at least one set".into(),
        });
    }
    for (i, s) in payload.sets.iter().enumerate() {
        if s.weight_minor <= 0 {
            errors.push(FieldError {
                field: format!("sets[{i}].weight_minor"),
                message: "must be > 0".into(),
            });
        }
        if s.reps <= 0 {
            errors.push(FieldError {
                field: format!("sets[{i}].reps"),
                message: "must be > 0".into(),
            });
        }
        let rir = s.rir.unwrap_or(0);
        if !(0..=10).contains(&rir) {
            errors.push(FieldError {
                field: format!("sets[{i}].rir"),
                message: "must be between 0 and 10".into(),
            });
        }
    }
    if !errors.is_empty() {
        return Err(AppError::Fields(errors));
    }

    let id = Uuid::new_v4().to_string();
    let performed_at = payload.performed_at.unwrap_or_else(Utc::now);
    let performed_str = format_ts(performed_at);
    let now = Utc::now();
    let now_str = format_ts(now);

    let mut tx = s.pool.begin().await?;

    sqlx::query!(
        r#"
        INSERT INTO workouts (id, name, performed_at, created_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        id,
        name,
        performed_str,
        now_str,
    )
    .execute(&mut *tx)
    .await?;

    for (i, set) in payload.sets.iter().enumerate() {
        // Verify the exercise exists; surface a Fields error so the frontend
        // can highlight the offending row.
        let exists = sqlx::query!("SELECT id FROM exercises WHERE id = ?1", set.exercise_id)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Err(AppError::Fields(vec![FieldError {
                field: format!("sets[{i}].exercise_id"),
                message: "exercise not found".into(),
            }]));
        }

        let set_id = Uuid::new_v4().to_string();
        let rir = set.rir.unwrap_or(0);
        let order = i as i64;
        sqlx::query!(
            r#"
            INSERT INTO sets (id, workout_id, exercise_id, weight_minor, reps, rir, set_order)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            set_id,
            id,
            set.exercise_id,
            set.weight_minor,
            set.reps,
            rir,
            order,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Re-read so the returned shape (PR flags, exercise names) is consistent.
    read(State(s), Path(id)).await.map(|json| (StatusCode::CREATED, json))
}

async fn delete(State(s): State<AppState>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM workouts WHERE id = ?1", id)
        .execute(&s.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
