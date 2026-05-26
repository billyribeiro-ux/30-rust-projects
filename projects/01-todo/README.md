# Project 01 — TODO Manager

> "I forget what I need to do today."

A tiny, fast, distraction-free TODO app. Add a task, check it off, delete it. That's the whole product. Nothing else. The whole point of project 1 is to build every layer of the stack — frontend, backend, database, tests — at its smallest possible size, so we never have to learn two new things at once again.

## What you'll build

- A SvelteKit page with a text input, an "Add" button, and a list of tasks.
- Each task has a checkbox button (toggles done/not-done) and a trash button (deletes).
- A header shows "N of M remaining" and updates live.
- The whole thing is mobile-first responsive (390 → 1440 px).
- An Axum REST API: `GET /api/todos`, `POST /api/todos`, `PATCH /api/todos/:id`, `DELETE /api/todos/:id`.
- A SQLite database with a forward migration.
- Compile-time-checked SQL via `sqlx::query!` — typos in SQL break the build, not production.
- Tests: backend unit tests (`cargo test`), frontend unit tests (Vitest), end-to-end tests (Playwright at 4 viewports).

## Stack

- **Frontend** — SvelteKit 2, Svelte 5 (runes), TypeScript strict, plain CSS, Phosphor icons.
- **Backend** — Rust 2024, Axum 0.8, tokio, sqlx 0.8 (SQLite).
- **Tests** — `cargo test`, Vitest (jsdom), Playwright (chromium @ 390/768/1024/1440).

## Run it

If you just want to see it work, the short version:

```bash
# Terminal 1 — backend
cd backend
sqlite3 todo.db < migrations/0001_init.sql
DATABASE_URL=sqlite:./todo.db cargo run

# Terminal 2 — frontend
cd frontend
pnpm install
pnpm dev --open
```

For the full step-by-step (recommended your first time), open [`COMMANDS.md`](./COMMANDS.md). Every shell command from `mkdir` to `git push` is there.

For the line-by-line teaching — what every file does and *why* — open [`LESSON.md`](./LESSON.md).

## Folder map

```
01-todo/
├── README.md            ← you are here
├── COMMANDS.md          ← every command in order
├── LESSON.md            ← line-by-line teaching
├── backend/
│   ├── Cargo.toml
│   ├── migrations/
│   │   └── 0001_init.sql
│   └── src/
│       ├── main.rs       ← Axum bootstrap + router
│       ├── db.rs         ← SQLite pool + migration runner
│       ├── error.rs      ← AppError → JSON response
│       └── routes/
│           ├── mod.rs
│           └── todos.rs  ← list / create / update / delete handlers
└── frontend/
    ├── package.json
    ├── svelte.config.js
    ├── vite.config.ts
    ├── playwright.config.ts
    ├── tsconfig.json
    ├── e2e/
    │   └── todos.spec.ts
    └── src/
        ├── app.css
        ├── app.html
        ├── app.d.ts
        ├── test-setup.ts
        ├── lib/
        │   ├── types.ts
        │   ├── api.ts
        │   ├── api.test.ts
        │   └── components/
        │       ├── Icon.svelte
        │       └── TodoItem.svelte
        └── routes/
            ├── +layout.svelte
            ├── +page.svelte
            └── +page.server.ts
```

## What we deliberately did NOT do

Restraint is half of senior engineering. Project 1 ships **the smallest version that solves the problem**:

- ❌ No auth — added in project 11.
- ❌ No optimistic UI — added in project 14 (Kanban). Form actions + `invalidateAll()` are simpler and correct.
- ❌ No remote functions — added in project 8 (recipes). Form actions teach the request/response cycle first.
- ❌ No real-time / WebSockets — added in project 13 (chat).
- ❌ No animations beyond CSS transitions — GSAP starts at project 14.

Every "missing" feature is on the curriculum. We add them when they are the lesson, not as decoration.
