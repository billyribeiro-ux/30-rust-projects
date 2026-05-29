//! The worker loop. Runs `concurrency` parallel claim-and-run tasks.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::jobs::Registry;
use crate::jobs::backoff::Backoff;
use crate::jobs::queue::{self, JobRow};
use crate::state::JobEvent;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub concurrency: usize,
    pub lease_secs: i64,
    pub poll_idle_ms: u64,
    pub batch_size: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            concurrency: 4,
            lease_secs: 60,
            poll_idle_ms: 1000,
            batch_size: 8,
        }
    }
}

pub async fn run_worker(
    pool: PgPool,
    registry: Arc<Registry>,
    events: broadcast::Sender<JobEvent>,
    queue: String,
    cfg: Config,
) {
    let worker_id = format!("{}/{}", hostname(), Uuid::new_v4());
    tracing::info!(%worker_id, %queue, "worker starting");

    let sem = Arc::new(Semaphore::new(cfg.concurrency));

    loop {
        let permits_available = sem.available_permits();
        if permits_available == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            continue;
        }
        let claimed = match queue::claim(
            &pool,
            &queue,
            &worker_id,
            cfg.batch_size.min(permits_available as i64),
            Duration::seconds(cfg.lease_secs),
        )
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(?e, "queue claim failed");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
        };
        if claimed.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(cfg.poll_idle_ms)).await;
            continue;
        }
        for job in claimed {
            let permit = sem
                .clone()
                .acquire_owned()
                .await
                .expect("semaphore never closes");
            let pool = pool.clone();
            let registry = registry.clone();
            let events = events.clone();
            tokio::spawn(async move {
                run_one(pool, registry, events, job).await;
                drop(permit);
            });
        }
    }
}

#[tracing::instrument(level = "info", skip_all, fields(job.id = %job.id, job.kind = %job.kind, job.attempt = job.attempts))]
async fn run_one(
    pool: PgPool,
    registry: Arc<Registry>,
    events: broadcast::Sender<JobEvent>,
    job: JobRow,
) {
    let _ = events.send(JobEvent {
        job_id: job.id,
        status: "running".into(),
        queue: job.queue.clone(),
        kind: job.kind.clone(),
        attempts: job.attempts,
    });

    let handler = match registry.get(&job.kind) {
        Some(h) => h,
        None => {
            let msg = format!("no handler registered for kind '{}'", job.kind);
            tracing::error!(%msg);
            let _ = queue::mark_failed_or_retry(
                &pool,
                job.id,
                &msg,
                Utc::now() + Duration::seconds(60),
            )
            .await;
            return;
        }
    };

    let result = handler(job.payload.clone()).await;
    match result {
        Ok(log) => {
            if let Err(e) = queue::mark_succeeded(&pool, job.id, &log).await {
                tracing::error!(?e, "mark_succeeded failed");
            }
            let _ = events.send(JobEvent {
                job_id: job.id,
                status: "succeeded".into(),
                queue: job.queue.clone(),
                kind: job.kind.clone(),
                attempts: job.attempts,
            });
        }
        Err(e) => {
            let bf = Backoff::default();
            let next = Utc::now() + Duration::seconds(bf.delay_secs(job.attempts as u32) as i64);
            let err = format!("{e}");
            let dead = queue::mark_failed_or_retry(&pool, job.id, &err, next)
                .await
                .unwrap_or(false);
            let status = if dead { "dead" } else { "pending" };
            let _ = events.send(JobEvent {
                job_id: job.id,
                status: status.into(),
                queue: job.queue.clone(),
                kind: job.kind.clone(),
                attempts: job.attempts,
            });
        }
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "local".into())
}
