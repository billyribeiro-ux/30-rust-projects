-- =========================================================================
-- Project 16 — Digital Product Storefront: schema
-- =========================================================================
-- Two audiences in this DB:
--  • Admins: have user rows + server-side sessions (same pattern as 11–14).
--  • Customers: identified by email only; no row in `users`. They never
--    log in; they get signed download links by email after paying Stripe.
--
-- Stripe-related tables:
--  • orders            — one row per checkout session we create.
--  • order_items       — line items (we currently support one-product
--                        checkouts, but the schema doesn't bake that in).
--  • download_links    — signed-URL receipts. Token is stored hashed.
--  • stripe_events     — idempotency table. PK = event_id. A duplicate
--                        webhook delivery hits the unique constraint and
--                        we short-circuit without re-fulfilling.
-- =========================================================================

CREATE TABLE users (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email             TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash     TEXT NOT NULL,
    name              TEXT NOT NULL DEFAULT '',
    role              TEXT NOT NULL DEFAULT 'admin'
                      CHECK (role IN ('admin')),
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

CREATE TABLE products (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sku         TEXT NOT NULL UNIQUE CHECK (length(sku) BETWEEN 1 AND 80),
    name        TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    description TEXT NOT NULL DEFAULT '',
    -- Stored as the smallest currency unit (cents for USD). NEVER floats.
    price_cents BIGINT NOT NULL CHECK (price_cents > 0),
    currency    TEXT NOT NULL DEFAULT 'usd'
                CHECK (currency ~ '^[a-z]{3}$'),
    -- Path on disk under PRODUCT_FILES_DIR. NULL until a file is uploaded.
    file_path   TEXT,
    file_name   TEXT,
    file_size   BIGINT,
    active      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_products_active ON products (active);

CREATE TABLE orders (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    customer_email      TEXT NOT NULL CHECK (customer_email = lower(customer_email)),
    -- Single-product checkout for now; order_items below is the proper model.
    stripe_session_id   TEXT NOT NULL UNIQUE,
    stripe_payment_intent TEXT,
    status              TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'paid', 'fulfilled', 'refunded', 'failed')),
    amount_cents        BIGINT NOT NULL CHECK (amount_cents >= 0),
    currency            TEXT NOT NULL CHECK (currency ~ '^[a-z]{3}$'),
    -- We pass this to Stripe as the Idempotency-Key header when creating
    -- the Checkout Session, so a duplicate POST /api/checkout doesn't
    -- create two sessions for the same intent.
    idempotency_key     TEXT NOT NULL UNIQUE,
    refunded_at         TIMESTAMPTZ,
    fulfilled_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_orders_email   ON orders (customer_email);
CREATE INDEX idx_orders_status  ON orders (status);
CREATE INDEX idx_orders_created ON orders (created_at DESC);

CREATE TABLE order_items (
    order_id      UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id    UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    quantity      INTEGER NOT NULL CHECK (quantity > 0),
    unit_price_cents BIGINT NOT NULL CHECK (unit_price_cents >= 0),
    PRIMARY KEY (order_id, product_id)
);
CREATE INDEX idx_order_items_product ON order_items (product_id);

CREATE TABLE download_links (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id    UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id  UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    -- SHA-256 of the raw token, NEVER the raw token. (Same pattern as
    -- session tokens.) The signed URL embeds the raw token.
    token_hash  BYTEA NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_download_links_order ON download_links (order_id);

-- Idempotency table. Stripe will retry a webhook delivery if our handler
-- returns non-2xx (or times out). We INSERT event_id ON CONFLICT DO NOTHING:
-- the inserter is the one that owns fulfilment; the loser short-circuits.
CREATE TABLE stripe_events (
    event_id     TEXT PRIMARY KEY,
    type         TEXT NOT NULL,
    received_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    payload      JSONB NOT NULL
);
CREATE INDEX idx_stripe_events_received ON stripe_events (received_at DESC);

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_products_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_orders_updated_at
    BEFORE UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
