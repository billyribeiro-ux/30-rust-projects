# Commands — Project 27 (Realtime Analytics)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Start Postgres

```bash
cd projects/27-analytics
docker compose up -d    # postgres on :5448
```

## 3. Backend

```bash
cd backend
cp .env.example .env

sqlx migrate run --database-url "postgres://analytics:analytics_dev_password@localhost:5448/analytics"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://analytics:analytics_dev_password@localhost:5448/analytics" cargo test
DATABASE_URL="postgres://analytics:analytics_dev_password@localhost:5448/analytics" cargo sqlx prepare

DATABASE_URL="postgres://analytics:analytics_dev_password@localhost:5448/analytics" cargo run
# Backend listens on :3026.
```

## 4. Frontend

```bash
cd projects/27-analytics/frontend
pnpm install
pnpm check        # 0/0
pnpm test:unit    # 4 pass
pnpm dev          # http://localhost:5199
```

## 5. E2E

```bash
cd projects/27-analytics/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3026 pnpm test:e2e
# 12/12 (3 specs × 4 viewports)
```

## 6. Send events

```bash
for i in $(seq 1 100); do
  curl -X POST http://localhost:3026/v1/ingest \
    -H 'content-type: application/json' \
    -d "{\"kind\":\"click\",\"session_id\":\"s$((RANDOM % 10))\",\"payload\":{\"i\":$i}}"
done
# Watch the dashboard at /dashboard — numbers roll in real time.
```

## 7. Upgrade to DuckDB (production path)

```toml
# Cargo.toml
duckdb = { version = "1", features = ["bundled"] }
```

```rust
// src/routes/kpis.rs
pub async fn compute_snapshot(_pool: &PgPool) -> AppResult<KpiSnapshot> {
    let conn = duckdb::Connection::open("events.duckdb")?;
    // ... same SQL, mostly the same shape.
}
```

The ETL (Postgres → DuckDB) is a separate worker task. Project 30
ships this in the capstone.

## 8. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://analytics:analytics_dev_password@localhost:5448/analytics"
```
