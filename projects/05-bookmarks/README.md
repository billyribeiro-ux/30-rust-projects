# Project 05 — Bookmark Manager with Tags

> "Browser bookmarks are unsearchable; I save links and never see them again."

A tiny bookmark manager. Add a link with a title, description, and free-form tags. Search across title/description/URL. Click a tag in the sidebar to filter. The URL captures the filter state, so back/forward and shareable links just work.

Fifth of 30 projects. New lessons:

- **Many-to-many SQL** — `bookmarks ↔ tags` via a `bookmark_tags` junction table with composite primary key. `GROUP_CONCAT` inlines each bookmark's tag list. JOIN-then-WHERE for tag filtering. `INSERT OR IGNORE` for idempotent upserts. All wrapped in a transaction (`pool.begin()` + `tx.commit()`).
- **URL-as-state filters** — `?q=axum&tag=rust` is the source of truth. Typing in the search box debounces and calls `goto()` to update the URL. Clicking a tag in the sidebar updates the URL. SvelteKit re-runs `load`, and `data.q`/`data.tag` flow back into the UI. Browser back/forward navigation is free.
- **Debounced search** with a typed `debounce()` helper (Vitest tests use `vi.useFakeTimers()` to verify the coalescing behavior).
- **`use:clickOutside` action** — the first custom Svelte action. Listens for `pointerdown`/`focusin` outside the node, fires a callback. Used to auto-close the "Add bookmark" panel.
- **`data-sveltekit-preload-data="off"`** on external links — we opt them out of SvelteKit's hover-preload (which would otherwise try to preload a remote site and fail CORS).

## Stack

- **Frontend** — SvelteKit 2 + Svelte 5, plain CSS with cascade layers, Phosphor icons. Custom `clickOutside` action. Vitest tests both the API client and the debounce helper.
- **Backend** — Rust 2024 + Axum 0.8 + sqlx + SQLite. `url` crate for URL validation. Tag normalization (lowercase, alphanumerics + `-_`, max 40 chars, max 20 per bookmark).
- **Tests** — `cargo test` (9 unit), Vitest (8 unit across `api.test.ts` + `debounce.test.ts`), Playwright (5 specs × 4 viewports = 20 runs).

## Layout

```
projects/05-bookmarks/
├── README.md / COMMANDS.md / LESSON.md
├── backend/
│   ├── Cargo.toml                     (+ url crate)
│   ├── migrations/0001_init.sql       (bookmarks, tags, bookmark_tags + indexes)
│   └── src/
│       ├── main.rs / db.rs / error.rs
│       └── routes/
│           ├── mod.rs
│           ├── bookmarks.rs            (CRUD + 4-way filter SQL match)
│           └── tags.rs                 (tag list with bookmark counts)
└── frontend/
    ├── package.json / svelte.config.js / vite.config.ts / tsconfig.json / playwright.config.ts
    ├── src/
    │   ├── app.html / app.css / test-setup.ts
    │   ├── lib/
    │   │   ├── api.ts, api.test.ts
    │   │   ├── debounce.ts, debounce.test.ts   (typed debounce with cancel)
    │   │   ├── actions.ts                       (use:clickOutside)
    │   │   ├── types.ts
    │   │   └── components/
    │   │       ├── Icon.svelte
    │   │       ├── BookmarkCard.svelte
    │   │       └── TagSidebar.svelte
    │   └── routes/
    │       ├── +layout.svelte
    │       ├── +page.svelte       (search, sidebar, grid, add panel)
    │       └── +page.server.ts    (load with q/tag from URL; create/remove actions)
    └── e2e/bookmarks.spec.ts
```

## What to do next

1. Open `COMMANDS.md` and follow it top-to-bottom.
2. Read `LESSON.md` for the SQL match-on-(q,tag) lesson and the URL-as-state pattern.
3. Try this: paste a URL with a `javascript:` scheme into the Add panel — the backend rejects it with HTTP 422 and the form shows the error. That's `normalize_url()` doing its job. Then paste a `ftp://` URL — same.
