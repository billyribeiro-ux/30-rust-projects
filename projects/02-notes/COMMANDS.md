# Project 02 — Commands

Every shell command you'll need, in execution order. Run from the repo root (`30-rust-projects/`) unless noted.

## 0. Prerequisites

Same toolchain as Project 01. If you finished that project, you have everything:

```bash
rustc --version    # 1.94+
cargo --version
node --version     # v22+
pnpm --version     # v10+
sqlite3 --version  # 3.45+
```

If any are missing, install per Project 01's `COMMANDS.md` section 0.

## 1. Create the project skeleton

```bash
mkdir -p projects/02-notes/backend/src/routes \
         projects/02-notes/backend/migrations \
         projects/02-notes/frontend/src/lib/components \
         projects/02-notes/frontend/src/routes/about \
         projects/02-notes/frontend/src/routes/notes/new \
         projects/02-notes/frontend/src/routes/notes/'[slug]' \
         projects/02-notes/frontend/e2e
```

## 2. Backend — Cargo project

From `projects/02-notes/backend/`:

```bash
cd projects/02-notes/backend
```

Create `Cargo.toml`, `migrations/0001_init.sql`, and the `src/*.rs` files exactly as shown in `LESSON.md` section A.

Then create and bootstrap the DB:

```bash
export DATABASE_URL="sqlite://$(pwd)/notes.db"
sqlite3 notes.db < migrations/0001_init.sql
```

This step is required before the first `cargo build`: `sqlx::query!` macros introspect the schema at compile time, so the DB must exist with the migrated schema before Rust can compile.

## 3. Backend — build and test

```bash
cargo build              # downloads deps + compiles
cargo fmt --check        # style
cargo clippy --all-targets -- -D warnings   # zero warnings
cargo test               # 14 tests pass
```

Expected output of `cargo test`:

```
test result: ok. 14 passed; 0 failed; 0 ignored
```

## 4. Backend — run

```bash
# Still in projects/02-notes/backend/, with DATABASE_URL set
cargo run --release      # listens on http://localhost:3001
```

Smoke-test from another terminal:

```bash
curl http://localhost:3001/healthz
# ok

curl -X POST http://localhost:3001/api/notes \
  -H 'content-type: application/json' \
  -d '{"title":"Hello world","body_md":"# Hi\n\nThis is **markdown**."}'
# {"id":"...","slug":"hello-world","title":"Hello world","body_md":"# Hi\n\nThis is **markdown**.",
#  "body_html":"<h1>Hi</h1>\n<p>This is <strong>markdown</strong>.</p>\n","excerpt":"Hi This is markdown.",
#  "created_at":"...","updated_at":"..."}

curl http://localhost:3001/api/notes
# [ {... summary, no body_md/body_html ...} ]

# XSS test — paste a script tag in:
curl -X POST http://localhost:3001/api/notes \
  -H 'content-type: application/json' \
  -d '{"title":"Bad","body_md":"<script>alert(1)</script>hello"}'
# body_html will be: "<p>hello</p>"  ← ammonia stripped the script
```

Leave the backend running. Open another terminal for the frontend.

## 5. Frontend — install + first run

From `projects/02-notes/frontend/`:

```bash
cd projects/02-notes/frontend
pnpm install
```

Create every file from `LESSON.md` section B. Then:

```bash
pnpm check       # 0 errors, 0 warnings
pnpm test:unit   # 6 tests pass
pnpm dev         # http://localhost:5173 in dev
```

Open the URL. Try creating a note, click Preview to see the live markdown render, navigate to `/about` (prerendered marketing page), check `/notes/this-does-not-exist` (custom 404).

## 6. Playwright — production build + 24 tests

Playwright drives the *built* preview server (more like prod than `pnpm dev`):

```bash
# First time only — install Chromium for Playwright:
pnpm exec playwright install chromium

# Make sure the backend is running (step 4) so E2E tests can hit /api/notes.
pnpm test:e2e    # 24 tests pass (6 specs × 4 viewports)
```

The 6 specs:

1. `home page is accessible (axe-core)` — runs every WCAG 2 A + AA rule against `/`.
2. `about page is accessible (axe-core)` — same against the prerendered `/about`.
3. `full lifecycle: create, view, delete` — fills the form, asserts the rendered HTML matches expectations, runs axe on the note page, deletes via the confirm dialog.
4. `preview tab renders markdown without saving` — clicks the Preview tab and asserts the rendered `<h2>` shows.
5. `validation: empty title blocks save` — Save button is disabled when title is empty.
6. `unknown slug shows 404 page` — `/notes/no-such-thing` returns 404 with the custom error page.

## 7. Reset the DB (anytime)

The backend connection holds the file open; SQLite needs the server to be restarted after a destructive reset:

```bash
# In the backend terminal — Ctrl+C to stop, then:
cd projects/02-notes/backend
rm -f notes.db notes.db-shm notes.db-wal
sqlite3 notes.db < migrations/0001_init.sql
cargo run --release
```

## 8. Lighthouse check (manual)

With the production preview running, run Lighthouse from Chrome DevTools (Application → Lighthouse) against:

- `http://localhost:4174/` — expect Performance ≥ 95, A11y 100, SEO 100, Best Practices 100.
- `http://localhost:4174/about` — same but Performance should be ≥ 98 (prerendered).
- `http://localhost:4174/notes/<some-slug>` — same.

You don't need to commit this; it's a sanity check. Section LESSON D explains every number.

## 9. Stop everything

```bash
# Backend terminal: Ctrl+C
# Frontend: Ctrl+C  (if pnpm dev / preview running)
```

That's it. You shipped a Markdown app with server-side XSS sanitization, a prerendered marketing page, full SEO, custom 404, and CI-gated WCAG AA accessibility.

Now read `LESSON.md` to understand every line.
