# Curriculum — 30 Projects

Each project builds on the last. Skip nothing.

## Tier 1 — Foundations (SQLite, no auth)

1. **TODO Manager** — first runes, first Axum, first sqlx, SQLite migrations.
2. **Markdown Notes** — SEO baseline, `<svelte:head>`, server-side sanitization, prerendered marketing.
3. **Habit Tracker with Streaks** — `$derived`, CSS Grid, window-function SQL.
4. **Pomodoro Timer** — `$effect` with cleanup, `prefers-reduced-motion`, optimistic UI.
5. **Bookmark Manager** — many-to-many SQL, Playwright at 4 breakpoints, URL state.
6. **Expense Splitter** — form actions + validation, money as `i64` cents, `IntoResponse` errors.

## Tier 2 — Relations and auth (Postgres + sessions)

7. **Personal Finance Ledger** — charts from scratch, CSV import, `rust_decimal`.
8. **Recipe Book** — **remote functions** introduced, file upload, Recipe JSON-LD.
9. **Reading Tracker** — external APIs, `moka` caching, snippets.
10. **Workout Logger** — complex `$derived` chains, indexed search, axe-core in CI.
11. **Contact Manager + Dashboard** — **Postgres**, **auth** (Argon2 + sessions), role guards.
12. **Job Application Tracker** — timelines, background email reminders, ICS export.

## Tier 3 — Real-time, files, search

13. **Real-time Chat** — WebSockets, broadcast, presence.
14. **Kanban Issue Tracker** — DnD, `animate:flip`, **GSAP timelines**, optimistic rollback.
15. **Calendar & Scheduler** — RRULE, timezones, shared calendars.
16. **File Vault** — chunked/resumable uploads, S3-compatible store, mime sniffing.
17. **Image Gallery** — server-side optimizer, `<picture>` srcset, **GSAP ScrollTrigger**.
18. **URL Shortener + Analytics** — high-throughput Axum, geo lookup, CWV beacon.

## Tier 4 — Money, scale, ops

19. **Newsletter Platform** — **Stripe #1** (Subscriptions Checkout), webhooks, dunning.
20. **Course Marketplace** — **Stripe #2** (Connect Express), payouts, HLS transcoding.
21. **Polls & Surveys** — Postgres `LISTEN/NOTIFY` → SSE, spring-animated bars.
22. **Geo Restaurant Finder** — PostGIS, MapLibre, server-rendered SEO results.
23. **Background Jobs Dashboard** — `SKIP LOCKED` queue, retries, OTLP tracing.
24. **Multi-tenant Help Desk** — RLS, tenant subdomains, RBAC, audit log triggers.

## Tier 5 — Distinguished work

25. **API Rate-limit Gateway** — Redis token bucket, JWT for service auth.
26. **Hybrid Search KB** — `tsvector` + `pg_trgm` + Meilisearch.
27. **Live Coding Interview** — CodeMirror via `@attach`, CRDT (`yrs`), sandboxed exec.
28. **Realtime Analytics** — DuckDB OLAP, WS-pushed KPI cards, **GSAP cinematic** reveals.
29. **Cinematic Portfolio + CMS** — full static, **heavy GSAP**, View Transitions, Lighthouse 100s.
30. **SaaS Capstone** — multi-tenant, billed, real-time, observable, **GSAP marketing site**.

## Stripe coverage

- Project 19 — Subscriptions Checkout (and Customer Portal).
- Project 20 — Connect Express (multi-vendor payouts).
- Project 30 — Stripe Billing Meters (reuses #19 patterns).

## GSAP coverage (cinematic motion)

Projects 14, 17, 28, 29, 30.

## Advanced Svelte 5 mastery — quick index

| Feature                        | Introduced in |
| ------------------------------ | ------------- |
| `$state`                       | 01            |
| `$derived`                     | 03            |
| `$effect`                      | 04            |
| `$props`                       | 01            |
| `$bindable`                    | 06            |
| `$inspect`                     | 04            |
| `$state.raw`                   | 09            |
| `$state.snapshot`              | 06            |
| `$derived.by`                  | 07            |
| `$effect.pre`                  | 14            |
| `$effect.root`                 | 13            |
| `$host` (custom elements)      | 29            |
| `SvelteMap` / `SvelteSet`      | 14            |
| `SvelteURL`                    | 05            |
| `MediaQuery` / reactivity/window | 13          |
| `svelte/transition` (fade etc.) | 02           |
| `animate:flip`                 | 14            |
| `Spring` / `Tween`             | 21            |
| `{#snippet}` / `{@render}`     | 09            |
| `<svelte:boundary>`            | 02            |
| `{@attach}`                    | 14            |
| Remote functions (`$app/server`) | 08          |
| Form actions                   | 06            |
| Hooks (`hooks.server.ts`)      | 11            |
| Shallow routing                | 17            |
| Snapshots (`export const snapshot`) | 12       |
| View Transitions API           | 29            |
| Service worker                 | 08            |

A full per-feature mapping lives in the master plan.
