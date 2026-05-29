# Project 23 — Multi-tenant Help Desk (+ Magic Links + RLS)

A SaaS help desk where every tenant's data is isolated **at the database
layer** via Postgres Row-Level Security. The application code can have
an auth bug and still not leak — RLS holds the line.

## What's new in this project

- **Postgres RLS** on `tickets`, `ticket_messages`, `canned_responses`.
  Policies key on `current_setting('app.tenant_id')`, set via `SET LOCAL`
  inside a transaction. The next transaction on the same connection
  sees no carryover.
- **Magic-link auth** for customers (passwordless). Single-use, hashed
  at rest, 10-min TTL. The endpoint never reveals whether the email/tenant
  exists.
- **Postgres-trigger audit log** on `tickets` + `ticket_messages`. The
  application cannot skip the audit row — it's enforced at the DB.
- **Role-based ACL** layered on RLS: `admin / agent / customer`.
  Customers see only their own tickets *even within their tenant*
  (app-level filter on top of RLS).
- **Tenant model**: `tenants`, `memberships`, `invitations`. Memberships
  are *not* RLS-gated (the "which tenants does user X belong to?"
  lookup is by-user and must cross tenants).

## Stack delta vs project 22

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres, Argon2, Axum, sessions | Adds `auth/rls.rs` (SET LOCAL helper), tenant routing, magic-link issuer, audit triggers |
| Frontend | hooks.server.ts auth | Adds tenants list page + tickets page per tenant + magic-link entry |
| Tests | sqlx integration | Adds **cross-tenant isolation proof** via two reqwest clients with separate cookie jars |

## Layout

```
projects/23-helpdesk/
├── README.md, COMMANDS.md, LESSON.md
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml
│   ├── .env.example
│   ├── migrations/0001_init.sql      RLS policies + audit triggers
│   ├── .sqlx/
│   ├── tests/rls_flow.rs             cross-tenant isolation, role gating
│   └── src/
│       ├── main.rs, lib.rs
│       ├── auth/{hash,session,rls,mod}.rs
│       ├── routes/
│       │   ├── auth.rs               password staff auth
│       │   ├── magic.rs              customer passwordless
│       │   ├── tenants.rs            list/create
│       │   ├── tickets.rs            RLS-aware CRUD
│       │   └── audit.rs              admin-only audit log read
│       └── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json                  ports dev 5195, preview 4195
    ├── e2e/helpdesk.spec.ts          3 specs × 4 viewports = 12 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts
        └── routes/
            ├── +layout.{svelte,server.ts}, +page.server.ts (→ /tenants or /login)
            ├── login/, signup/, logout/
            ├── tenants/+page.{svelte,server.ts}
            └── t/[slug]/+page.{svelte,server.ts}
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **3/3 pass** (Argon2 round-trip + cross-tenant isolation + customer-can't-post-internal) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **5/5 pass** |
| `pnpm test:e2e` | **12/12 pass** (3 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean |

## What you can do now

Run a multi-tenant SaaS the safe way: RLS at the DB, ACL on top, audit
log enforced by trigger, magic-link auth for customers. The app can
ship a bug and not leak.

## What's next

Project 24 — Hybrid Search Knowledge Base. Postgres `tsvector` + `pg_trgm`
for keyword + typo-tolerant search.
