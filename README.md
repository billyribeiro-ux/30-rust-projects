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

- [x] **01 — TODO Manager** — first runes, first Axum handler, first sqlx query.
- [ ] 02 — Markdown Notes
- [ ] ...

(See `CURRICULUM.md` for the full list.)
