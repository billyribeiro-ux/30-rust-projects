# Project 09 — Workout Logger with PR Detection

> "I can't tell if I'm getting stronger."

A workout logger that proves you are. Add exercises (autocomplete search powered by SQLite FTS5), log workouts as sets of (exercise, weight, reps, RIR), and see which sets are personal records — strictly greater cumulative volume than every prior set of the same exercise.

Ninth of 30 projects. New lessons:

- **SQLite FTS5 full-text search** on exercise names. A virtual table mirrors the `exercises` table via insert/update/delete triggers; the autocomplete endpoint runs `MATCH` with prefix tokens and orders by `rank`.
- **Pure PR-detection function with proptest** — `detect_prs(sets) -> Vec<PrInfo>`. Five properties test it, including the central one: the volumes of consecutive PRs are strictly increasing.
- **Complex `$derived` chains** — the dashboard's stat tiles depend on each other: `workouts` → `totalSets` → `averageSetsPerWorkout`, `workouts` → `last7Workouts` → `volumeThisWeek`. Each is one expression; Svelte handles the topology.
- **axe-core as a CI gate** — `enforces-zero-violations.spec.ts` is named like the failure mode it prevents. A single WCAG 2 A/AA violation on any of three routes fails the build.
- **CSV export** — `GET /api/export.csv` returns `text/csv; charset=utf-8` with chronological sets and proper RFC-4180 escaping.
- **Playwright visual regression** — `toHaveScreenshot()` against the `<PrBadge>` component on a deterministic seed.

## Stack

Same as previous projects. Adds `proptest` as a backend dev-dep; no new frontend deps.

## Quality gates

- `cargo fmt --check` clean
- `cargo clippy --all-targets -- -D warnings` clean
- `cargo test` → **20/20** (15 unit + 5 proptest properties)
- `pnpm check` → 0 ERRORS, 0 WARNINGS
- `pnpm test:unit` → **16/16** (api + weight helpers)
- `pnpm test:e2e` → **40/40** (5 specs × 4 viewports + visual regression)
- svelte-autofixer clean (one intentional `bind:this` + one intentional debounced `$effect`, both documented in `LESSON.md`)

## Ports

- Backend: `3008`
- Frontend dev: `5181`
- Playwright preview: `4181`

## What's next

Project 10 — last single-user SQLite project, then files + uploads in 11+.
