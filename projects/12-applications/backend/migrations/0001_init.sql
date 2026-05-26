-- =========================================================================
-- Project 12 — Job Application Tracker: schema
-- =========================================================================
-- Auth tables (users, sessions, auth_tokens) are IDENTICAL to project 11.
-- This is intentional: the auth foundation is the SAME shape across every
-- project from 11 onwards. New projects swap their domain tables; the auth
-- shape stays put.
--
-- New for project 12: the `legacy_bcrypt_hash` column on users. When we
-- "acquire" a company with bcrypt-hashed passwords, we import them into
-- this column. The login path tries Argon2 first (modern); falls back to
-- bcrypt; on a successful bcrypt verify, RE-HASHES the password as Argon2
-- and clears `legacy_bcrypt_hash`. Over time the legacy column drains.
-- =========================================================================

-- ---------- Auth tables (project-11 shape + legacy column) ----------

CREATE TABLE users (
    id                   UUID PRIMARY KEY,
    email                TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash        TEXT,         -- Argon2id PHC; NULL only during legacy migration
    legacy_bcrypt_hash   TEXT,         -- bcrypt $2b$...; cleared after first successful login
    name                 TEXT NOT NULL DEFAULT '',
    email_verified_at    TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- A user MUST have at least one hash.
    CONSTRAINT user_has_some_hash CHECK (
        password_hash IS NOT NULL OR legacy_bcrypt_hash IS NOT NULL
    )
);

CREATE TABLE sessions (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      BYTEA NOT NULL UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip              TEXT,
    user_agent      TEXT
);
CREATE INDEX idx_sessions_user_id    ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

CREATE TABLE auth_tokens (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (kind IN ('verify_email', 'password_reset')),
    token_hash   BYTEA NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ
);
CREATE INDEX idx_auth_tokens_user_id ON auth_tokens (user_id);

-- ---------- Domain tables ----------

CREATE TABLE applications (
    id            UUID PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    company       TEXT NOT NULL,
    role          TEXT NOT NULL,
    location      TEXT NOT NULL DEFAULT '',
    salary_min    INTEGER,
    salary_max    INTEGER,
    job_url       TEXT NOT NULL DEFAULT '',
    notes         TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT 'applied'
                  CHECK (status IN ('wishlist', 'applied', 'screening',
                                    'interview', 'offer', 'accepted',
                                    'rejected', 'withdrawn')),
    applied_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_applications_user_status   ON applications (user_id, status);
CREATE INDEX idx_applications_user_updated  ON applications (user_id, updated_at DESC);

-- Timeline of events
CREATE TABLE application_events (
    id              UUID PRIMARY KEY,
    application_id  UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL CHECK (kind IN ('status_change', 'note',
                                                  'contact_made', 'rejected',
                                                  'offer_received', 'withdrew')),
    body            TEXT NOT NULL DEFAULT '',
    new_status      TEXT,
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_events_application ON application_events (application_id, occurred_at DESC);

CREATE TABLE next_steps (
    id              UUID PRIMARY KEY,
    application_id  UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body            TEXT NOT NULL,
    due_at          TIMESTAMPTZ NOT NULL,
    completed_at    TIMESTAMPTZ,
    -- Set when the background task emails the user. Prevents double-send.
    reminded_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_next_steps_user_due
    ON next_steps (user_id, due_at)
    WHERE completed_at IS NULL;
CREATE INDEX idx_next_steps_pending_reminder
    ON next_steps (due_at)
    WHERE completed_at IS NULL AND reminded_at IS NULL;

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_applications_updated_at
    BEFORE UPDATE ON applications
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
