CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    role          TEXT NOT NULL DEFAULT 'viewer' CHECK (role IN ('viewer','editor')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   BYTEA NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_sessions_expires ON sessions (expires_at);

-- The OLTP events table — high-volume insert. In production this would
-- be partitioned by day and replicated into a columnar store (DuckDB,
-- ClickHouse, BigQuery) for OLAP queries. See LESSON §A1.
CREATE TABLE events (
    id         BIGSERIAL PRIMARY KEY,
    ts         TIMESTAMPTZ NOT NULL DEFAULT now(),
    kind       TEXT NOT NULL,
    payload    JSONB NOT NULL DEFAULT '{}'::jsonb,
    user_id    UUID,
    session_id TEXT
);
CREATE INDEX idx_events_ts ON events (ts DESC);
CREATE INDEX idx_events_kind_ts ON events (kind, ts DESC);
