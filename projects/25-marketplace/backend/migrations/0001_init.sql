CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    role          TEXT NOT NULL DEFAULT 'student' CHECK (role IN ('student','instructor','admin')),
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

-- One row per instructor. `stripe_account_id` is the Connect Express
-- account id (`acct_…`). `payouts_enabled` reflects the latest
-- `account.updated` webhook from Stripe.
CREATE TABLE instructors (
    user_id            UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    stripe_account_id  TEXT UNIQUE,
    payouts_enabled    BOOLEAN NOT NULL DEFAULT FALSE,
    details_submitted  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE courses (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instructor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    slug          TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9-]{1,120}$'),
    title         TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    summary       TEXT NOT NULL DEFAULT '',
    price_cents   BIGINT NOT NULL CHECK (price_cents > 0),
    currency      TEXT NOT NULL DEFAULT 'usd' CHECK (length(currency) = 3),
    status        TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_courses_instructor ON courses (instructor_id);

CREATE TABLE lessons (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id       UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    title           TEXT NOT NULL,
    position        INT NOT NULL,
    video_path      TEXT, -- local storage; production would be object_store (project 15)
    hls_manifest    TEXT, -- populated by the HLS pipeline (mocked here)
    duration_seconds INT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_lessons_course ON lessons (course_id, position);

-- Enrollments are the post-payment record. `stripe_payment_intent_id`
-- is what the webhook handler matches incoming events against.
CREATE TABLE enrollments (
    course_id                 UUID NOT NULL REFERENCES courses(id),
    student_user_id           UUID NOT NULL REFERENCES users(id),
    stripe_payment_intent_id  TEXT NOT NULL,
    refunded_at               TIMESTAMPTZ,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (course_id, student_user_id)
);
CREATE UNIQUE INDEX uq_enrollments_pi ON enrollments (stripe_payment_intent_id);

CREATE TABLE stripe_events (
    event_id     TEXT PRIMARY KEY,
    received_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
