# Commands — Project 21 (Newsletter, Stripe Subscriptions)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
# Docker for postgres + mailhog (or system postgres + a local SMTP receiver)
```

## 2. Start Postgres + MailHog

```bash
cd projects/21-newsletter
docker compose up -d
# Postgres on :5442, MailHog SMTP :1026, UI :8026.
```

## 3. Backend

```bash
cd backend
cp .env.example .env
# In .env, set STRIPE_SECRET_KEY / STRIPE_WEBHOOK_SECRET / STRIPE_PRICE_ID
# from https://dashboard.stripe.com/test/apikeys. Or leave them at the
# sk_test_local default and use the test-mode signing helper in unit
# tests — the live Stripe API never gets called in CI.

sqlx migrate run --database-url "postgres://newsletter:newsletter_dev_password@localhost:5442/newsletter"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://newsletter:newsletter_dev_password@localhost:5442/newsletter" cargo test
DATABASE_URL="postgres://newsletter:newsletter_dev_password@localhost:5442/newsletter" cargo sqlx prepare

DATABASE_URL="postgres://newsletter:newsletter_dev_password@localhost:5442/newsletter" cargo run
# Backend listens on :3020.
```

## 4. Frontend

```bash
cd projects/21-newsletter/frontend
pnpm install
pnpm check          # 0 errors, 0 warnings
pnpm test:unit      # 5 pass
pnpm dev            # http://localhost:5193
```

## 5. End-to-end

Backend must be running.

```bash
cd projects/21-newsletter/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3020 pnpm test:e2e
# 16/16 (4 specs × 4 viewports)
```

## 6. Wiring the Stripe CLI for live webhook testing

```bash
# In one terminal:
stripe login
stripe listen --forward-to localhost:3020/api/stripe/webhook
# Copy the `whsec_…` it prints into backend/.env as STRIPE_WEBHOOK_SECRET.

# In another, simulate a payment:
stripe trigger checkout.session.completed
# Backend logs "subscriber promoted to pro".
```

## 7. Create the Pro plan in Stripe (one-time)

```
Dashboard → Products → New product
  Name: Newsletter Pro
  Pricing: Recurring, $5/month
  → Copy the price_… ID into STRIPE_PRICE_ID.
```

## 8. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://newsletter:newsletter_dev_password@localhost:5442/newsletter"
```
