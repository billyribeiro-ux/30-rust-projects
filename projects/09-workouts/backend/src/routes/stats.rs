use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::Serialize;

use crate::error::AppResult;
use crate::prs::{SetInput, detect_prs};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct Stats {
    pub total_sets: i64,
    pub total_volume_minor: i64,
    pub workouts_last_7_days: i64,
    pub pr_count_total: i64,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(read))
}

async fn read(State(s): State<AppState>) -> AppResult<Json<Stats>> {
    let totals = sqlx::query!(
        r#"
        SELECT
            COUNT(*) AS "total_sets!: i64",
            COALESCE(SUM(weight_minor * reps), 0) AS "total_volume_minor!: i64"
        FROM sets
        "#,
    )
    .fetch_one(&s.pool)
    .await?;

    let cutoff = Utc::now() - Duration::days(7);
    let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let last_7 = sqlx::query!(
        r#"SELECT COUNT(*) AS "n!: i64" FROM workouts WHERE performed_at >= ?1"#,
        cutoff_str,
    )
    .fetch_one(&s.pool)
    .await?;

    // PR count across ALL exercises: for each exercise, run detect_prs on
    // its full history and sum the trues. This is O(N) over total sets —
    // for a single-user app with thousands of sets, plenty fast.
    let exercises = sqlx::query!(r#"SELECT id FROM exercises"#)
        .fetch_all(&s.pool)
        .await?;
    let mut pr_count: i64 = 0;
    for ex in exercises {
        let ex_id = ex.id.expect("id is non-null primary key");
        let history = sqlx::query!(
            r#"
            SELECT s.weight_minor, s.reps, w.performed_at
            FROM sets s
            JOIN workouts w ON w.id = s.workout_id
            WHERE s.exercise_id = ?1
            ORDER BY w.performed_at ASC, s.set_order ASC
            "#,
            ex_id,
        )
        .fetch_all(&s.pool)
        .await?;
        let inputs: Vec<SetInput> = history
            .into_iter()
            .map(|r| SetInput {
                weight_minor: r.weight_minor,
                reps: r.reps,
                timestamp: chrono::DateTime::parse_from_rfc3339(&r.performed_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
            .collect();
        let info = detect_prs(&inputs);
        pr_count += info.iter().filter(|p| p.is_pr).count() as i64;
    }

    Ok(Json(Stats {
        total_sets: totals.total_sets,
        total_volume_minor: totals.total_volume_minor,
        workouts_last_7_days: last_7.n,
        pr_count_total: pr_count,
    }))
}
