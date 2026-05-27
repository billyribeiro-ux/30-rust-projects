-- =========================================================================
-- Project 17 — URL Shortener + Click Analytics: schema
-- =========================================================================
-- Auth tables (users / sessions / auth_tokens) reuse the project-13/14 shape:
-- Argon2id-only, raw session token in cookie, SHA-256 hash in DB.
--
-- New in project 17:
--   links         — slug -> target URL mapping owned by a user.
--   clicks        — one row per click for analytics aggregation.
--   2FA           — users.totp_secret + users.totp_enabled_at,
--                   backup_codes(user_id, code_hash) for single-use recovery.
-- =========================================================================

CREATE TABLE users (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email             TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash     TEXT NOT NULL,
    name              TEXT NOT NULL DEFAULT '',
    email_verified_at TIMESTAMPTZ,
    -- 2FA: NULL secret = 2FA off. totp_enabled_at is set once the user
    -- verifies their first code (proving the secret is in their authenticator).
    totp_secret       BYTEA,
    totp_enabled_at   TIMESTAMPTZ,
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
    -- 'verify_email' / 'password_reset' (kept for parity).
    -- 'totp_intermediate' is the 5-minute step-up cookie issued after
    -- password-stage success when 2FA is on.
    kind        TEXT NOT NULL CHECK (kind IN ('verify_email', 'password_reset', 'totp_intermediate')),
    token_hash  BYTEA NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ
);
CREATE INDEX idx_auth_tokens_user_id ON auth_tokens (user_id);

-- ---------- Backup codes ----------
-- 10 codes issued at 2FA enable. Each Argon2-hashed at rest (never plaintext).
-- Single-use: `used_at` flips to non-NULL on first successful verify.
CREATE TABLE backup_codes (
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash  TEXT NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, code_hash)
);
CREATE INDEX idx_backup_codes_user ON backup_codes (user_id);

-- ---------- Domain ----------

CREATE TABLE links (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    slug        TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-zA-Z0-9_-]{3,32}$'),
    target_url  TEXT NOT NULL CHECK (length(target_url) BETWEEN 1 AND 2048),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_links_user ON links (user_id, created_at DESC);

-- One row per click. Aggregations via SQL. The hot path can `tokio::spawn`
-- this INSERT and respond to the user immediately; Redis INCR carries the
-- "live counter" that's read by /stats while clicks are still flushing.
CREATE TABLE clicks (
    id          BIGSERIAL PRIMARY KEY,
    link_id     UUID NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    user_agent  TEXT,
    referer     TEXT,
    ip_country  TEXT,  -- nullable; populated if MaxMind lookup succeeds
    is_bot      BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX idx_clicks_link_time ON clicks (link_id, occurred_at DESC);

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
