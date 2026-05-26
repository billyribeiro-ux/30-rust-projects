# Project 02 — Markdown Notes

> "I draft ideas in seven different apps and lose them all."

A tiny, fast Markdown notes app. Write in plain Markdown, save server-side after sanitization, share via clean URLs that rank well on Google.

This is the second of 30 projects in the curriculum. It introduces:

- **Postgres-grade safety on SQLite** — server-side Markdown rendering + HTML sanitization (`pulldown-cmark` + `ammonia`). The XSS lesson.
- **The full SEO baseline** — `<svelte:head>`, canonical URLs, OpenGraph, Twitter cards, JSON-LD (Article + WebApplication), `prerender = true` for the marketing page.
- **`<svelte:boundary>`** — per-route error containment so one broken note doesn't blank the whole page.
- **First `svelte/transition`** (`fade`) on the notes grid.
- **First a11y enforcement in CI** — `@axe-core/playwright` checks WCAG 2 AA on every page across all four viewports.
- **First custom `+error.svelte`** — a tailored 404 page that explains what happened.
- **Slug allocation with collision retry** — two notes called "ideas" get `/notes/ideas` and `/notes/ideas-1`.

Everything from Project 01 still applies (Svelte 5 runes, sqlx compile-time-checked queries, Axum 0.8, the 4-viewport Playwright matrix, etc.). This project layers new ideas on top of that foundation.

## Stack

- **Frontend** — SvelteKit 2 + Svelte 5 (runes), TypeScript strict (`noUncheckedIndexedAccess`, `noImplicitOverride`), plain CSS with cascade layers, Phosphor icons, `marked` for live preview only, `@axe-core/playwright` for a11y.
- **Backend** — Rust 2024 edition, Axum 0.8, SQLite via sqlx, `pulldown-cmark` for Markdown → HTML, `ammonia` for HTML sanitization.
- **Tests** — `cargo test` (14 tests, including XSS-stripping snapshots), Vitest (6 tests on the API client), Playwright (6 specs × 4 viewports = 24 runs, including axe-core a11y assertions on every visited page).

## Layout

```
projects/02-notes/
├── README.md          ← you are here
├── COMMANDS.md        ← every shell command you'll run
├── LESSON.md          ← line-by-line build walkthrough
├── backend/
│   ├── Cargo.toml
│   ├── migrations/0001_init.sql
│   └── src/
│       ├── main.rs      (Axum server, graceful shutdown)
│       ├── db.rs        (SQLite pool + migrations)
│       ├── error.rs     (typed errors → JSON responses)
│       ├── markdown.rs  (pulldown-cmark + ammonia + slugify)
│       └── routes/
│           ├── mod.rs
│           └── notes.rs  (CRUD + slug allocation + tests)
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
    │   │   ├── api.ts            (typed fetch client)
    │   │   ├── api.test.ts       (Vitest suite)
    │   │   ├── markdown.ts       (client-side preview only)
    │   │   ├── types.ts
    │   │   └── components/
    │   │       ├── Icon.svelte
    │   │       └── NoteCard.svelte
    │   └── routes/
    │       ├── +layout.svelte
    │       ├── +page.svelte               (notes grid)
    │       ├── +page.server.ts
    │       ├── about/
    │       │   ├── +page.ts               (prerender = true)
    │       │   └── +page.svelte
    │       └── notes/
    │           ├── new/
    │           │   ├── +page.svelte       (editor + preview)
    │           │   └── +page.server.ts
    │           └── [slug]/
    │               ├── +page.svelte       (renders sanitized HTML)
    │               ├── +page.server.ts
    │               └── +error.svelte      (404 page)
    └── e2e/
        └── notes.spec.ts   (Playwright + axe-core)
```

## What to do next

1. Open `COMMANDS.md` and follow every command top-to-bottom. The first time, type each one — don't paste — to build muscle memory.
2. When something compiles or a test passes, **read** `LESSON.md` for that section. Don't read ahead — the lesson assumes you've just seen the code work.
3. Break things on purpose. Remove the `ammonia` sanitization and watch the XSS test fail. Drop `<svelte:boundary>` and see what an in-page error looks like. Disable `prerender` on `/about` and inspect how the page is served. The understanding sticks when you've seen the wrong path.

When you finish: `git diff projects/02-notes`, push, then move on to project 03.
