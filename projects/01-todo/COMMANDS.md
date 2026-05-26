# Project 01 — Commands

Every shell command you need to reproduce this project, in order. Copy-paste your way to a running app. Each block has a one-line note above it explaining what it does.

> **Prerequisites** (once per machine — skip if you've already done them):
>
> - Rust toolchain (`rustup`)
> - Node 22 LTS (any installer)
> - `corepack enable && corepack use pnpm@latest`
> - `sqlite3` CLI (`apt install sqlite3` on Debian/Ubuntu, `brew install sqlite` on macOS)
> - `cargo install sqlx-cli --no-default-features --features rustls,sqlite,postgres`

---

## 1. Create the project folders

Inside the monorepo root (`30-rust-projects/`), make the project folder and its two halves.

```bash
mkdir -p projects/01-todo/backend/src/routes
mkdir -p projects/01-todo/backend/migrations
mkdir -p projects/01-todo/frontend
cd projects/01-todo
```

## 2. Scaffold the backend (Rust)

Initialize a new Rust binary crate.

```bash
cd backend
cargo init --name todo-backend --bin .
```

Add every backend dependency in one shot. Read the LESSON.md to learn what each one does.

```bash
cargo add axum --features macros
cargo add tokio --features rt-multi-thread,macros,signal,net
cargo add tower-http --features cors,trace,set-header
cargo add sqlx --no-default-features --features runtime-tokio,tls-rustls,sqlite,macros,migrate,chrono
cargo add serde --features derive
cargo add serde_json
cargo add thiserror
cargo add tracing
cargo add tracing-subscriber --features env-filter,json
cargo add chrono --features serde
cargo add uuid --features v4,serde
cargo add --dev reqwest --features json
```

## 3. Create the database and apply the migration

`sqlx::query!` checks SQL at **compile time** against a real database. So the DB must exist before we can compile.

```bash
# inside backend/
sqlite3 todo.db < migrations/0001_init.sql
```

(The migration file `migrations/0001_init.sql` is created in the LESSON — see step 4 of the LESSON walkthrough.)

## 4. Compile-check the backend

This will fail until you've written the source files from the LESSON. After you have, this is the loop you'll run constantly.

```bash
DATABASE_URL=sqlite:./todo.db cargo check
```

Run the unit tests.

```bash
DATABASE_URL=sqlite:./todo.db cargo test
```

## 5. Run the backend

```bash
DATABASE_URL=sqlite:./todo.db cargo run
# → server listening at 0.0.0.0:3000
```

Smoke-test from another terminal.

```bash
curl http://localhost:3000/healthz
# → ok

curl -X POST http://localhost:3000/api/todos \
  -H 'content-type: application/json' \
  -d '{"title":"buy milk"}'
# → {"id":"...","title":"buy milk","done":false,...}

curl http://localhost:3000/api/todos
# → [{"id":"...","title":"buy milk","done":false,...}]
```

## 6. Scaffold the frontend (SvelteKit)

In a **new terminal**, from the project root.

```bash
cd projects/01-todo
pnpm dlx sv@latest create --template minimal --types ts --no-add-ons --no-install frontend
cd frontend
```

Swap `adapter-auto` for `adapter-node` (we deploy to Node), and add the icon library.

```bash
pnpm remove @sveltejs/adapter-auto
pnpm add -D @sveltejs/adapter-node @types/node
pnpm add phosphor-svelte
```

Add testing dependencies.

```bash
pnpm add -D vitest @vitest/browser playwright @playwright/test \
            @testing-library/svelte @testing-library/jest-dom jsdom
pnpm exec playwright install chromium
```

Install everything.

```bash
pnpm install
```

## 7. Type-check the frontend

After you've written the source files from the LESSON.

```bash
pnpm check
# → 0 errors, 0 warnings
```

Run the unit tests.

```bash
pnpm test:unit
```

## 8. Run the frontend in dev mode

```bash
pnpm dev --open
# → http://localhost:5173 opens in your browser
```

(Backend must still be running in the other terminal.)

## 9. Run the end-to-end tests

This builds the frontend, starts a preview server on port 4173, and runs Playwright at 4 viewport sizes (mobile-390, tablet-768, laptop-1024, desktop-1440). The backend must be on port 3000.

```bash
pnpm test:e2e
# → 8 passed (4 viewports × 2 tests)
```

## 10. Production build

```bash
pnpm build
node build/index.js
# → SvelteKit preview at http://localhost:3000 (or PORT env)
```

## 11. Commit and push

From the monorepo root.

```bash
cd ../../..   # back to 30-rust-projects/

git add projects/01-todo shared/design-tokens.css \
        README.md CURRICULUM.md .gitignore .editorconfig

git commit -m "feat(01-todo): first project — full stack, tested, responsive"

git push -u origin claude/fervent-mccarthy-2lbRU
```

## Cleanup

To wipe the database and start fresh:

```bash
cd backend
rm -f todo.db todo.db-journal todo.db-shm todo.db-wal
sqlite3 todo.db < migrations/0001_init.sql
```

To reset node_modules:

```bash
cd frontend
rm -rf node_modules .svelte-kit
pnpm install
```
