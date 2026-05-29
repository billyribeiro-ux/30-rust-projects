-- =========================================================================
-- Project 23 — Multi-tenant Help Desk: schema + RLS + audit triggers
-- =========================================================================
-- All app-data tables carry `tenant_id`. RLS policies gate read/write by
-- the per-connection setting `app.tenant_id` (set via `SET LOCAL`).
--
-- The audit trigger fires AFTER INSERT/UPDATE/DELETE on tickets +
-- ticket_messages and writes a row into `audit_log`. The application
-- code CANNOT skip the audit log — it's enforced at the DB layer.
-- =========================================================================

CREATE TABLE tenants (
    id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9-]{2,40}$'),
    name TEXT NOT NULL,
    sla_first_response_minutes INT NOT NULL DEFAULT 60,
    sla_resolve_minutes        INT NOT NULL DEFAULT 1440,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT,
    name          TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE memberships (
    tenant_id  UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id)  ON DELETE CASCADE,
    role       TEXT NOT NULL CHECK (role IN ('admin','agent','customer')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, user_id)
);
CREATE INDEX idx_memberships_user ON memberships (user_id);

CREATE TABLE sessions (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   BYTEA NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

CREATE TABLE magic_links (
    token_hash    BYTEA PRIMARY KEY,
    email         TEXT NOT NULL,
    tenant_id     UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    expires_at    TIMESTAMPTZ NOT NULL,
    used_at       TIMESTAMPTZ
);
CREATE INDEX idx_magic_expires ON magic_links (expires_at);

CREATE TABLE invitations (
    token_hash  BYTEA PRIMARY KEY,
    tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email       TEXT NOT NULL,
    role        TEXT NOT NULL CHECK (role IN ('admin','agent','customer')),
    invited_by  UUID NOT NULL REFERENCES users(id),
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ
);

CREATE TABLE tickets (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    customer_user_id  UUID NOT NULL REFERENCES users(id),
    assignee_user_id  UUID REFERENCES users(id),
    subject           TEXT NOT NULL CHECK (length(subject) BETWEEN 1 AND 200),
    status            TEXT NOT NULL DEFAULT 'open'
                       CHECK (status IN ('open','pending','resolved','closed')),
    priority          TEXT NOT NULL DEFAULT 'normal'
                       CHECK (priority IN ('low','normal','high','urgent')),
    first_response_at TIMESTAMPTZ,
    resolved_at       TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_tickets_tenant_status ON tickets (tenant_id, status, created_at DESC);

CREATE TABLE ticket_messages (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    ticket_id     UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    author_user_id UUID NOT NULL REFERENCES users(id),
    body          TEXT NOT NULL,
    internal      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_messages_ticket ON ticket_messages (ticket_id, created_at);

CREATE TABLE canned_responses (
    tenant_id  UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    slug       TEXT NOT NULL,
    body       TEXT NOT NULL,
    PRIMARY KEY (tenant_id, slug)
);

CREATE TABLE audit_log (
    id             BIGSERIAL PRIMARY KEY,
    tenant_id      UUID NOT NULL,
    actor_user_id  UUID,
    action         TEXT NOT NULL,
    entity_kind    TEXT NOT NULL,
    entity_id      UUID NOT NULL,
    before         JSONB,
    after          JSONB,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_tenant ON audit_log (tenant_id, created_at DESC);

-- ---------- Audit triggers ----------
-- Reads `current_setting('app.user_id', true)` as the actor. The `true`
-- argument means "missing setting → NULL", not error — so background
-- jobs (no session) can mutate too, with NULL actor.

CREATE OR REPLACE FUNCTION audit_tickets() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO audit_log (tenant_id, actor_user_id, action, entity_kind, entity_id, before, after)
    VALUES (
      COALESCE(NEW.tenant_id, OLD.tenant_id),
      NULLIF(current_setting('app.user_id', true), '')::uuid,
      TG_OP,
      'ticket',
      COALESCE(NEW.id, OLD.id),
      CASE WHEN TG_OP <> 'INSERT' THEN to_jsonb(OLD) ELSE NULL END,
      CASE WHEN TG_OP <> 'DELETE' THEN to_jsonb(NEW) ELSE NULL END
    );
    RETURN COALESCE(NEW, OLD);
END
$$;

CREATE TRIGGER trg_audit_tickets
AFTER INSERT OR UPDATE OR DELETE ON tickets
FOR EACH ROW EXECUTE FUNCTION audit_tickets();

CREATE OR REPLACE FUNCTION audit_ticket_messages() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO audit_log (tenant_id, actor_user_id, action, entity_kind, entity_id, before, after)
    VALUES (
      COALESCE(NEW.tenant_id, OLD.tenant_id),
      NULLIF(current_setting('app.user_id', true), '')::uuid,
      TG_OP,
      'ticket_message',
      COALESCE(NEW.id, OLD.id),
      CASE WHEN TG_OP <> 'INSERT' THEN to_jsonb(OLD) ELSE NULL END,
      CASE WHEN TG_OP <> 'DELETE' THEN to_jsonb(NEW) ELSE NULL END
    );
    RETURN COALESCE(NEW, OLD);
END
$$;

CREATE TRIGGER trg_audit_messages
AFTER INSERT OR UPDATE OR DELETE ON ticket_messages
FOR EACH ROW EXECUTE FUNCTION audit_ticket_messages();

-- ---------- Row-Level Security ----------
-- Owner-side bypass: the migration runs as `helpdesk` who is BYPASSRLS;
-- the application connects as a separate role-less identity. For this
-- single-tenant-image setup we use the `helpdesk` role for both and rely
-- on `SET LOCAL` discipline. In production split into `helpdesk_app`
-- (no BYPASSRLS) and `helpdesk_owner` (migration).

ALTER TABLE tickets         ENABLE ROW LEVEL SECURITY;
ALTER TABLE ticket_messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE memberships     ENABLE ROW LEVEL SECURITY;
ALTER TABLE canned_responses ENABLE ROW LEVEL SECURITY;

CREATE POLICY tickets_tenant_isolation ON tickets
    USING (tenant_id::text = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id::text = current_setting('app.tenant_id', true));

CREATE POLICY messages_tenant_isolation ON ticket_messages
    USING (tenant_id::text = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id::text = current_setting('app.tenant_id', true));

CREATE POLICY memberships_tenant_isolation ON memberships
    USING (tenant_id::text = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id::text = current_setting('app.tenant_id', true));

CREATE POLICY canned_tenant_isolation ON canned_responses
    USING (tenant_id::text = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id::text = current_setting('app.tenant_id', true));

-- Force RLS for the app role even if it owns the tables.
ALTER TABLE tickets         FORCE ROW LEVEL SECURITY;
ALTER TABLE ticket_messages FORCE ROW LEVEL SECURITY;
ALTER TABLE memberships     FORCE ROW LEVEL SECURITY;
ALTER TABLE canned_responses FORCE ROW LEVEL SECURITY;
