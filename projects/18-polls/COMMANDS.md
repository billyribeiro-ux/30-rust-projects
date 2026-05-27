# Project 18 — Commands

## Bootstrap

```bash
sudo -u postgres psql -c "CREATE USER polls WITH PASSWORD 'polls_dev_password';" 2>&1 || true
sudo -u postgres psql -c "CREATE DATABASE polls OWNER polls;" 2>&1 || true
sudo -u postgres psql -d polls -c "CREATE EXTENSION IF NOT EXISTS pgcrypto;"

cd backend
cp .env.example .env
# sqlx-cli (optional) — or just let `cargo run` apply migrations at startup:
PGPASSWORD=polls_dev_password psql -U polls -h localhost -d polls -f migrations/0001_init.sql
```

## Backend gates

```bash
cd backend
DATABASE_URL=postgres://polls:polls_dev_password@localhost:5432/polls cargo fmt --check
DATABASE_URL=postgres://polls:polls_dev_password@localhost:5432/polls cargo clippy --all-targets -- -D warnings
DATABASE_URL=postgres://polls:polls_dev_password@localhost:5432/polls cargo test
```

## Frontend gates

```bash
cd frontend
pnpm install
pnpm check
pnpm build
# Bring the backend up *with the e2e preview origin* before running tests:
( cd ../backend && \
  DATABASE_URL=postgres://polls:polls_dev_password@localhost:5432/polls \
  PORT=3016 FRONTEND_ORIGIN=http://localhost:4189 cargo run & )
pnpm test:e2e
```

## Live verify (manual smoke)

```bash
# In one shell, watch the SSE stream:
curl -N http://localhost:3016/api/polls/<slug>/stream
# In another, cast a vote — you should see a `counts` event appear immediately.
curl -X POST http://localhost:3016/api/polls/<slug>/vote \
  -H 'content-type: application/json' \
  -d '{"option_id":"<uuid>"}'
```
