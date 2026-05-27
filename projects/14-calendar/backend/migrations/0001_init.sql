-- =========================================================================
-- Project 14 — Calendar & Scheduler: schema
-- =========================================================================
-- Auth tables (users / sessions / auth_tokens) reuse the project-13 shape:
-- password_hash NOT NULL (no legacy migration anymore), Argon2id-only.
--
-- Domain:
--   calendars       — a named container (color, default timezone, owner).
--   calendar_shares — per-(calendar, user) permission row: 'view' or 'edit'.
--                     Owner permission is implicit on calendars.created_by.
--   events          — single occurrence + optional RRULE for recurrence.
--                     `start_at/end_at` are TIMESTAMPTZ stored UTC.
--                     `tz` is the IANA zone the user authored in (e.g.
--                     'America/Los_Angeles'); we render at that zone.
-- =========================================================================

CREATE TABLE users (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email             TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash     TEXT NOT NULL,
    name              TEXT NOT NULL DEFAULT '',
    email_verified_at TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   BYTEA NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip           TEXT,
    user_agent   TEXT
);
CREATE INDEX idx_sessions_user_id    ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

CREATE TABLE auth_tokens (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('verify_email', 'password_reset')),
    token_hash  BYTEA NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ
);
CREATE INDEX idx_auth_tokens_user_id ON auth_tokens (user_id);

-- ---------- Domain ----------

CREATE TABLE calendars (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    color       TEXT NOT NULL DEFAULT '#3b82f6'
                CHECK (color ~ '^#[0-9a-fA-F]{6}$'),
    default_tz  TEXT NOT NULL DEFAULT 'UTC',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_calendars_owner ON calendars (owner_id);

CREATE TABLE calendar_shares (
    calendar_id UUID NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission  TEXT NOT NULL CHECK (permission IN ('view', 'edit')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (calendar_id, user_id)
);
CREATE INDEX idx_calendar_shares_user ON calendar_shares (user_id);

CREATE TABLE events (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    calendar_id   UUID NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    created_by    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title         TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    description   TEXT NOT NULL DEFAULT '',
    location      TEXT NOT NULL DEFAULT '',
    -- For recurring events: start_at/end_at describe the FIRST occurrence;
    -- subsequent occurrences are computed by applying `rrule` (RFC 5545)
    -- to start_at in `tz`. For one-off events `rrule IS NULL`.
    start_at      TIMESTAMPTZ NOT NULL,
    end_at        TIMESTAMPTZ NOT NULL,
    tz            TEXT NOT NULL DEFAULT 'UTC',
    rrule         TEXT,
    all_day       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (end_at >= start_at)
);
-- Range query: WHERE calendar_id = ANY($1) AND start_at < window_end AND end_at > window_start
CREATE INDEX idx_events_calendar_time ON events (calendar_id, start_at);
CREATE INDEX idx_events_recurring ON events (calendar_id)
    WHERE rrule IS NOT NULL;

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_calendars_updated_at
    BEFORE UPDATE ON calendars
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_events_updated_at
    BEFORE UPDATE ON events
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
