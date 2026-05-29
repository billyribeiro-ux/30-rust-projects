# Commands — Project 30 (SaaS Capstone)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Start Postgres + MailHog

```bash
cd projects/30-capstone
docker compose up -d    # postgres :5451, MailHog SMTP :1029, UI :8029
```

## 3. Backend

```bash
cd backend
cp .env.example .env

sqlx migrate run --database-url "postgres://capstone:capstone_dev_password@localhost:5451/capstone"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://capstone:capstone_dev_password@localhost:5451/capstone" cargo test
DATABASE_URL="postgres://capstone:capstone_dev_password@localhost:5451/capstone" cargo sqlx prepare

DATABASE_URL="postgres://capstone:capstone_dev_password@localhost:5451/capstone" cargo run
# Backend listens on :3029.
```

## 4. Frontend

```bash
cd projects/30-capstone/frontend
pnpm install
pnpm check        # 0/0
pnpm test:unit    # 4 pass
pnpm dev          # http://localhost:5202
```

## 5. E2E

```bash
cd projects/30-capstone/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3029 pnpm test:e2e
# 12/12 (3 specs × 4 viewports)
```

## 6. Demo the flow

```bash
# Sign up.
curl -c c.txt -X POST http://localhost:3029/api/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"founder@acme.com","password":"correct horse battery","name":"Founder"}'

# Create a workspace.
curl -b c.txt -X POST http://localhost:3029/api/tenants \
  -H 'content-type: application/json' \
  -d '{"slug":"acme","name":"Acme"}'

# Create a project under acme.
PROJ=$(curl -b c.txt -X POST http://localhost:3029/api/t/acme/projects \
  -H 'content-type: application/json' \
  -d '{"slug":"web","name":"Web"}' | jq -r .id)

# Add a task.
curl -b c.txt -X POST "http://localhost:3029/api/t/acme/projects/$PROJ/tasks" \
  -H 'content-type: application/json' \
  -d '{"title":"Ship it","body":"All of it."}'
```

## 7. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://capstone:capstone_dev_password@localhost:5451/capstone"
```
