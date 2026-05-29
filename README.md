# 30 Full-Stack Projects — Svelte 5 + Rust/Axum

A zero-to-Principal-Engineer curriculum. Thirty real, runnable products. Every line explained. No magic.

## Stack (every project)

- **Frontend** — SvelteKit 2 + Svelte 5 (runes) + TypeScript strict + plain CSS (no Tailwind) + Phosphor icons.
- **Backend** — Rust 2024 + Axum + tokio + sqlx (compile-checked SQL).
- **DB** — SQLite for foundations, Postgres + Redis for the rest.
- **Quality** — `sv check`, `cargo clippy`, Vitest, Playwright at 4 breakpoints, Lighthouse ≥ 95.

See [`CURRICULUM.md`](./CURRICULUM.md) for the full 30-project syllabus, and [`/root/.claude/plans/i-d-like-to-create-cuddly-spark.md`](file:///root/.claude/plans/i-d-like-to-create-cuddly-spark.md) for the master plan.

## How to read a project

Each `projects/NN-name/` folder has three teaching files plus the code:

| File          | Purpose                                                                   |
| ------------- | ------------------------------------------------------------------------- |
| `README.md`   | What we're building, who it's for, how to run it.                         |
| `COMMANDS.md` | Every shell command in order. Copy-paste your way to a running app.       |
| `LESSON.md`   | Line-by-line teaching: *what* the code says, *why*, and *what would break*. |
| `frontend/`   | SvelteKit app.                                                            |
| `backend/`    | Rust crate.                                                               |

## Prerequisites (install once)

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Node 22 LTS via fnm or nvm
# (any modern installer — verify with: node --version)

# pnpm via Corepack (ships with Node 16.10+)
corepack enable
corepack use pnpm@latest

# Docker (for Postgres/Redis/MinIO from project 11 onwards)
# Install from https://docs.docker.com/engine/install/

# sqlx-cli (compile-time SQL checking + migrations)
cargo install sqlx-cli --no-default-features --features rustls,sqlite,postgres
```

## Repo layout

```
30-rust-projects/
├── projects/
│   ├── 01-todo/
│   ├── 02-notes/
│   └── ...
└── shared/
    └── design-tokens.css   # CSS custom properties reused across projects
```

Each project is fully self-contained. No workspaces. `cd projects/01-todo && follow COMMANDS.md`.

## Progress

Tier 1 — SQLite foundations:

- [x] **01 — TODO Manager** — first runes, first Axum handler, first sqlx query.
- [x] **02 — Markdown Notes** — server-side HTML sanitization, full SEO, JSON-LD, `<svelte:boundary>`, axe-core a11y gate.
- [x] **03 — Habit Tracker** — `proptest`, SQL islands-and-gaps with window functions, 7×7 keyboard-navigable calendar grid, `prefers-reduced-motion`.
- [x] **04 — Pomodoro Timer** — `$effect` + `requestAnimationFrame` + `untrack`, Web Audio synthesized chime, `$inspect`, `<svelte:window>` shortcuts.
- [x] **05 — Bookmark Manager** — many-to-many SQL, URL-as-state filters, typed debounce, `use:clickOutside` action.
- [x] **06 — Expense Splitter** — `i64` cents money, proptested splitter (largest-remainder method), `$bindable` MoneyInput, field-level errors.
- [x] **07 — Personal Finance Ledger** — double-entry validation with proptest, `rust_decimal`, CSV import, canvas bar chart from scratch.
- [x] **08 — Reading Tracker** — Open Library API + `moka` cache, `wiremock-rs` tests, streamed `load`, `$state.raw`, class with rune fields.
- [x] **09 — Workout Logger** — SQLite FTS5 autocomplete, proptested PR detection (5 properties), axe-core CI gate, CSV export, Playwright visual-regression baselines across 4 viewports.
- [x] **10 — Recipe Book** — multipart file uploads (mime-sniff via `infer` + EXIF strip via image-encode round-trip), first SvelteKit remote function (`$app/server` query/form/command), module-scoped snippet, full Recipe JSON-LD, first service worker, HMAC-signed 24h share URLs.

Tier 2 — Postgres + Auth (11–18):

- [x] **11 — Contact Manager + Dashboard** — first Postgres + Docker Compose + MailHog; Argon2id + server-side hashed sessions + sliding expiry + HttpOnly+SameSite=Lax+Secure cookies; single-use email-verify + password-reset tokens with atomic consume; tsvector FTS with weighted GIN index; `hooks.server.ts` auth handle; cross-origin cookie forwarding; `(auth)`/`(app)` route-group guards; Playwright permission-matrix tests proving server-side per-user scoping.
- [x] 12 — Job Application Tracker (+ bcrypt legacy-migration lesson)
- [x] 13 — Real-time Chat Rooms (+ first WebSockets)
- [x] 14 — Calendar & Scheduler
- [x] **15 — File Vault with Chunked Uploads** — tus-style resumable protocol, SHA-256 content-addressed dedup, `object_store` (local + S3), EXIF strip, folder tree UI.
- [x] **16 — Digital Product Storefront (Stripe Checkout #1)** — hand-rolled webhook HMAC-SHA256 verify with replay window, `stripe_events` idempotency, HMAC-signed 24h download URLs, refunds, `wiremock`'d Stripe in CI.
- [x] 17 — URL Shortener + Analytics (+ Redis, + 2FA)
- [x] 18 — Polls & Surveys with Live Results

Tier 3 — Animation, payments, scale (19–24):

- [x] **19 — Kanban Issue Tracker (GSAP #1)** — optimistic drag-drop with rollback, `animate:flip`, lazy GSAP drop physics, lexorank positions, RBAC at the route extractor.
- [x] **20 — Geo-aware Restaurant Finder (+ OAuth)** — PostGIS `ST_DWithin`, hand-rolled OAuth 2.0 + PKCE for Google + GitHub (state/verifier/nonce), `test_only` session-injection route, Restaurant JSON-LD.
- [x] **21 — Newsletter Platform (Stripe Subscriptions #2)** — local mirror of Stripe state, dunning 14-day grace + auto-downgrade, magic-link auth for subscribers, RSS feed, 402 paywall with `isAccessibleForFree` JSON-LD.
- [x] **22 — Background Jobs Dashboard** — Postgres `FOR UPDATE SKIP LOCKED` queue, exponential backoff + jitter (proptested), dead-letter on max attempts, SSE live job stream + `invalidateAll()`.
- [x] **23 — Multi-tenant Help Desk (+ Magic Links + RLS)** — Postgres RLS via `SET LOCAL app.tenant_id`, audit log by trigger (app cannot skip), permission matrix tests, magic-link customer auth.
- [x] **24 — Hybrid Search Knowledge Base** — weighted `tsvector` GIN (title A > summary B > body C) + trigger, `pg_trgm` typo fallback, optional Meili via Reciprocal Rank Fusion, `ts_headline` snippets with `<mark>`, sitemap.xml + Article/FAQ JSON-LD.

Tier 4 — Distinguished work (25–30):

- [x] **25 — Course Marketplace (Stripe Connect #3)** — Express account-link onboarding, destination charges with `application_fee_amount` + `transfer_data`, refunds with `reverse_transfer + refund_application_fee`, 14-day refund window enforced server-side.
- [x] **26 — Live Coding Interview Platform (+ SAML/SSO)** — Axum WebSocket + `broadcast` hub for shared editor, `CodeRunner` trait (MockRunner ships, DockerRunner doc'd), SAML SP metadata endpoint, two-client WebSocket race test.
- [x] **27 — Realtime Analytics Dashboard (GSAP #2)** — `/v1/ingest` → Postgres aggregate → SSE broadcast loop, GSAP number-roll with lazy import + reduced-motion guard, DuckDB upgrade path documented.
- [x] **28 — AI Inference API (Stripe Metered Billing #4 + WebAuthn)** — `webauthn-rs` passkey enrol/login, API keys SHA-256-hashed at rest with `sk_live_` prefix, per-key RPM gate, `usage_events` partial index for Stripe Meter Events reconciler.
- [x] **29 — Cinematic Portfolio + Headless CMS (GSAP #3)** — `{@attach}` reveal-on-scroll, lazy GSAP, View Transitions API one-liner, MDX-ish content pipeline (server-rendered Markdown), CreativeWork JSON-LD + hreflang.
- [x] **30 — SaaS Capstone — Multi-tenant Project Management** — tenants × memberships × projects × tasks, Postgres RLS via `SET LOCAL`, audit log by trigger, Stripe Subscriptions groundwork, cross-tenant isolation integration test. The integration of every primitive in this curriculum.

See [`CURRICULUM.md`](./CURRICULUM.md) for the full project specs and [`PATTERNS.md`](./PATTERNS.md) for cross-project conventions.
