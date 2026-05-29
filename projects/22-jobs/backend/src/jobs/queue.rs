//! Queue primitives — enqueue + claim + mark.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobRow {
    pub id: Uuid,
    pub queue: String,
    pub kind: String,
    pub payload: serde_json::Value,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: DateTime<Utc>,
    pub locked_until: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn enqueue(
    pool: &PgPool,
    queue: &str,
    kind: &str,
    payload: serde_json::Value,
    max_attempts: i32,
    run_at: DateTime<Utc>,
) -> AppResult<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO jobs (id, queue, kind, payload, max_attempts, run_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        id,
        queue,
        kind,
        payload,
        max_attempts,
        run_at
    )
    .execute(pool)
    .await?;
    Ok(id)
}

/// Claim up to N jobs via `SELECT ... FOR UPDATE SKIP LOCKED`. The
/// returned jobs have their `status` set to `running` and a lease
/// window of `lease`.
///
/// SKIP LOCKED is the whole point. Without it, two workers race for
/// the same row and one blocks until the other commits — at scale
/// this serialises the queue. With it, workers grab non-overlapping
/// batches and the throughput is N × concurrency.
pub async fn claim(
    pool: &PgPool,
    queue: &str,
    worker_id: &str,
    n: i64,
    lease: Duration,
) -> AppResult<Vec<JobRow>> {
    let lease_until = Utc::now() + lease;
    let rows = sqlx::query!(
        r#"
        WITH claimed AS (
            SELECT id
            FROM jobs
            WHERE queue = $1
              AND status = 'pending'
              AND run_at <= now()
            ORDER BY run_at
            LIMIT $2
            FOR UPDATE SKIP LOCKED
        )
        UPDATE jobs
        SET status = 'running',
            locked_until = $3,
            locked_by = $4,
            attempts = attempts + 1,
            updated_at = now()
        FROM claimed
        WHERE jobs.id = claimed.id
        RETURNING
            jobs.id,
            jobs.queue,
            jobs.kind,
            jobs.payload,
            jobs.attempts,
            jobs.max_attempts,
            jobs.run_at,
            jobs.locked_until,
            jobs.locked_by,
            jobs.status,
            jobs.last_error,
            jobs.created_at,
            jobs.updated_at
        "#,
        queue,
        n,
        lease_until,
        worker_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| JobRow {
            id: r.id,
            queue: r.queue,
            kind: r.kind,
            payload: r.payload,
            attempts: r.attempts,
            max_attempts: r.max_attempts,
            run_at: r.run_at,
            locked_until: r.locked_until,
            locked_by: r.locked_by,
            status: r.status,
            last_error: r.last_error,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

pub async fn mark_succeeded(pool: &PgPool, id: Uuid, log: &str) -> AppResult<()> {
    sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'succeeded',
            locked_until = NULL,
            locked_by = NULL,
            last_error = NULL,
            updated_at = now()
        WHERE id = $1
        "#,
        id
    )
    .execute(pool)
    .await?;
    record_result(pool, id, true, log).await?;
    Ok(())
}

pub async fn mark_failed_or_retry(
    pool: &PgPool,
    id: Uuid,
    err: &str,
    next_run_at: DateTime<Utc>,
) -> AppResult<bool> {
    let row = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = CASE
                       WHEN attempts >= max_attempts THEN 'dead'
                       ELSE 'pending'
                     END,
            run_at = CASE
                       WHEN attempts >= max_attempts THEN run_at
                       ELSE $2
                     END,
            locked_until = NULL,
            locked_by = NULL,
            last_error = $3,
            updated_at = now()
        WHERE id = $1
        RETURNING status
        "#,
        id,
        next_run_at,
        err
    )
    .fetch_one(pool)
    .await?;
    record_result(pool, id, false, err).await?;
    Ok(row.status == "dead")
}

async fn record_result(pool: &PgPool, id: Uuid, success: bool, log: &str) -> AppResult<()> {
    let attempts: i32 = sqlx::query_scalar!("SELECT attempts FROM jobs WHERE id = $1", id)
        .fetch_one(pool)
        .await?;
    sqlx::query!(
        r#"
        INSERT INTO job_results (job_id, attempt, started_at, finished_at, success, log)
        VALUES ($1, $2, now(), now(), $3, $4)
        "#,
        id,
        attempts,
        success,
        log
    )
    .execute(pool)
    .await?;
    Ok(())
}
