# Project 17 — Commands

Cheatsheet for local development of the shortener.

## One-time setup

```bash
# Postgres
sudo -u postgres psql -c "CREATE USER shortener WITH PASSWORD 'shortener_dev_password';"
sudo -u postgres psql -c "CREATE DATABASE shortener OWNER shortener;"

# Backend env
cd backend && cp .env.example .env

# Redis (optional; the app degrades gracefully if missing)
service redis-server start   # or: docker run -d -p 6379:6379 redis:7-alpine
```

## Backend

```bash
cd backend
# Run dev server (migrations apply automatically on boot)
DATABASE_URL=postgres://shortener:shortener_dev_password@localhost:5432/shortener cargo run

# Quality gates
cargo fmt --check
DATABASE_URL=postgres://shortener:shortener_dev_password@localhost:5432/shortener cargo clippy --all-targets -- -D warnings
DATABASE_URL=postgres://shortener:shortener_dev_password@localhost:5432/shortener cargo test
```

## Frontend

```bash
cd frontend
pnpm install
pnpm dev          # http://localhost:5188
pnpm check        # svelte-kit sync + svelte-check
pnpm build
pnpm test:e2e     # boots `pnpm build && pnpm preview` and runs Playwright
```

## Live smoke test (no UI)

```bash
# 1. Register a user
EMAIL="smoke-$(date +%s)@example.com"
curl -s -c /tmp/c.txt -H 'content-type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"correct horse battery staple\"}" \
  http://localhost:3015/api/auth/register | jq

# 2. Create a link with a custom slug
curl -s -b /tmp/c.txt -H 'content-type: application/json' \
  -d '{"target_url":"https://example.com","slug":"hello"}' \
  http://localhost:3015/api/links | jq

# 3. Hit the redirect (real browser UA)
curl -i -A "Mozilla/5.0 Safari/605.1.15" http://localhost:3015/hello

# 4. Hit it as a bot (still 307, but no click recorded)
curl -i -A "curl/8.4.0" http://localhost:3015/hello

# 5. Check the live counter directly
redis-cli GET clicks:hello:total

# 6. Fetch stats
curl -s -b /tmp/c.txt http://localhost:3015/api/links/hello/stats | jq
```

## Reset the database

```bash
PGPASSWORD=shortener_dev_password psql -U shortener -h localhost -d shortener \
  -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
# Then re-run `cargo run` to apply migrations.
```
