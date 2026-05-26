# Project 03 — Habit Tracker with Streaks

> "I want to build a daily habit and need to see my streak."

A tiny habit tracker. Add a habit, click cells in the 7×7 calendar grid to mark completions, watch your current and longest streaks update. Keyboard-navigable, motion-respectful, mathematically correct.

This is the third of 30 projects. It introduces:

- **`$derived` for streak math** — current/longest/total streaks recompute from the completions array without explicit subscriptions.
- **Pure functions tested with `proptest`** — first property-based testing in the curriculum. Seven invariants verified against thousands of randomly generated date sequences.
- **Complex SQL with window functions** — the "islands and gaps" pattern (`ROW_NUMBER() OVER (ORDER BY …)`, group-by-streak-key) to group consecutive dates into streak windows. Exposed via `GET /api/habits/:id/streak-windows`.
- **CSS Grid mastery** — `grid-auto-flow: column`, `grid-template-rows: repeat(7, 1fr)` for the calendar.
- **Keyboard navigation in a grid** — arrow keys, Home, End move focus between cells; `tabindex={0}` on the entry cell, `-1` on the rest. WCAG 2.1.1 compliant.
- **`prefers-reduced-motion`** honored — cell transition disabled when the user opts out. Test verifies it.

Everything from projects 01 and 02 still applies (Svelte 5 runes, sqlx compile-time-checked queries, Axum, the 4-viewport Playwright matrix with axe-core a11y).

## Stack

- **Frontend** — SvelteKit 2 + Svelte 5 (runes), TypeScript strict, plain CSS with cascade layers, Phosphor icons, `@axe-core/playwright`.
- **Backend** — Rust 2024 + Axum 0.8 + sqlx + SQLite. **`proptest`** for property-based tests of the streak function.
- **Tests** — `cargo test` (19 tests: 12 unit + 7 proptest, each generating ~1024 cases). Vitest (6 tests). Playwright (5 specs × 4 viewports = 20 runs).

## Layout

```
projects/03-habits/
├── README.md          ← you are here
├── COMMANDS.md
├── LESSON.md
├── backend/
│   ├── Cargo.toml
│   ├── migrations/0001_init.sql
│   └── src/
│       ├── main.rs
│       ├── db.rs
│       ├── error.rs
│       ├── streaks.rs     (pure function + 7 proptest properties)
│       └── routes/
│           ├── mod.rs
│           └── habits.rs   (CRUD + toggle + window-function streak_windows)
└── frontend/
    ├── package.json
    ├── svelte.config.js
    ├── vite.config.ts
    ├── tsconfig.json
    ├── playwright.config.ts
    ├── src/
    │   ├── app.html
    │   ├── app.css
    │   ├── test-setup.ts
    │   ├── lib/
    │   │   ├── api.ts            (typed fetch client + ApiCallError)
    │   │   ├── api.test.ts       (6 Vitest)
    │   │   ├── types.ts          (Habit, StreakInfo, ALLOWED_COLORS)
    │   │   └── components/
    │   │       ├── Icon.svelte
    │   │       ├── CalendarGrid.svelte  (7×7 grid, keyboard nav)
    │   │       └── HabitRow.svelte
    │   └── routes/
    │       ├── +layout.svelte
    │       ├── +page.svelte         (dashboard)
    │       └── +page.server.ts      (load + create/toggle/remove actions)
    └── e2e/
        └── habits.spec.ts   (Playwright + axe + keyboard + reduced-motion)
```

## What to do next

1. Open `COMMANDS.md` and follow it top-to-bottom. The first run will install crates, scaffold the DB, build the Rust binary, install pnpm deps, and run all 45+ tests.
2. After everything is green, read `LESSON.md` for the line-by-line walkthrough — especially the `streaks.rs` proptest section and the SQL islands-and-gaps query.
3. Try this: in `streaks.rs`, remove the `if w[1] - w[0] == Duration::days(1)` condition and replace it with `if w[1] - w[0] <= Duration::days(2)`. Watch `current_equals_trailing_run_when_recent` find a counterexample within seconds. **This is what proptest is for.**
