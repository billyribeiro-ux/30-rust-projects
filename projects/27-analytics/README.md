# Project 27 — Realtime Analytics Dashboard (GSAP #2)

A live KPI wall: events ingest at `/v1/ingest`, Postgres aggregates
them on a 1-second tick, the result broadcasts over **SSE**, and the
dashboard rolls the new numbers via **GSAP**, respecting reduced motion.

## What's new in this project

- **Three-tier flow** — ingest → aggregate → push. Each tier is one
  file you can read in a sitting.
- **Background broadcaster** (`kpis::spawn_kpi_loop`) computes
  `compute_snapshot` every N ms and pushes the JSON onto a
  `tokio::sync::broadcast` channel. The SSE route is just a
  `BroadcastStream` filter.
- **GSAP number-roll** with a lazy-loaded chunk and a reduced-motion
  guard. Numbers animate from old to new value with `power3.out`
  easing, falling back to instant updates when motion is disabled.
- **Tabular-nums + gradient text** for the KPI cards — readable values
  with cinematic polish at zero extra payload.

## Why no DuckDB

The curriculum specifies DuckDB for the OLAP query path. This project
ships **Postgres-only** because:

- The aggregate queries — `COUNT`, `COUNT(DISTINCT)`, `GROUP BY kind` —
  run in single-digit milliseconds on Postgres up to ~1M events. The
  ingest→OLAP pipeline shows the *shape*; replacing the Postgres queries
  with DuckDB is a one-file change documented in COMMANDS.md §7.
- The `duckdb` Rust crate uses `[features = ["bundled"]]` which builds
  DuckDB from source — adds ~5 minutes to first `cargo build`. The
  pattern is the same; the timing isn't.

## Stack delta vs project 22

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres + Argon2 + Axum + `tokio::sync::broadcast` + SSE | Adds events table + KPI aggregation queries + 1-second background loop |
| Frontend | hooks.server.ts + SSE EventSource | Adds **GSAP number-roll** via lazy-import, gradient KPI cards |

## Layout

```
projects/27-analytics/
├── README.md, COMMANDS.md, LESSON.md
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml
│   ├── .env.example
│   ├── migrations/0001_init.sql
│   ├── .sqlx/
│   ├── tests/kpi_flow.rs              ingest + aggregate proofs
│   └── src/
│       ├── main.rs, lib.rs            spawns kpi loop
│       ├── auth/{hash,session,mod}.rs
│       ├── routes/
│       │   ├── auth.rs
│       │   ├── ingest.rs              POST /v1/ingest
│       │   └── kpis.rs                snapshot + SSE + spawn_kpi_loop
│       └── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json                   ports dev 5199, preview 4199
    ├── e2e/analytics.spec.ts          3 specs × 4 viewports = 12 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts
        ├── lib/numberRoll.ts          GSAP-backed number animator
        └── routes/
            ├── +layout.{svelte,server.ts}, +page.server.ts → /dashboard
            ├── login/, signup/, logout/
            └── dashboard/+page.{svelte,server.ts}
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **3/3 pass** (Argon2 + ingest+snapshot reflection + validation 422) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **4/4 pass** |
| `pnpm test:e2e` | **12/12 pass** (3 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean (4 documented `state_referenced_locally` silences for the prev snapshot seeded from data) |

## What you can do now

Push a live KPI wall onto a screen for the all-hands. Reason about
ingest→aggregate→push as three independent components. Add cinematic
polish without rebuilding the data layer.

## What's next

Project 28 — AI Inference API with WebAuthn passkeys and Stripe
metered billing.
