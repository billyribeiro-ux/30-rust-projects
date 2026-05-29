# Lesson — Project 23 (Multi-tenant Help Desk, RLS)

Three things this project teaches that no earlier project did:

1. **Postgres Row-Level Security with `SET LOCAL`** as the *database*
   safety net for multi-tenant isolation.
2. **Audit logging via Postgres triggers** so the application code
   cannot forget to write one.
3. **Memberships are deliberately NOT RLS-gated** — and why that's
   the safe choice.

---

## A. Backend

### A1. The RLS policy

```sql
ALTER TABLE tickets ENABLE ROW LEVEL SECURITY;
ALTER TABLE tickets FORCE ROW LEVEL SECURITY;

CREATE POLICY tickets_tenant_isolation ON tickets
    USING (tenant_id::text = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id::text = current_setting('app.tenant_id', true));
```

- **`ENABLE` enables for normal users**, but the table owner bypasses RLS
  by default. **`FORCE`** removes that bypass — the app role respects
  the policy even though it owns the table.
- **`USING`** filters reads. **`WITH CHECK`** validates writes (you
  cannot insert a row that wouldn't be visible to you afterwards).
- **`current_setting('app.tenant_id', true)`** — the `true` argument
  means "missing setting → empty string", not error. An unscoped
  connection sees zero rows, not a server error.

### A2. The discipline: `SET LOCAL` inside a tx

```rust
pub async fn enter_tenant(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Option<Uuid>,
) -> AppResult<()> {
    sqlx::query(&format!("SET LOCAL app.tenant_id = '{tenant_id}'"))
        .execute(&mut **tx).await?;
    if let Some(uid) = user_id {
        sqlx::query(&format!("SET LOCAL app.user_id = '{uid}'"))
            .execute(&mut **tx).await?;
    }
    Ok(())
}
```

`SET LOCAL` lives for the duration of the transaction only. The
connection is returned to the pool with no app-level GUCs set. The
next request on the same connection starts fresh — there is no path
where a leaked `app.tenant_id` lets one tenant see another's data.

Every handler that touches RLS-protected tables begins with:

```rust
let mut tx = s.pool.begin().await?;
enter_tenant(&mut tx, tenant_id, Some(user.id)).await?;
// … use &mut *tx for every RLS-aware query …
tx.commit().await?;
```

We don't ship a "default tenant context" middleware — putting it at the
extractor layer hides the connection-vs-transaction lifetime story, and
that's where the bugs live.

### A3. The audit trigger

```sql
CREATE OR REPLACE FUNCTION audit_tickets() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO audit_log (tenant_id, actor_user_id, action, entity_kind, entity_id, before, after)
    VALUES (
      COALESCE(NEW.tenant_id, OLD.tenant_id),
      NULLIF(current_setting('app.user_id', true), '')::uuid,
      TG_OP, 'ticket', COALESCE(NEW.id, OLD.id),
      CASE WHEN TG_OP <> 'INSERT' THEN to_jsonb(OLD) ELSE NULL END,
      CASE WHEN TG_OP <> 'DELETE' THEN to_jsonb(NEW) ELSE NULL END
    );
    RETURN COALESCE(NEW, OLD);
END
$$;
```

Why this beats application-level logging:

- **You cannot forget.** The app has many code paths into tickets; the
  trigger fires on all of them. Adding a new endpoint? Audit covered.
- **Atomic with the change.** The audit row commits in the same
  transaction as the mutation. No partial state.
- **`actor_user_id` reads the GUC** the app set with `SET LOCAL
  app.user_id = '…'`. If the GUC is unset (e.g., a background job
  with no user context), the actor is NULL — honest about provenance.

### A4. Why memberships are *not* RLS-gated

If we put RLS on `memberships`, the auth path

```sql
SELECT t.id, m.role
FROM tenants t JOIN memberships m ON m.tenant_id = t.id
WHERE t.slug = $1 AND m.user_id = $2
```

…runs **before** we know the `tenant_id` to put into `app.tenant_id`.
Chicken-and-egg. We'd have to disable RLS for this lookup, which means
the discipline isn't airtight anyway.

The cross-tenant blast radius of "your user_id appears in memberships"
is *your own memberships*, which you already know. So we drop RLS on
the memberships table and gain bootstrap simplicity. The cost is
explicit: a SQL injection in a different query path *could* read
memberships. We have parameterized queries everywhere; if that fails
we have bigger problems.

### A5. The cross-tenant proof — `tests/rls_flow.rs`

Two reqwest clients (`alice`, `bob`), each with its own cookie jar.
Alice creates tenant Acme and a ticket. Bob, signed in as himself,
GETs `/api/t/acme/tickets`. The test asserts **404**, not 403 — we
don't leak the tenant's existence to a non-member.

The audit assertion is one SQL:

```sql
SELECT COUNT(*) FROM audit_log
WHERE entity_kind = 'ticket' AND action = 'INSERT'
```

The trigger wrote it. The app didn't need to.

---

## B. Frontend

Minimal but real:

- `/login` + `/signup` for staff (password) auth.
- `/tenants` shows tenants the signed-in user belongs to, plus a
  create form.
- `/t/[slug]` lists tickets in that tenant (RLS-bounded server-side)
  + new-ticket form.

No magic-link UI is shipped on the frontend — the backend's `POST
/api/magic/start` works via curl/email, and project 30 (capstone)
brings it into the UI. Documented.

---

## C. Tests

### C1. `cargo test` (3 tests)

- `auth::hash::tests::round_trip` — Argon2 sanity.
- `rls_flow::cross_tenant_isolation_via_rls` — Alice's tenant invisible
  to Bob; audit row exists.
- `rls_flow::customer_cannot_post_internal_message` — role enforcement.

### C2. `pnpm test:unit` (5 tests)

`tenantsApi.list`, `ticketsApi.list` URL-encoding, `ticketsApi.create`
body shape, `ticketsApi.postMessage` internal flag, non-2xx →
`ApiCallError`.

### C3. `pnpm test:e2e` (3 × 4 = 12 runs)

- Redirect to /login when unauthenticated.
- /login axe-clean.
- Signed-in: create tenant, create ticket, see it in the list.

---

## D. What you can do now

1. Stand up multi-tenant SaaS with database-level isolation.
2. Reason about RLS as a *safety net*, not a *replacement* for app-
   level auth — both layers must be right.
3. Make audit logging undeniable via triggers.

Project 24 — Hybrid Search Knowledge Base. Postgres `tsvector` +
`pg_trgm`, weighted GIN index, ts_headline snippets.
