# Project 19 — Kanban Issue Tracker (GSAP #1)

A Linear-style kanban: boards, lists, cards, comments, drag-and-drop between
lists, with **GSAP** drop physics that respect `prefers-reduced-motion`.
Membership-based RBAC enforces viewer / editor / admin at the route layer.

## What's new in this project

- **GSAP loaded client-side, lazy-imported.** Drop animations only run when
  the user hasn't opted out of motion. Server-side renders nothing GSAP-aware
  (the import is dynamic), so the first paint is unaffected.
- **Optimistic mutations with rollback.** Dropping a card mutates local state
  immediately, fires the PATCH, and reverts on server error. The user never
  waits on the network for a perceived response.
- **`animate:flip`** from `svelte/animate` for tactile reorders.
- **`{@attach}`-free drag handle** — Svelte 5's `{@attach}` is the modern
  action replacement, but the kanban drag is implemented with plain HTML5 DnD
  so it survives reduced-motion + assistive-tech contexts. We use GSAP only
  for the cinematic *settle* on drop, not for the drag itself.
- **Lexorank-ish positions** as `DOUBLE PRECISION`. New cards land at
  `(prev.position + next.position) / 2`. Rebalance is a future concern.
- **RBAC at the route layer.** Each mutating route runs `require_role` against
  the user's membership row; viewers get `403` and never see a state change.

## Stack delta vs project 14

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres, Argon2, sessions, Axum, sqlx | Adds the `lists`/`cards`/`comments` resources, `memberships` RBAC, lexorank positions |
| Frontend | hooks.server.ts auth, (auth)/(app) groups, Phosphor | Adds `gsap`, `animate:flip`, optimistic mutate-and-rollback in `KanbanBoard.svelte`, HTML5 drag-and-drop |
| Tests | Vitest + Playwright × 4 viewports + axe-core | Adds RBAC permission-matrix integration test (`tests/rbac_flow.rs`) |

## Layout

```
projects/19-kanban/
├── README.md
├── COMMANDS.md
├── LESSON.md
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml
│   ├── .env.example
│   ├── migrations/0001_init.sql
│   ├── tests/rbac_flow.rs            real HTTP + Postgres permission matrix
│   ├── .sqlx/                        offline query cache (committed)
│   └── src/
│       ├── main.rs                   port 3018, graceful shutdown
│       ├── lib.rs                    build_app(state, cors_origin)
│       ├── auth/{hash,session,rbac,mod}.rs
│       ├── routes/{auth,boards,lists,cards,comments,mod}.rs
│       ├── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json                  ports: dev 5191, preview 4191
    ├── playwright.config.ts          4-viewport matrix
    ├── e2e/kanban.spec.ts            5 specs × 4 viewports = 20 runs
    └── src/
        ├── app.{html,css,d.ts}
        ├── hooks.server.ts           lookups /api/auth/me on every request
        ├── lib/
        │   ├── api.ts, api.test.ts   typed HTTP client + 5 unit tests
        │   ├── types.ts
        │   ├── motion.ts             lazy GSAP, reduced-motion guard
        │   └── components/KanbanBoard.svelte   optimistic DnD + GSAP settle
        └── routes/
            ├── +layout.{svelte,server.ts}
            ├── (auth)/
            │   ├── +layout.{svelte,server.ts}
            │   ├── login/+page.{svelte,server.ts}
            │   └── signup/+page.{svelte,server.ts}
            └── (app)/
                ├── +layout.{svelte,server.ts}
                ├── +page.server.ts            → redirect to /boards
                ├── logout/+page.server.ts
                └── boards/
                    ├── +page.{svelte,server.ts}
                    └── [slug]/+page.{svelte,server.ts}
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | 6 unit + 3 integration = **9/9 pass** |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **5/5 pass** |
| `pnpm test:e2e` | **20/20 pass** (5 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean (one documented `state_referenced_locally` silenced — see LESSON §B) |

## What you can do now

Build kanban-style products that *feel* tactile without sacrificing
accessibility — and enforce permission boundaries in the right place
(route extractor, not "trust the UI").

## What's next

Project 20 adds OAuth 2.0 + PKCE and PostGIS for geo-aware search.
