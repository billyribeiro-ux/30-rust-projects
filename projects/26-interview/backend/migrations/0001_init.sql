CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT,
    name          TEXT NOT NULL DEFAULT '',
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

CREATE TABLE interviews (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    interviewer_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    candidate_email TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'scheduled'
                     CHECK (status IN ('scheduled','live','ended')),
    code            TEXT NOT NULL DEFAULT '',
    language        TEXT NOT NULL DEFAULT 'rust',
    started_at      TIMESTAMPTZ,
    ended_at        TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_interviews_interviewer ON interviews (interviewer_id, created_at DESC);

CREATE TABLE executions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    interview_id  UUID NOT NULL REFERENCES interviews(id) ON DELETE CASCADE,
    language      TEXT NOT NULL,
    code          TEXT NOT NULL,
    stdout        TEXT NOT NULL DEFAULT '',
    stderr        TEXT NOT NULL DEFAULT '',
    exit_code     INT,
    duration_ms   INT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_executions_interview ON executions (interview_id, created_at);
