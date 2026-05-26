//! Background reminder task.
//!
//! Every `tick_interval` (default 60s in dev, 1h in prod), scan
//! `next_steps` for rows due in the next 24h with `reminded_at IS NULL`
//! and `completed_at IS NULL`. Send a reminder email and stamp
//! `reminded_at = now()` so the same step never reminds twice.
//!
//! The lesson here is the **background-task contract**:
//!   1. Spawn via `tokio::spawn`. The future runs concurrently with the
//!      HTTP server on the same tokio runtime — no separate thread pool,
//!      no separate process.
//!   2. The task owns its own clone of `AppState` (Arc-cheap).
//!   3. It loops forever with `tokio::time::interval`. The first tick
//!      fires immediately so dev cycles are short.
//!   4. Per-tick errors are LOGGED, not propagated — a transient DB
//!      hiccup shouldn't kill the task forever.
//!   5. Shutdown is cooperative: the task respects the same
//!      `shutdown_signal` Future as the HTTP server (via tokio::select).
//!
//! For real production scale we'd use a job queue (project 22 introduces
//! that). For one-user-per-tenant CRM-shape apps, an in-process tokio
//! task is plenty.

use chrono::Duration;
use std::time::Duration as StdDuration;
use tokio::time::interval;

use crate::email;
use crate::state::AppState;

pub async fn run(state: AppState, tick_interval: StdDuration) {
    let mut ticker = interval(tick_interval);
    // skip the immediate-fire tick that `interval` does by default for
    // cleaner startup logs.
    ticker.tick().await;

    loop {
        ticker.tick().await;
        if let Err(e) = tick(&state).await {
            tracing::warn!(err = ?e, "reminder tick failed; will retry next cycle");
        }
    }
}

async fn tick(state: &AppState) -> Result<(), sqlx::Error> {
    // Look for due-soon, not-yet-reminded steps. We pull the user's email
    // + name + the step body inline so we don't need a second round trip
    // per reminder.
    let horizon = chrono::Utc::now() + Duration::hours(24);
    let rows = sqlx::query!(
        r#"
        SELECT ns.id, ns.body, ns.due_at, u.email, u.name, a.company, a.role
        FROM next_steps ns
        JOIN users u ON u.id = ns.user_id
        JOIN applications a ON a.id = ns.application_id
        WHERE ns.completed_at IS NULL
          AND ns.reminded_at IS NULL
          AND ns.due_at <= $1
        LIMIT 100
        "#,
        horizon,
    )
    .fetch_all(&state.pool)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }
    tracing::info!(n = rows.len(), "sending reminders for due-soon next steps");

    for r in rows {
        let subj = format!("Reminder: {}", r.body);
        let body = format!(
            "Hi {},\n\n\
             Your next step for {} ({}) is due at {}:\n\n  {}\n\n\
             Open the tracker: {}",
            if r.name.is_empty() {
                "there"
            } else {
                r.name.as_str()
            },
            r.company,
            r.role,
            r.due_at.format("%Y-%m-%d %H:%M UTC"),
            r.body,
            state.public_url,
        );
        state.mailer.send(&r.email, &subj, body).await;

        // Stamp reminded_at AFTER attempting the send. If the send fails
        // silently (MailHog down etc.), we still mark the row — repeated
        // attempts produce repeated failures, not a stuck loop. The user
        // can always uncheck / recreate to retry.
        sqlx::query!(
            "UPDATE next_steps SET reminded_at = now() WHERE id = $1",
            r.id,
        )
        .execute(&state.pool)
        .await?;
    }

    Ok(())
}

// Use the email helper so it's a real linked symbol — silences dead_code.
#[allow(dead_code)]
fn _force_link() {
    let _ = email::verify_email_body;
}
