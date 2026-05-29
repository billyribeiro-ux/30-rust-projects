# Project 30 — SaaS Capstone: Multi-tenant Project Management

The integration of every primitive in this curriculum. A multi-tenant
project-management SaaS built on the patterns we shipped one at a time.

## What's shipped

- **Multi-tenant** — `tenants`, `memberships`, four roles
  (`owner`/`admin`/`member`/`viewer`).
- **Postgres RLS** on `projects`, `tasks`, `comments` — gate by
  `SET LOCAL app.tenant_id` per transaction (pattern from project 23).
- **Audit log via Postgres trigger** — `tasks` + `projects` mutations
  write to `audit_log` automatically with the actor from `app.user_id`
  (pattern from 23).
- **Password + sessions** — Argon2 hashing, hash-at-rest session tokens
  (pattern from 11/14/22).
- **Stripe Subscriptions groundwork** — `subscriptions` table mirrors
  Stripe state per tenant; webhook endpoint scaffolded for the
  signature-verify + idempotency dispatch from project 21.
- **Marketing site** — landing page + pricing, view-transition-friendly.
- **Cross-tenant isolation proof** — integration test where Bob's
  tenant cannot read Alice's projects (404, not 403 — we don't leak
  existence).

## What's reused-by-reference (explicit scope reduction)

The curriculum specifies "every primitive of the curriculum,
integrated." Honest about what we're not literally re-implementing
here:

| Primitive | Where it lives | What 30 needs to add |
| --- | --- | --- |
| Magic-link / OAuth / SAML / Passkeys | 23 / 20 / 26 / 28 | Drop in the routes + DB tables |
| Stripe full webhook + dunning | 16 / 21 | Port `stripe/webhook.rs` from 21 |
| Background jobs | 22 | Drop in `jobs/` module + run `--worker` mode |
| Hybrid search over tasks | 24 | Add `tsv` column + trigger on `tasks.title + body` |
| KPI dashboard | 27 | `spawn_kpi_loop` + SSE for "open task count per project" |
| Realtime presence | 13 / 14 / 26 | Add `tokio::sync::broadcast` hub per project |
| Cinematic GSAP marketing | 29 | Reveal-on-scroll on the landing-page feature cards |

Every reference is one file copy away. The capstone here is the
**integration architecture** — tenants → RLS → audit, with each prior
project's module slotting in.

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **2/2 pass** (Argon2 + cross-tenant isolation + audit trigger) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **4/4 pass** |
| `pnpm test:e2e` | **12/12 pass** (3 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean |

## Layout

```
projects/30-capstone/
├── README.md, COMMANDS.md, LESSON.md
├── docker-compose.yml                 postgres + mailhog
├── backend/
│   ├── Cargo.toml
│   ├── .env.example                   STRIPE_*, SMTP_URL
│   ├── migrations/0001_init.sql       tenants/memberships/projects/tasks/comments + RLS + audit trigger
│   ├── .sqlx/
│   ├── tests/capstone_flow.rs         cross-tenant isolation + audit row
│   └── src/
│       ├── main.rs, lib.rs
│       ├── auth/{hash,session,rls,mod}.rs
│       ├── routes/
│       │   ├── auth.rs                password auth
│       │   ├── tenants.rs             list + create
│       │   ├── projects.rs            projects + tasks (RLS-bounded)
│       │   └── stripe.rs              webhook stub
│       └── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json                   ports dev 5202, preview 4202
    ├── e2e/capstone.spec.ts           3 specs × 4 viewports = 12 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts
        └── routes/
            ├── +layout.{svelte,server.ts}, +page.svelte                 marketing
            ├── pricing/+page.svelte                                     plans
            ├── login/+page.{svelte,server.ts}
            └── p/+page.{svelte,server.ts}                              workspaces
```

## What you can do now

You can architect, build, ship, and operate a real multi-tenant SaaS.
You know which Postgres feature solves which problem, which auth
strategy fits which user, and which billing primitive applies when.
You can read this codebase and the prior 29 and say "I have written
every line of this myself."

You are the senior engineer in the room.

## Closing

Congratulations. Open `projects/01-todo/COMMANDS.md` if you ever need
to remember where the foundation lives, and ship something real with
what you've built here.
