# Commands — Project 22 (Background Jobs Dashboard)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Start Postgres

```bash
cd projects/22-jobs
docker compose up -d
# postgres on :5443
```

## 3. Backend

```bash
cd backend
cp .env.example .env

sqlx migrate run --database-url "postgres://jobs:jobs_dev_password@localhost:5443/jobs"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://jobs:jobs_dev_password@localhost:5443/jobs" cargo test
DATABASE_URL="postgres://jobs:jobs_dev_password@localhost:5443/jobs" cargo sqlx prepare

# Run the server + worker (port 3021).
DATABASE_URL="postgres://jobs:jobs_dev_password@localhost:5443/jobs" cargo run
```

## 4. Frontend

```bash
cd projects/22-jobs/frontend
pnpm install
pnpm check          # 0 errors, 0 warnings
pnpm test:unit      # 5 pass
pnpm dev            # http://localhost:5194
```

## 5. End-to-end

Backend must be running.

```bash
cd projects/22-jobs/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3021 pnpm test:e2e
# 12/12 (3 specs × 4 viewports)
```

## 6. Watch a job flow

```bash
# Get a session token.
curl -c cookies.txt -X POST http://localhost:3021/api/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"dev@example.com","password":"correct horse battery","name":"Dev"}'

# Enqueue 5 jobs in parallel — workers will pick them up via SKIP LOCKED.
for i in 1 2 3 4 5; do
  curl -b cookies.txt -X POST http://localhost:3021/api/admin/jobs \
    -H 'content-type: application/json' \
    -d "{\"kind\":\"resize_image\",\"payload\":{\"url\":\"/img/$i.png\"}}" &
done
wait

# Watch the live SSE feed.
curl -b cookies.txt -N http://localhost:3021/api/stream/jobs
```

## 7. Optional — OpenTelemetry tracing export

Set `OTEL_EXPORTER_OTLP_ENDPOINT` to a Tempo / Jaeger HTTP receiver
(`http://localhost:4318`) and rebuild. The `#[tracing::instrument]` on
the worker emits a span per job run.

## 8. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://jobs:jobs_dev_password@localhost:5443/jobs"
```
