# Project 03 — Commands

Run from the repo root unless noted. Same toolchain as projects 01 and 02.

## 1. Scaffold

```bash
mkdir -p projects/03-habits/backend/src/routes \
         projects/03-habits/backend/migrations \
         projects/03-habits/frontend/src/lib/components \
         projects/03-habits/frontend/src/routes \
         projects/03-habits/frontend/e2e
```

## 2. Backend — Cargo + DB

From `projects/03-habits/backend/`:

```bash
cd projects/03-habits/backend

# Create Cargo.toml, migrations/0001_init.sql, src/*.rs per LESSON.md section A.
export DATABASE_URL="sqlite://$(pwd)/habits.db"
sqlite3 habits.db < migrations/0001_init.sql
```

`sqlx::query!` introspects the DB at compile time, so the schema must exist before `cargo build`.

## 3. Backend — build + test

```bash
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                    # 19 tests pass (12 unit + 7 proptest)
```

The `proptest` lines look like:

```
test streaks::tests::longest_at_least_current ... ok
test streaks::tests::current_equals_trailing_run_when_recent ... ok
test streaks::tests::last_is_max ... ok
```

Each property runs 1024 randomly-generated cases. If any case finds a counterexample, proptest shrinks it to the smallest failing input and prints it. That output is gold for debugging.

## 4. Backend — run

```bash
cargo run --release      # listens on http://localhost:3002
```

Smoke-test from another terminal:

```bash
curl http://localhost:3002/healthz
# ok

curl -X POST http://localhost:3002/api/habits \
  -H 'content-type: application/json' \
  -d '{"name":"Read 30m","color":"#4F46E5"}'
# {"id":"...", "name":"Read 30m", "color":"#4F46E5", ..., "streak":{"current":0,"longest":0,"total":0,"last_completion":null}, "completions":[]}

# Save the id from above as HID, then toggle today's completion:
HID="paste-id-here"
TODAY=$(date -u +%Y-%m-%d)
curl -X POST "http://localhost:3002/api/habits/$HID/completions" \
  -H 'content-type: application/json' \
  -d "{\"date\":\"$TODAY\"}"
# {"completed":true,"streak":{"current":1,"longest":1,"total":1,"last_completion":"2026-..."},"completions":["2026-..."]}

# View the window-function streak history:
curl "http://localhost:3002/api/habits/$HID/streak-windows"
# [{"start":"2026-...","end":"2026-...","length":1}]
```

Leave the backend running.

## 5. Frontend

```bash
cd projects/03-habits/frontend
pnpm install
pnpm check            # 0 errors, 0 warnings
pnpm test:unit        # 6 tests pass
pnpm dev              # http://localhost:5173
```

Open the URL. Add a habit, pick a color, click cells in the calendar to toggle completions. Try keyboard navigation: focus the bottom-right (today) cell, then arrow keys to move, Space/Enter to toggle.

## 6. Playwright

```bash
# First time only — install Chromium for Playwright:
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium

# Backend must be running on :3002.
pnpm test:e2e         # 20 tests pass (5 specs × 4 viewports)
```

The 5 specs:

1. `home page is accessible (axe-core)` — WCAG 2 AA on `/`.
2. `create habit, toggle today, see current streak = 1, delete` — full lifecycle including the streak math.
3. `validation: empty habit name disables Add` — button stays disabled until name is non-empty.
4. `keyboard: arrow keys move focus across calendar grid` — verifies `ArrowUp/Down/Left/Right`, `Home`, `End` move focus per the grid layout (columns = weeks, rows = weekdays).
5. `respects prefers-reduced-motion (no cell transition)` — sets `reducedMotion: 'reduce'` on the browser context and asserts `transition-duration` resolves to `0s`.

## 7. Reset the DB

```bash
# Backend terminal: Ctrl+C, then:
cd projects/03-habits/backend
rm -f habits.db habits.db-shm habits.db-wal
sqlite3 habits.db < migrations/0001_init.sql
cargo run --release
```

## 8. Stop

```bash
# Ctrl+C in each running terminal.
```
