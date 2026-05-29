-- =========================================================================
-- Project 22 — Background Jobs Dashboard: schema
-- =========================================================================
-- A single `jobs` table backs the queue. `status` is the state machine:
--   pending  → claimed via `SELECT ... FOR UPDATE SKIP LOCKED` + UPDATE
--   running  → status set, locked_until in the future
--   succeeded
--   failed   → attempts < max_attempts → goes back to pending with new run_at
--   dead     → attempts == max_attempts → no further retries
-- =========================================================================

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   BYTEA NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

CREATE TABLE jobs (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    queue         TEXT NOT NULL DEFAULT 'default',
    kind          TEXT NOT NULL,
    payload       JSONB NOT NULL DEFAULT '{}'::jsonb,
    attempts      INT NOT NULL DEFAULT 0,
    max_attempts  INT NOT NULL DEFAULT 5,
    run_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until  TIMESTAMPTZ,
    locked_by     TEXT,
    status        TEXT NOT NULL DEFAULT 'pending'
                  CHECK (status IN ('pending','running','succeeded','failed','dead')),
    last_error    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The queue's hot path: WHERE status = 'pending' AND run_at <= now()
-- ORDER BY run_at FOR UPDATE SKIP LOCKED.
CREATE INDEX idx_jobs_pending_run_at
    ON jobs (status, run_at)
    WHERE status = 'pending';

-- For listings ordered by recency.
CREATE INDEX idx_jobs_created_at ON jobs (created_at DESC);

-- One row per attempt — useful for the dashboard timeline.
CREATE TABLE job_results (
    id           BIGSERIAL PRIMARY KEY,
    job_id       UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    attempt      INT NOT NULL,
    started_at   TIMESTAMPTZ NOT NULL,
    finished_at  TIMESTAMPTZ NOT NULL,
    success      BOOLEAN NOT NULL,
    log          TEXT NOT NULL DEFAULT ''
);
CREATE INDEX idx_job_results_job ON job_results (job_id, attempt);
