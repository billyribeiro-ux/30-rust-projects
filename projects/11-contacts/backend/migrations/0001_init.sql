-- =========================================================================
-- Project 11 — Contact Manager: schema baseline
-- =========================================================================
-- Postgres 16. This is the schema reused (with additions) through every
-- project from 11 onwards. The auth tables (users, sessions, *_tokens)
-- are the foundation — they don't change.
--
-- Conventions:
-- - UUIDs everywhere (random v4, generated app-side)
-- - timestamps stored as TIMESTAMPTZ (zone-aware), defaulting to now()
-- - email stored in lowercase + UNIQUE (citext would be nicer but stdlib
--   suffices for one-user-per-email)
-- - soft-delete via deleted_at on contacts
-- - tsvector-based FTS with a generated column + GIN index
-- =========================================================================

-- ---------- Auth tables ----------

CREATE TABLE users (
    id                UUID PRIMARY KEY,
    email             TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash     TEXT NOT NULL,
    name              TEXT NOT NULL DEFAULT '',
    email_verified_at TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Server-side sessions: opaque 32-byte tokens (base64url, no padding) stored
-- ONLY hashed on the server. The plaintext is only ever in the user's
-- cookie. Hashing here means a DB leak doesn't immediately let an attacker
-- impersonate users; they'd still need the cookie. We hash with SHA-256
-- (a sha-256 of 32 random bytes has effectively the same security as the
-- token itself; we use it solely for unguessable lookup keys).
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

-- Email verification + password reset use the SAME table.
-- `kind` discriminates intent; tokens are single-use (consumed_at set).
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

CREATE TABLE contacts (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    email       TEXT NOT NULL DEFAULT '',
    phone       TEXT NOT NULL DEFAULT '',
    company     TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT '',
    last_contacted_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at  TIMESTAMPTZ,
    -- Generated tsvector column kept in sync by Postgres automatically.
    -- Concatenates the four searchable fields with appropriate weights
    -- (A = name has highest rank, B = email/company, D = notes lowest).
    search_tsv  TSVECTOR GENERATED ALWAYS AS (
        setweight(to_tsvector('simple', coalesce(name,    '')), 'A') ||
        setweight(to_tsvector('simple', coalesce(email,   '')), 'B') ||
        setweight(to_tsvector('simple', coalesce(company, '')), 'B') ||
        setweight(to_tsvector('simple', coalesce(notes,   '')), 'D')
    ) STORED
);

CREATE INDEX idx_contacts_user_id        ON contacts (user_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_contacts_user_deleted   ON contacts (user_id, deleted_at);
CREATE INDEX idx_contacts_search_tsv     ON contacts USING GIN (search_tsv);
CREATE INDEX idx_contacts_last_contacted ON contacts (user_id, last_contacted_at DESC NULLS LAST)
    WHERE deleted_at IS NULL;

-- Tags: scoped to a user (so two users can have the same tag name without colliding)
CREATE TABLE tags (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, name)
);

CREATE TABLE contact_tags (
    contact_id UUID NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    tag_id     UUID NOT NULL REFERENCES tags(id)     ON DELETE CASCADE,
    PRIMARY KEY (contact_id, tag_id)
);

CREATE INDEX idx_contact_tags_tag ON contact_tags (tag_id);

-- Interactions: one row per recorded touchpoint (call, email, meeting, etc.)
CREATE TABLE interactions (
    id           UUID PRIMARY KEY,
    contact_id   UUID NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    user_id      UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (kind IN ('call', 'email', 'meeting', 'message', 'other')),
    note         TEXT NOT NULL DEFAULT '',
    occurred_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_interactions_contact ON interactions (contact_id, occurred_at DESC);
CREATE INDEX idx_interactions_user    ON interactions (user_id, occurred_at DESC);

-- Reminders: per-user follow-up tasks tied to a contact
CREATE TABLE reminders (
    id           UUID PRIMARY KEY,
    contact_id   UUID NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    user_id      UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    body         TEXT NOT NULL,
    due_at       TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_reminders_user_due ON reminders (user_id, due_at)
    WHERE completed_at IS NULL;

-- Trigger to keep contacts.updated_at fresh on any UPDATE
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_contacts_updated_at
    BEFORE UPDATE ON contacts
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();
