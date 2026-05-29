-- =========================================================================
-- Project 21 — Newsletter Platform: schema
-- =========================================================================
-- Domain:
--   users         (the author / admin). Password auth, sessions.
--   subscribers   (the readers). NO password — magic links only.
--   magic_links   single-use, hashed token, 10-minute TTL.
--   subscriptions LOCAL mirror of Stripe. The trusted source for "is this
--                 subscriber Pro right now?" Updated by webhook handlers.
--   posts         Markdown body + computed HTML on write. `is_gated`
--                 controls whether body is truncated for non-subscribers.
--   stripe_events idempotency table — INSERT … ON CONFLICT DO NOTHING.
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
    expires_at   TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip           TEXT,
    user_agent   TEXT
);
CREATE INDEX idx_sessions_user_id    ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

CREATE TABLE subscribers (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email        TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE magic_links (
    token_hash   BYTEA PRIMARY KEY,
    subscriber_id UUID NOT NULL REFERENCES subscribers(id) ON DELETE CASCADE,
    expires_at   TIMESTAMPTZ NOT NULL,
    used_at      TIMESTAMPTZ
);
CREATE INDEX idx_magic_expires ON magic_links (expires_at);

CREATE TABLE subscriber_sessions (
    id           UUID PRIMARY KEY,
    subscriber_id UUID NOT NULL REFERENCES subscribers(id) ON DELETE CASCADE,
    token_hash   BYTEA NOT NULL UNIQUE,
    expires_at   TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_subscriber_sessions_expires ON subscriber_sessions (expires_at);

CREATE TABLE subscriptions (
    subscriber_id          UUID PRIMARY KEY REFERENCES subscribers(id) ON DELETE CASCADE,
    stripe_customer_id     TEXT NOT NULL UNIQUE,
    stripe_subscription_id TEXT UNIQUE,
    plan                   TEXT NOT NULL DEFAULT 'free' CHECK (plan IN ('free','pro')),
    status                 TEXT NOT NULL DEFAULT 'active'
                           CHECK (status IN ('active','past_due','canceled','incomplete','trialing')),
    current_period_end     TIMESTAMPTZ,
    past_due_since         TIMESTAMPTZ,
    cancel_at_period_end   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_subs_past_due ON subscriptions (past_due_since) WHERE past_due_since IS NOT NULL;

CREATE TABLE posts (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9-]{1,120}$'),
    title         TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    summary       TEXT NOT NULL DEFAULT '',
    body_md       TEXT NOT NULL,
    body_html     TEXT NOT NULL,
    is_gated      BOOLEAN NOT NULL DEFAULT FALSE,
    published_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_posts_published ON posts (published_at DESC NULLS LAST);

CREATE TABLE stripe_events (
    event_id     TEXT PRIMARY KEY,
    received_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
