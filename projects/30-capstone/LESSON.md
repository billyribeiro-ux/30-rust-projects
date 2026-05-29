# Lesson — Project 30 (SaaS Capstone)

This is the integration. Not a new lesson — a *connecting* lesson.

For each primitive the capstone uses, here's the previous project
that taught it and the single change to wire it in here.

---

## A. Connecting every primitive

### A1. Tenants (the foundation, novel to this project)

Every domain table carries `tenant_id`. Memberships join users to
tenants with a role. All other primitives layer on top.

The four roles (`owner`/`admin`/`member`/`viewer`) follow the rank
pattern from project 19 — `viewer` < `member` < `admin` < `owner`.
The capstone enforces "viewer cannot mutate" at the handler layer
(check `role == "viewer"`); the RLS policies only enforce *which
tenant's rows you can see*, not *which actions you can take*. The
split is intentional: RLS is the "no leak" safety net; the role
matrix is the explicit ACL.

### A2. Postgres RLS — from project 23

```sql
ALTER TABLE projects ENABLE ROW LEVEL SECURITY;
ALTER TABLE projects FORCE ROW LEVEL SECURITY;
CREATE POLICY projects_iso ON projects
  USING (tenant_id::text = current_setting('app.tenant_id', true))
  WITH CHECK (tenant_id::text = current_setting('app.tenant_id', true));
```

Same pattern as 23, three tables instead of two. The `auth/rls.rs`
helper is copied verbatim — `enter_tenant(tx, tenant_id, user_id)`
sets both GUCs, runs once per transaction. Every handler that touches
RLS tables does:

```rust
let mut tx = s.pool.begin().await?;
enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
// ... queries ...
tx.commit().await?;
```

### A3. Audit triggers — from project 23

```sql
CREATE OR REPLACE FUNCTION audit_row() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO audit_log (tenant_id, actor_user_id, action, entity_kind, entity_id)
    VALUES (
      COALESCE(NEW.tenant_id, OLD.tenant_id),
      NULLIF(current_setting('app.user_id', true), '')::uuid,
      TG_OP, TG_TABLE_NAME, COALESCE(NEW.id, OLD.id)
    );
    RETURN COALESCE(NEW, OLD);
END
$$;
CREATE TRIGGER trg_audit_tasks    AFTER INSERT OR UPDATE OR DELETE ON tasks    FOR EACH ROW EXECUTE FUNCTION audit_row();
CREATE TRIGGER trg_audit_projects AFTER INSERT OR UPDATE OR DELETE ON projects FOR EACH ROW EXECUTE FUNCTION audit_row();
```

The trigger reads `current_setting('app.user_id', true)` — the second
arg `true` means "missing setting → NULL" instead of error. So a
background job (no user context) can mutate too, with NULL actor.
The capstone test asserts that an `INSERT INTO projects` produces an
`audit_log` row with `action='INSERT'` and `entity_kind='projects'`.

### A4. Auth — from project 11 / 14 / 22

Password + sessions, identical pattern. The `auth/session.rs` file is
copied. To add the other auth strategies the curriculum lists:

- **Magic links** — copy `routes/magic.rs` from project 23. Add
  `magic_links` table.
- **OAuth Google/GitHub** — copy `oauth/` from project 20.
- **SAML** — copy `routes/saml.rs` from project 26 (metadata) and the
  `samael` integration described in 26's LESSON.
- **WebAuthn passkeys** — copy `routes/passkeys.rs` + `webauthn-rs`
  setup from project 28.

Each one is "drop in a module + add a route nest in `lib.rs`."

### A5. Stripe Subscriptions — from project 21

The `subscriptions` table mirrors Stripe state, one row per tenant.
The webhook stub at `routes/stripe.rs` returns 200 to anything; the
production wire-up replaces it with project 21's full
`stripe/webhook.rs` (HMAC verify + replay window + idempotency on
`stripe_events.event_id` + dispatch by event type).

