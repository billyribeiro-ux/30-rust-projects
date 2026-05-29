# Commands — Project 28 (AI Inference API)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Start Postgres

```bash
cd projects/28-ai-api
docker compose up -d    # postgres on :5449
```

## 3. Backend

```bash
cd backend
cp .env.example .env

sqlx migrate run --database-url "postgres://aiapi:aiapi_dev_password@localhost:5449/aiapi"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://aiapi:aiapi_dev_password@localhost:5449/aiapi" cargo test
DATABASE_URL="postgres://aiapi:aiapi_dev_password@localhost:5449/aiapi" cargo sqlx prepare

DATABASE_URL="postgres://aiapi:aiapi_dev_password@localhost:5449/aiapi" cargo run
# Backend listens on :3027.
```

## 4. Frontend

```bash
cd projects/28-ai-api/frontend
pnpm install
pnpm check        # 0/0
pnpm test:unit    # 5 pass
pnpm dev          # http://localhost:5200
```

## 5. E2E

```bash
cd projects/28-ai-api/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3027 pnpm test:e2e
# 12/12 (3 specs × 4 viewports)
```

## 6. Try the inference endpoint

```bash
# 1. Sign up.
curl -c c.txt -X POST http://localhost:3027/api/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"dev@example.com","password":"correct horse battery","name":"Dev"}'

# 2. Create an API key.
SECRET=$(curl -b c.txt -X POST http://localhost:3027/api/keys \
  -H 'content-type: application/json' \
  -d '{"name":"first","rpm_limit":60}' | jq -r .secret)

# 3. Call inference.
curl -X POST http://localhost:3027/v1/inference \
  -H "authorization: Bearer $SECRET" \
  -H 'content-type: application/json' \
  -d '{"prompt":"hello"}'

# 4. See usage on the dashboard at /keys.
```

## 7. Wire Stripe Meter Events (production path)

```rust
// Background task — runs every N seconds.
let rows = sqlx::query!(
    "SELECT id, api_key_id, ts, units
     FROM usage_events
     WHERE reported_to_stripe_at IS NULL
     ORDER BY ts LIMIT 1000"
).fetch_all(&pool).await?;
for row in rows {
    let idem = format!("usage-{}", row.id);
    // POST https://api.stripe.com/v1/billing/meter_events
    //   event_name=inference_calls
    //   payload[stripe_customer_id]=<lookup from api_key>
    //   payload[value]=<row.units>
    //   timestamp=<row.ts as unix>
    //   Idempotency-Key: usage-<row.id>
    if ok {
        sqlx::query!("UPDATE usage_events SET reported_to_stripe_at = now()
                      WHERE id = $1", row.id).execute(&pool).await?;
    }
}
```

## 8. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://aiapi:aiapi_dev_password@localhost:5449/aiapi"
```
