CREATE TABLE IF NOT EXISTS sessions (
    id                 TEXT PRIMARY KEY,
    kind               TEXT NOT NULL CHECK (kind IN ('work', 'short_break', 'long_break')),
    label              TEXT,
    planned_seconds    INTEGER NOT NULL,
    actual_seconds     INTEGER NOT NULL,
    started_at         TEXT NOT NULL,
    ended_at           TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions (started_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_kind ON sessions (kind);