The `subscriptions` schema includes `seats` for seat-based billing.
A `count(*)` over `memberships` per tenant enforces the seat limit
on `POST /api/memberships` (not shipped in 30, documented as
follow-up).

### A6. Background jobs — from project 22

Drop in the `jobs/` module from project 22. Run the binary with
`RUN_WORKERS=1` to enable the worker pool. The capstone's reaper
needs are:

- **Dunning** — daily check for `past_due_since < now() - INTERVAL '14 days'`
  and downgrade.
- **Audit log retention** — monthly delete from `audit_log` where
  `created_at < now() - INTERVAL '1 year'` (subject to your retention
  policy).
- **Email digests** — weekly task summary per tenant.

### A7. Search — from project 24

For hybrid search over tasks, add:

```sql
ALTER TABLE tasks ADD COLUMN tsv tsvector;
CREATE INDEX idx_tasks_tsv ON tasks USING GIN (tsv);
CREATE OR REPLACE FUNCTION tasks_tsv_update() RETURNS TRIGGER ... ;
CREATE TRIGGER trg_tasks_tsv BEFORE INSERT OR UPDATE OF title, body ON tasks ... ;
```

…and the `search.rs` from 24 with `tsvector @@ plainto_tsquery`. The
RLS policy on `tasks` already scopes search to the current tenant.

### A8. KPIs — from project 27

The dashboard widgets the capstone would show:

- Open tasks per project (single GROUP BY tasks.project_id).
- Median task age in open status (PERCENTILE_CONT on `now() - created_at`).
- Active members in the last week (`memberships` JOIN `audit_log` on
  actor_user_id).

The `spawn_kpi_loop` + SSE plumbing is the project 27 pattern.

### A9. Cinematic marketing — from project 29

The landing page is server-rendered, ready for view-transition
animations on the feature cards. Adding GSAP reveal-on-scroll is
copying `lib/motion.ts` from 29 and `{@attach}`-ing each card.

---

## B. The capstone test

`tests/capstone_flow.rs::full_flow_signup_tenant_project_isolation`:

1. Alice registers, creates tenant Acme, creates a project.
2. Bob registers, creates tenant Globex.
3. Bob GETs Acme's projects — expects 404. The RLS policy makes the
   row invisible; the handler returns NotFound rather than Forbidden
   because we don't reveal that Acme exists.
4. Direct DB check: `audit_log` has at least one `INSERT` row for
   `entity_kind='projects'`. The trigger fired.

That single test exercises:
- session auth
- multi-tenant model
- RLS isolation
- audit trigger

…which is *why* it's the one capstone test. Not a "tests every
endpoint" suite; a "the integration works at the surfaces that
matter" proof.

---

## C. Deploy

`docker compose up -d && cargo run` is the dev path. For production:

1. **Database** — managed Postgres with PITR (Point-In-Time Recovery)
   on the WAL. Pretty much any cloud Postgres does this.
2. **Web tier** — `cargo build --release && ./target/release/capstone-backend`
   behind Caddy or nginx. Reverse-proxy TLS termination.
3. **Workers** — same binary with `RUN_WORKERS=1` as a separate
   systemd unit (so the API tier can scale independently of the
   workers).
4. **Observability** — wire `tracing-opentelemetry` + OTLP HTTP
   exporter to Tempo/Jaeger (project 22 lesson).
5. **Email** — replace MailHog with a real provider (Resend, Postmark,
   SES). The `lettre` SMTP URL is the only change.
6. **Stripe** — set `STRIPE_SECRET_KEY` and `STRIPE_WEBHOOK_SECRET`
   to the live values. Stripe CLI in dev for local webhook forwarding.
7. **CI** — copy the gate sequence from any prior project's
   COMMANDS.md.

---

## D. What you can do now

You can build any product on this curriculum's surface. There's no
new primitive to learn — every primitive you've shipped here is in
the toolkit a senior engineer at a top-tier tech company uses on a
Tuesday afternoon.

Welcome to the other side.
