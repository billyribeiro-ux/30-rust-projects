# Project 22 — Background Jobs Dashboard

A Postgres-backed job queue with workers, exponential backoff, dead-lettering,
and a live SSE-driven dashboard. No Redis. No RabbitMQ. Just `SELECT … FOR
UPDATE SKIP LOCKED`.

## What's new in this project

- **Postgres-backed queue** using `FOR UPDATE SKIP LOCKED`. Two workers
  hitting the same row don't block — one wins, the other moves to the
  next row. Proven by an integration test.
- **Tokio-based worker tier** with bounded concurrency via `Semaphore`.
  Runs in the same binary as the API; `RUN_WORKERS=0` disables it for
  API-only deployments.
- **Exponential backoff with jitter** (`30s → 90s → 4.5min …`, capped at
  1h). Proptested to be monotonic in expectation and capped under the
  ceiling × max jitter.
- **Retry → dead-letter** on `attempts >= max_attempts`. Each attempt
  writes a row to `job_results` so the dashboard timeline is honest.
- **SSE live feed** at `/api/stream/jobs`. The dashboard subscribes via
  `EventSource` and `invalidateAll()`s the page data on every event —
  no polling.
- **Lease pattern**: claimed jobs carry `locked_until`. If a worker
  dies mid-job the lease expires and another worker re-claims (extending
  the lease isn't shipped — documented as future work in §A).

## Stack delta vs project 21

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres, Argon2 sessions, Axum | Adds `jobs/` module (backoff + queue + registry + runner), broadcast channel → SSE, JOB result history |
| Frontend | hooks.server.ts auth | Adds `EventSource` SSE consumer + `invalidateAll()` refresh pattern, queues table, enqueue form |
| Tests | sqlx integration | Adds **proptest-style** statistical assertions on backoff schedule, **race test** for SKIP LOCKED |

## Layout

```
projects/22-jobs/
├── README.md, COMMANDS.md, LESSON.md
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml
│   ├── .env.example
│   ├── migrations/0001_init.sql
│   ├── .sqlx/
│   ├── tests/queue_flow.rs        SKIP LOCKED race, retry → dead-letter
│   └── src/
│       ├── main.rs                spawns worker for "default" queue
│       ├── lib.rs                 build_app
│       ├── auth/{hash,session,mod}.rs
│       ├── jobs/
│       │   ├── backoff.rs         delay_secs() + tests
│       │   ├── queue.rs           enqueue / claim / mark
│       │   ├── registry.rs        kind → HandlerFn map
│       │   └── runner.rs          worker loop
│       ├── routes/
│       │   ├── auth.rs
│       │   ├── admin.rs           /queues, /jobs CRUD, retry
│       │   └── stream.rs          SSE
│       └── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json               ports dev 5194, preview 4194
    ├── e2e/jobs.spec.ts           3 specs × 4 viewports = 12 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts
        └── routes/
            ├── +layout.{svelte,server.ts}, +page.server.ts (→ /jobs or /login)
            ├── login/+page.{svelte,server.ts}
            ├── signup/+page.{svelte,server.ts}
            ├── logout/+page.server.ts
            └── jobs/+page.{svelte,server.ts}     dashboard
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **6/6 pass** (3 unit incl. backoff stats; 3 integration: SKIP LOCKED race, retry→dead-letter, job_results history) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **5/5 pass** |
| `pnpm test:e2e` | **12/12 pass** (3 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean |

## What you can do now

Replace Sidekiq / Resque / a "we'll add Celery later" plan with one
PostgreSQL extension you already have. Observe a worker tier in
production. Reason about retry/lease/dead-letter as a state machine,
not as "we'll figure it out."

## What's next

Project 23 — **Multi-tenant Help Desk** with Postgres RLS, magic
links, and a Postgres-trigger audit log.
