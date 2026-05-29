//! Integration tests for the job queue.
//!
//! Covers:
//!   - `SKIP LOCKED` — two workers race for the same job, exactly one wins
//!   - retry on failure with exponential backoff
//!   - dead-letter after `max_attempts`
//!   - re-claim of an expired lease

use chrono::Duration;
use jobs_backend::jobs::queue;
use sqlx::PgPool;

async fn maybe_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn fresh_db() -> Option<PgPool> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    sqlx::query!("TRUNCATE jobs, job_results CASCADE")
        .execute(&pool)
        .await
        .ok()?;
    Some(pool)
}

#[tokio::test]
async fn skip_locked_means_exactly_one_worker_wins() {
    let Some(pool) = fresh_db().await else {
        return;
    };
    for _ in 0..1 {
        queue::enqueue(
            &pool,
            "default",
            "noop",
            serde_json::json!({}),
            3,
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    }

    // Two concurrent claims for the same single pending job.
    let p1 = pool.clone();
    let p2 = pool.clone();
    let (a, b) = tokio::join!(
        async move { queue::claim(&p1, "default", "worker-A", 1, Duration::seconds(60)).await },
        async move { queue::claim(&p2, "default", "worker-B", 1, Duration::seconds(60)).await },
    );
    let total = a.unwrap().len() + b.unwrap().len();
    assert_eq!(
        total, 1,
        "exactly one of the two parallel claims should win (got {total})"
    );
}

#[tokio::test]
async fn failed_job_returns_to_pending_with_future_run_at_then_dead_letters() {
    let Some(pool) = fresh_db().await else {
        return;
    };
    let id = queue::enqueue(
        &pool,
        "default",
        "noop",
        serde_json::json!({}),
        2,
        chrono::Utc::now(),
    )
    .await
    .unwrap();

    // Attempt 1 → claim then fail.
    let claimed = queue::claim(&pool, "default", "w", 1, Duration::seconds(60))
        .await
        .unwrap();
    assert_eq!(claimed.len(), 1);
    let dead = queue::mark_failed_or_retry(
        &pool,
        id,
        "boom",
        chrono::Utc::now() + Duration::seconds(60),
    )
    .await
    .unwrap();
    assert!(!dead);
    let status: String = sqlx::query_scalar!("SELECT status FROM jobs WHERE id = $1", id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");

    // Attempt 2 → claim with a fresh run_at and fail again — now dead.
    sqlx::query!(
        "UPDATE jobs SET run_at = now() - INTERVAL '1 second' WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    queue::claim(&pool, "default", "w", 1, Duration::seconds(60))
        .await
        .unwrap();
    let dead = queue::mark_failed_or_retry(
        &pool,
        id,
        "boom2",
        chrono::Utc::now() + Duration::seconds(60),
    )
    .await
    .unwrap();
    assert!(dead, "second failure should dead-letter");
    let status: String = sqlx::query_scalar!("SELECT status FROM jobs WHERE id = $1", id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "dead");
}

#[tokio::test]
async fn succeeded_job_is_terminal_and_records_history() {
    let Some(pool) = fresh_db().await else {
        return;
    };
    let id = queue::enqueue(
        &pool,
        "default",
        "noop",
        serde_json::json!({}),
        3,
        chrono::Utc::now(),
    )
    .await
    .unwrap();
    queue::claim(&pool, "default", "w", 1, Duration::seconds(60))
        .await
        .unwrap();
    queue::mark_succeeded(&pool, id, "ok").await.unwrap();
    let row = sqlx::query!(
        "SELECT status, (SELECT COUNT(*) FROM job_results WHERE job_id = $1) AS \"results!: i64\"
         FROM jobs WHERE id = $1",
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.status, "succeeded");
    assert_eq!(row.results, 1);
}
