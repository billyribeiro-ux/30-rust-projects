# Commands — Project 23 (Multi-tenant Help Desk + RLS)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Start Postgres + MailHog

```bash
cd projects/23-helpdesk
docker compose up -d
# Postgres on :5444, MailHog SMTP :1027, UI :8027.
```

## 3. Backend

```bash
cd backend
cp .env.example .env

sqlx migrate run --database-url "postgres://helpdesk:helpdesk_dev_password@localhost:5444/helpdesk"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://helpdesk:helpdesk_dev_password@localhost:5444/helpdesk" cargo test
DATABASE_URL="postgres://helpdesk:helpdesk_dev_password@localhost:5444/helpdesk" cargo sqlx prepare

DATABASE_URL="postgres://helpdesk:helpdesk_dev_password@localhost:5444/helpdesk" cargo run
# Backend listens on :3022.
```

## 4. Frontend

```bash
cd projects/23-helpdesk/frontend
pnpm install
pnpm check          # 0 errors, 0 warnings
pnpm test:unit      # 5 pass
pnpm dev            # http://localhost:5195
```

## 5. End-to-end

Backend must be running.

```bash
cd projects/23-helpdesk/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3022 pnpm test:e2e
# 12/12 (3 specs × 4 viewports)
```

## 6. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://helpdesk:helpdesk_dev_password@localhost:5444/helpdesk"
```

## 7. Verify RLS manually

```bash
psql "postgres://helpdesk:helpdesk_dev_password@localhost:5444/helpdesk"
SELECT * FROM tickets; -- empty: no app.tenant_id set, RLS hides everything
SET LOCAL app.tenant_id = '00000000-0000-0000-0000-000000000000';
SELECT * FROM tickets; -- still empty for that tenant
```
