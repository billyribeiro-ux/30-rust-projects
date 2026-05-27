# Commands

Run these in order from a fresh clone.

## 0) Bring up services

```bash
cd projects/16-storefront
docker compose up -d db mailhog
# Wait for healthcheck (or just check):
docker compose exec db pg_isready -U storefront -d storefront
```

> The repo's CI runs against a system Postgres at `localhost:5432`. The
> docker-compose binds **5437** to host so it doesn't collide with a
> system PG. If you use the system one, set `DATABASE_URL=postgres://storefront:storefront_dev_password@localhost:5432/storefront`
> in `.env`.

## 1) Backend

```bash
cd backend
cp .env.example .env
# (optional, but recommended) override DATABASE_URL / DOWNLOAD_SIGNING_SECRET

# Run migrations, then build with offline sqlx data.
export DATABASE_URL=postgres://storefront:storefront_dev_password@localhost:5437/storefront
cargo sqlx migrate run                     # creates the schema
cargo sqlx prepare --check || cargo sqlx prepare   # regen .sqlx/ if needed
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Dev server (port 3015).
cargo run
```

### Reset the DB

```bash
docker compose exec db psql -U storefront -d postgres -c "DROP DATABASE storefront"
docker compose exec db psql -U storefront -d postgres -c "CREATE DATABASE storefront"
cargo sqlx migrate run
```

## 2) Frontend

```bash
cd ../frontend
pnpm install
pnpm check                # 0 errors, 0 warnings
pnpm test:unit            # vitest
pnpm dev                  # http://localhost:5188

# Production preview:
pnpm build && pnpm preview --port 4188
```

## 3) Playwright (e2e)

```bash
cd frontend
# One-time browser install:
npx playwright install chromium
# Backend must be running on :3015 for /api/* calls to succeed.
VITE_BACKEND_URL=http://localhost:3015 pnpm test:e2e
```

## 4) sqlx offline data

The repo commits `backend/.sqlx/`. Regenerate after a schema or query
change:

```bash
cd backend
DATABASE_URL=postgres://storefront:storefront_dev_password@localhost:5437/storefront \
  cargo sqlx prepare
git add .sqlx && git commit -m "sqlx prepare: <reason>"
```

CI builds with `SQLX_OFFLINE=true cargo build`, no DB needed.

## 5) Stripe walkthrough (informational — tests don't need it)

The integration tests use `wiremock` and never call Stripe. For a live
walkthrough:

```bash
# Once: install the Stripe CLI, then log in with a test-mode account.
stripe login

# Terminal A: backend
cd backend && cargo run

# Terminal B: forward webhooks to localhost:3015 and copy the printed
# whsec_... value into STRIPE_WEBHOOK_SECRET in .env (then restart backend).
stripe listen --forward-to http://localhost:3015/api/stripe/webhook

# Terminal C: trigger a test event
stripe trigger checkout.session.completed
```

You should see the webhook arrive, the event row appear in
`stripe_events`, the order flip to `'fulfilled'`, and the buyer email
land in MailHog at http://localhost:8025.

## 6) Common queries while debugging

```bash
docker compose exec db psql -U storefront storefront
```

```sql
-- All orders, newest first
SELECT id, customer_email, status, amount_cents, fulfilled_at, refunded_at
FROM orders ORDER BY created_at DESC LIMIT 20;

-- Live download links
SELECT order_id, product_id, expires_at, used_at, revoked_at
FROM download_links ORDER BY created_at DESC LIMIT 20;

-- Idempotency table
SELECT event_id, type, received_at FROM stripe_events ORDER BY received_at DESC LIMIT 20;
```
