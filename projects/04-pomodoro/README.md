# Project 04 — Pomodoro Timer + Session History

> "I lose focus and want enforced 25-minute blocks with a record."

A tiny Pomodoro timer with three modes (Focus, Short break, Long break), session history with stats, and an audio chime when each session completes. Keyboard-driven, motion-respectful, optimistic UI.

Fourth of 30 projects. New lessons:

- **`$effect` with cleanup** — the canonical RAF-animation pattern. Scheduling `requestAnimationFrame` inside an effect and returning a cleanup function that cancels it.
- **`requestAnimationFrame`** — smooth tick (vs `setInterval`, which drifts and pauses when the tab is backgrounded). Time is measured against `performance.now()`, not by incrementing a counter.
- **`untrack()`** — reading a `$state` variable inside an effect without making the effect depend on it. Required when the effect both reads and writes the same value (otherwise it loops).
- **Web Audio API** — synthesizing a chime tone (two sine waves with exponential decay) instead of shipping an audio file. Zero network cost.
- **`$inspect`** — live console logging of a reactive expression. The debugger you wanted but didn't know existed.
- **`<svelte:window>`** for global keyboard shortcuts (Space, R, 1/2/3).
- **Optimistic UI** — when a session finishes, we record it via a hidden form action; the chime + UI update happen instantly, server confirmation happens behind the scenes.

## Stack

- **Frontend** — SvelteKit 2 + Svelte 5 (runes), TypeScript strict, plain CSS with cascade layers, Phosphor icons. Web Audio API for the chime.
- **Backend** — Rust 2024 + Axum 0.8 + sqlx + SQLite. `kind` column has a `CHECK` constraint enforcing the three allowed values.
- **Tests** — `cargo test` (8 unit), Vitest (6 unit), Playwright (6 specs × 4 viewports = 24 runs).

## Layout

```
projects/04-pomodoro/
├── README.md / COMMANDS.md / LESSON.md
├── backend/
│   ├── Cargo.toml
│   ├── migrations/0001_init.sql      (sessions + CHECK on kind)
│   └── src/
│       ├── main.rs
│       ├── db.rs
│       ├── error.rs
│       └── routes/
│           ├── mod.rs
│           └── sessions.rs            (CRUD + stats endpoint)
└── frontend/
    ├── package.json / svelte.config.js / vite.config.ts / tsconfig.json / playwright.config.ts
    ├── src/
    │   ├── app.html / app.css / test-setup.ts
    │   ├── lib/
    │   │   ├── api.ts, api.test.ts
    │   │   ├── chime.ts           (Web Audio synthesized tone)
    │   │   ├── format.ts          (formatClock, formatMinutes, formatTimeOfDay)
    │   │   ├── types.ts           (SessionKind, KIND_LABEL/COLOR/SECONDS)
    │   │   └── components/
    │   │       ├── Icon.svelte
    │   │       └── SessionRow.svelte
    │   └── routes/
    │       ├── +layout.svelte
    │       ├── +page.svelte       (the timer)
    │       └── +page.server.ts    (load: sessions + stats; record/remove actions)
    └── e2e/pomodoro.spec.ts
```

## What to do next

1. Open `COMMANDS.md` and follow it top-to-bottom.
2. After everything is green, read `LESSON.md`. The `$effect` + RAF + `untrack` interplay is the most subtle topic in the whole curriculum so far — read it twice.
3. Try this: in `+page.svelte`, change `KIND_SECONDS.work` from `25 * 60` to `3` (three seconds), reload `/`, and click Start. Watch the timer tick to zero, hear the chime, see the session appear in history. The whole timer mechanism works at any time scale — the 25 minutes is just configuration.
