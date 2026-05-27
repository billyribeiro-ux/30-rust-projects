# Project 14 — Commands

Ports: backend `3013`, frontend dev `5186`, Playwright preview `4186`.

## 1. Local infra

```bash
cd projects/14-calendar
docker compose up -d                # Postgres + MailHog
```

Postgres comes up as `calendar` / `calendar_dev_password` on database `calendar`.

## 2. Backend bootstrap

```bash
cd backend
cp .env.example .env
cargo run                           # listens on :3013
```

## 3. Frontend bootstrap

```bash
cd frontend
pnpm install
pnpm dev                            # http://localhost:5186
```

## 4. Smoke-test RRULE expansion

```bash
# Register + capture cookie
curl -s -c /tmp/c -H 'content-type: application/json' \
  http://localhost:3013/api/auth/register \
  -d '{"email":"a@x.com","password":"correct horse battery staple","name":"A"}'

# Create a calendar
CAL=$(curl -s -b /tmp/c -H 'content-type: application/json' \
  http://localhost:3013/api/calendars \
  -d '{"name":"My calendar","default_tz":"America/Los_Angeles"}')
CAL_ID=$(echo "$CAL" | python3 -c "import sys,json;print(json.load(sys.stdin)['id'])")

# Create a weekly recurring event (Monday, 8 occurrences)
curl -s -b /tmp/c -H 'content-type: application/json' \
  http://localhost:3013/api/events \
  -d "{\"calendar_id\":\"$CAL_ID\",\"title\":\"Weekly standup\",\"start_at\":\"2026-05-25T16:00:00Z\",\"end_at\":\"2026-05-25T16:30:00Z\",\"tz\":\"America/Los_Angeles\",\"rrule\":\"RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=8\"}" >/dev/null

# Range query expands recurrence in-window
curl -s -b /tmp/c "http://localhost:3013/api/events?from=2026-05-24T00:00:00Z&to=2026-06-22T00:00:00Z" \
  | python3 -m json.tool
```

## 5. Subscribe to ICS

```bash
# Add this URL to Apple Calendar / Google Calendar:
echo "http://localhost:3013/api/export/calendar/$CAL_ID"

# Inspect locally
curl -b /tmp/c "http://localhost:3013/api/export/calendar/$CAL_ID"
```

## 6. Quality gates

```bash
# Backend
cd backend
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                              # 7 tests

# Frontend
cd frontend
pnpm check                              # svelte-check, 0 errors
pnpm build                              # production build
pnpm test:e2e                           # Playwright, 4 viewports, 40 tests
```

## 7. Reset the database

```bash
psql postgres://calendar:calendar_dev_password@localhost:5432/calendar \
  -c "TRUNCATE events, calendar_shares, calendars, auth_tokens, sessions, users RESTART IDENTITY CASCADE;"
```

## 8. Commit + push

```bash
cd ../..
git add projects/14-calendar README.md
git commit -m "ship project 14: calendar & scheduler"
git push -u origin claude/fervent-mccarthy-2lbRU
```
