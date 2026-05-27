-- =========================================================================
-- Project 18 — Polls & Surveys with Live Results: schema
-- =========================================================================
-- Auth tables (users / sessions / auth_tokens) reused from project 14.
--
-- Domain:
--   polls    — owner + question + slug + open/closed flag
--   options  — ordered list of choices per poll
--   votes    — append-only, with voter_key (sha256(slug + ip + cookie))
--              enforced one-per-poll via unique (option_id, voter_key).
--              Because voter_key includes the poll slug as part of the hash
--              input, the same voter has a *different* key for each poll —
--              so a single unique index across votes suffices.
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

CREATE TABLE polls (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    slug        TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-zA-Z0-9_-]{4,32}$'),
    question    TEXT NOT NULL CHECK (length(question) BETWEEN 1 AND 500),
    is_open     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at   TIMESTAMPTZ
);
CREATE INDEX idx_polls_owner ON polls (owner_id);

CREATE TABLE options (
    id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    poll_id   UUID NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    label     TEXT NOT NULL CHECK (length(label) BETWEEN 1 AND 200),
    position  INTEGER NOT NULL
);
CREATE INDEX idx_options_poll ON options (poll_id, position);

CREATE TABLE votes (
    id         BIGSERIAL PRIMARY KEY,
    option_id  UUID NOT NULL REFERENCES options(id) ON DELETE CASCADE,
    voter_key  TEXT NOT NULL,
    voted_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- One vote per (poll, voter_key) — see lesson. voter_key is derived from
-- (slug + ip + cookie) so the same person has a *different* key per poll,
-- which is why a single unique index across all votes is enough.
CREATE UNIQUE INDEX idx_votes_unique_per_poll
    ON votes (option_id, voter_key);
CREATE INDEX idx_votes_option ON votes (option_id);

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
