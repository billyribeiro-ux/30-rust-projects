CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT, -- NULL = passkey-only account
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

CREATE TABLE webauthn_credentials (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   BYTEA NOT NULL UNIQUE,
    credential      JSONB NOT NULL,
    nickname        TEXT NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ
);

-- Short-lived registration / authentication state. Cleaned by a job.
CREATE TABLE webauthn_states (
    id          UUID PRIMARY KEY,
    user_id     UUID,
    state       JSONB NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('register','authenticate')),
    expires_at  TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_webauthn_states_expires ON webauthn_states (expires_at);

CREATE TABLE api_keys (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    prefix       TEXT NOT NULL,        -- first 8 chars, shown in UI
    hash         BYTEA NOT NULL UNIQUE, -- SHA-256 of the full secret
    scopes       TEXT[] NOT NULL DEFAULT '{}',
    rpm_limit    INT NOT NULL DEFAULT 60,
    revoked_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX idx_api_keys_user ON api_keys (user_id, created_at DESC);

CREATE TABLE usage_events (
    id            BIGSERIAL PRIMARY KEY,
    api_key_id    UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    ts            TIMESTAMPTZ NOT NULL DEFAULT now(),
    units         INT NOT NULL DEFAULT 1,
    reported_to_stripe_at TIMESTAMPTZ
);
CREATE INDEX idx_usage_unreported ON usage_events (reported_to_stripe_at)
    WHERE reported_to_stripe_at IS NULL;
CREATE INDEX idx_usage_api_key_ts ON usage_events (api_key_id, ts DESC);
