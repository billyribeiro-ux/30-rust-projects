# Project 04 — Commands

Same toolchain as projects 01–03. Backend on port 3003, frontend dev on 5176, Playwright preview on 4176.

## 1. Scaffold

```bash
mkdir -p projects/04-pomodoro/backend/src/routes \
         projects/04-pomodoro/backend/migrations \
         projects/04-pomodoro/frontend/src/lib/components \
         projects/04-pomodoro/frontend/src/routes \
         projects/04-pomodoro/frontend/e2e
```

## 2. Backend — create + build + test

```bash
cd projects/04-pomodoro/backend
# Write Cargo.toml, migrations/0001_init.sql, src/*.rs per LESSON.md section A.
export DATABASE_URL="sqlite://$(pwd)/pomodoro.db"
sqlite3 pomodoro.db < migrations/0001_init.sql
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test     # 8 tests pass
```

## 3. Backend — run + smoke

```bash
cargo run --release      # http://localhost:3003
```

```bash
curl http://localhost:3003/healthz
# ok

curl -X POST http://localhost:3003/api/sessions \
  -H 'content-type: application/json' \
  -d '{"kind":"work","label":"Read paper","planned_seconds":1500,"actual_seconds":1500,
       "started_at":"2026-05-24T10:00:00Z","ended_at":"2026-05-24T10:25:00Z"}'

curl http://localhost:3003/api/sessions/stats
# {"focus_seconds_today":1500,"focus_seconds_week":1500,"sessions_today":1,"pomodoros_today":1}
```

## 4. Frontend

```bash
cd projects/04-pomodoro/frontend
pnpm install
pnpm check          # 0/0
pnpm test:unit      # 6/6
pnpm dev            # http://localhost:5176
```

Click Start. The clock counts down with smooth motion (60 fps RAF). Try:
- **Space** — start/pause
- **R** — reset
- **1 / 2 / 3** — switch to Focus / Short break / Long break
- type in the label field — the keyboard shortcuts are correctly ignored while typing

Watch the browser console — you should see periodic `$inspect` lines logging `mode`, `running`, `remainingSec`. That's Svelte 5's `$inspect` rune doing its job. Remove the `$inspect` line in `+page.svelte` when you ship.

## 5. Playwright

```bash
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium  # first time only
pnpm test:e2e        # 24 tests pass (6 specs × 4 viewports)
```

The 6 specs:
1. `home page is accessible (axe-core)` — WCAG 2 AA across the whole page.
2. `initial state: 25:00 work timer, paused`.
3. `Space toggles start/pause; the label flips`.
4. `mode keys 1/2/3 switch the active tab and reset the clock`.
5. `recorded session shows in history; delete removes it` — seeds a session via `request.post` (the 25-minute live timer is not Playwright-friendly), reloads, asserts.
6. `respects prefers-reduced-motion (ring transition disabled)`.

## 6. Reset DB

```bash
# Ctrl+C the backend, then:
rm -f pomodoro.db pomodoro.db-shm pomodoro.db-wal
sqlite3 pomodoro.db < migrations/0001_init.sql
cargo run --release
```

## 7. Stop

Ctrl+C in each terminal.
