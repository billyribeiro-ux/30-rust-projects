# Commands — Project 25 (Course Marketplace, Stripe Connect)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Start Postgres

```bash
cd projects/25-marketplace
docker compose up -d db    # postgres on :5446
```

## 3. Backend

```bash
cd backend
cp .env.example .env
# Get STRIPE_SECRET_KEY from https://dashboard.stripe.com/test/apikeys.
# Real Connect testing also needs to enable Connect in your dashboard:
#   https://dashboard.stripe.com/connect/onboarding

sqlx migrate run --database-url "postgres://marketplace:marketplace_dev_password@localhost:5446/marketplace"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://marketplace:marketplace_dev_password@localhost:5446/marketplace" cargo test
DATABASE_URL="postgres://marketplace:marketplace_dev_password@localhost:5446/marketplace" cargo sqlx prepare

DATABASE_URL="postgres://marketplace:marketplace_dev_password@localhost:5446/marketplace" cargo run
# Backend listens on :3024.
```

## 4. Frontend

```bash
cd projects/25-marketplace/frontend
pnpm install
pnpm check          # 0/0
pnpm test:unit      # 4 pass
pnpm dev            # http://localhost:5197
```

## 5. E2E

```bash
cd projects/25-marketplace/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3024 pnpm test:e2e
# 12/12 (3 specs × 4 viewports)
```

## 6. Live Connect testing (the real thing)

```bash
# Listen for webhooks (in another terminal):
stripe listen --forward-to localhost:3024/api/stripe/webhook \
  --events checkout.session.completed,account.updated,charge.refunded
# Copy `whsec_…` into backend/.env as STRIPE_WEBHOOK_SECRET.

# Walk the flow:
# 1. Sign up at /signup
# 2. POST /api/instructor/me/onboard → visit the Stripe-hosted URL it returns
# 3. Complete the Stripe onboarding form (Stripe's test data is acceptable)
# 4. Watch `payouts_enabled` flip from FALSE → TRUE in the instructors table
# 5. As a separate student account, hit /c/<slug> and click "Buy course"
```

## 7. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://marketplace:marketplace_dev_password@localhost:5446/marketplace"
```
