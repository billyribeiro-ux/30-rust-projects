use axum::extract::{Path, Query, State};
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
pub struct Exercise {
    pub id: String,
    pub name: String,
    pub muscle_group: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateExercise {
    pub name: String,
    pub muscle_group: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub q: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PrRow {
    pub set_id: String,
    pub workout_id: String,
    pub weight_minor: i64,
    pub reps: i64,
    pub rir: i64,
    pub volume_minor: i64,
    pub performed_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::delete(delete))
        .route("/{id}/prs", get(prs))
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<Exercise>>> {
    let query = q.q.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

    // FTS5 MATCH search vs plain list — branched because sqlx's query!
    // macro needs a concrete SQL string. We escape the query and append
    // `*` so partial token matches work (e.g., "pre" → "press"). We use
    // INNER JOIN on rowid to map FTS hits back to the canonical row.
    let rows = match query {
        None => {
            let r = sqlx::query!(
                r#"
                SELECT id, name, muscle_group, created_at
                FROM exercises
                ORDER BY name COLLATE NOCASE ASC
                "#
            )
            .fetch_all(&s.pool)
            .await?;
            r.into_iter()
                .map(|r| Exercise {
                    id: r.id.expect("id is non-null primary key"),
                    name: r.name,
                    muscle_group: r.muscle_group,
                    created_at: parse_ts(&r.created_at),
                })
                .collect()
        }
        Some(text) => {
            let match_query = build_fts_query(text);
            let r = sqlx::query!(
                r#"
                SELECT e.id, e.name, e.muscle_group, e.created_at
                FROM exercises_fts f
                JOIN exercises e ON e.rowid = f.rowid
                WHERE exercises_fts MATCH ?1
                ORDER BY rank
                LIMIT 50
                "#,
                match_query,
            )
            .fetch_all(&s.pool)
            .await?;
            r.into_iter()
                .map(|r| Exercise {
                    id: r.id.expect("id is non-null primary key"),
                    name: r.name,
                    muscle_group: r.muscle_group,
                    created_at: parse_ts(&r.created_at),
                })
                .collect()
        }
    };

    Ok(Json(rows))
}

async fn create(
    State(s): State<AppState>,
    Json(payload): Json<CreateExercise>,
) -> AppResult<(StatusCode, Json<Exercise>)> {
    let name = normalize_name(&payload.name)?;
    let muscle_group = normalize_muscle(&payload.muscle_group)?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = format_ts(now);

    let result = sqlx::query!(
        r#"
        INSERT INTO exercises (id, name, muscle_group, created_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        id,
        name,
        muscle_group,
        now_str,
    )
    .execute(&s.pool)
    .await;

    match result {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(Exercise {
                id,
                name,
                muscle_group,
                created_at: now,
            }),
        )),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(AppError::Conflict(
            "an exercise with that name already exists".into(),
        )),
        Err(other) => Err(AppError::Database(other)),
    }
}

async fn delete(State(s): State<AppState>, Path(id): Path<String>) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM exercises WHERE id = ?1", id)
        .execute(&s.pool)
        .await;
    match result {
        Ok(r) if r.rows_affected() == 0 => Err(AppError::NotFound),
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => Err(AppError::Conflict(
            "exercise is referenced by existing sets; remove those workouts first".into(),
        )),
        Err(other) => Err(AppError::Database(other)),
    }
}

async fn prs(State(s): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Vec<PrRow>>> {
    // Confirm exercise exists for a clean 404 (vs an empty list).
    let exists = sqlx::query!("SELECT id FROM exercises WHERE id = ?1", id)
        .fetch_optional(&s.pool)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    let rows = sqlx::query!(
        r#"
        SELECT s.id AS set_id, s.workout_id, s.weight_minor, s.reps, s.rir, w.performed_at
        FROM sets s
        JOIN workouts w ON w.id = s.workout_id
        WHERE s.exercise_id = ?1
        ORDER BY w.performed_at ASC, s.set_order ASC
        "#,
        id,
    )
    .fetch_all(&s.pool)
    .await?;

    let inputs: Vec<SetInput> = rows
        .iter()
        .map(|r| SetInput {
            weight_minor: r.weight_minor,
            reps: r.reps,
            timestamp: parse_ts(&r.performed_at),
        })
        .collect();
    let pr_info = detect_prs(&inputs);

    let out: Vec<PrRow> = rows
        .into_iter()
        .zip(pr_info.into_iter())
        .filter(|(_, p)| p.is_pr)
        .map(|(r, p)| PrRow {
            set_id: r.set_id.expect("id is non-null primary key"),
            workout_id: r.workout_id,
            weight_minor: r.weight_minor,
            reps: r.reps,
            rir: r.rir,
            volume_minor: p.volume_minor,
            performed_at: parse_ts(&r.performed_at),
        })
        .collect();

    Ok(Json(out))
}

// ---------------- helpers ----------------

fn build_fts_query(raw: &str) -> String {
    // Sanitize: strip FTS5 special chars so user input can't break out of
    // the query (no need for advanced syntax in autocomplete). Each token
    // gets a trailing `*` so partial-prefix matching works ("pre" → "press").
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect();
    let tokens: Vec<String> = cleaned
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("{t}*"))
        .collect();
    if tokens.is_empty() {
        // FTS5 MATCH with empty query is an error — return a no-op token.
        "zzznoresultsxyz*".to_string()
    } else {
        tokens.join(" ")
    }
}

fn normalize_name(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "must not be empty".into(),
        }]));
    }
    if trimmed.chars().count() > 80 {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "must be 80 chars or fewer".into(),
        }]));
    }
    Ok(trimmed.to_string())
}

fn normalize_muscle(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().to_lowercase();
    match trimmed.as_str() {
        "chest" | "back" | "legs" | "shoulders" | "arms" | "core" | "other" => Ok(trimmed),
        _ => Err(AppError::Fields(vec![FieldError {
            field: "muscle_group".into(),
            message: "must be one of chest, back, legs, shoulders, arms, core, other".into(),
        }])),
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
    fn name_must_not_be_empty() {
        assert!(matches!(normalize_name(""), Err(AppError::Fields(_))));
    }

    #[test]
    fn muscle_whitelist() {
        assert_eq!(normalize_muscle("Legs").unwrap(), "legs");
        assert!(matches!(
            normalize_muscle("invalid"),
            Err(AppError::Fields(_))
        ));
    }

    #[test]
    fn fts_query_escapes_specials() {
        // Sanitization replaces FTS5 specials with spaces.
        assert_eq!(build_fts_query("bench-press"), "bench* press*");
        assert_eq!(build_fts_query("over\"head\""), "over* head*");
    }

    #[test]
    fn fts_query_empty_is_noop() {
        assert!(build_fts_query("").contains("noresult"));
        assert!(build_fts_query("   ").contains("noresult"));
    }

    #[test]
    fn fts_query_appends_prefix_star() {
        assert_eq!(build_fts_query("press"), "press*");
    }
}
