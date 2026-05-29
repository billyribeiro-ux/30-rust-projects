# Commands — Project 19 (Kanban)

Type these in order. Each is idempotent unless noted.

## 1. Prereqs

```bash
# Rust toolchain
rustup default stable

# pnpm (via Corepack)
corepack enable
corepack use pnpm@latest

# sqlx-cli (compile-time SQL checking + migrations)
cargo install sqlx-cli --no-default-features --features rustls,postgres

# Docker (for Postgres) — or use a system Postgres + create role/db
#   sudo -u postgres psql -c "CREATE ROLE kanban WITH LOGIN PASSWORD 'kanban_dev_password';"
#   sudo -u postgres psql -c "CREATE DATABASE kanban OWNER kanban;"
```

## 2. Start Postgres

```bash
cd projects/19-kanban
docker compose up -d db
# Or, on a host without Docker, point DATABASE_URL at any postgres 14+ instance.
```

## 3. Backend

```bash
cd backend
cp .env.example .env
# Edit .env if your DB lives elsewhere.

# Run migrations.
sqlx migrate run --database-url "postgres://kanban:kanban_dev_password@localhost:5440/kanban"

# Compile + sanity check.
SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Run integration tests (needs DATABASE_URL).
DATABASE_URL="postgres://kanban:kanban_dev_password@localhost:5440/kanban" cargo test

# Refresh the offline query cache (commit `.sqlx/`).
DATABASE_URL="postgres://kanban:kanban_dev_password@localhost:5440/kanban" cargo sqlx prepare

# Start the server (port 3018).
DATABASE_URL="postgres://kanban:kanban_dev_password@localhost:5440/kanban" cargo run
```

## 4. Frontend

```bash
cd projects/19-kanban/frontend
pnpm install
pnpm check          # 0 errors, 0 warnings
pnpm test:unit      # 5 pass
pnpm dev            # http://localhost:5191
```

## 5. End-to-end (Playwright)

Backend must be running first.

```bash
cd projects/19-kanban/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3018 pnpm test:e2e
# 20/20 (5 specs × 4 viewports — mobile-portrait-390, tablet-768, laptop-1024, desktop-1440)
```

## 6. Reset the database (drop & re-migrate)

```bash
docker compose down -v && docker compose up -d db
# Or on system Postgres:
#   sudo -u postgres psql -c "DROP DATABASE kanban;" \
#     -c "CREATE DATABASE kanban OWNER kanban;"
cd backend
sqlx migrate run --database-url "postgres://kanban:kanban_dev_password@localhost:5440/kanban"
```

## 7. Production build

```bash
cd frontend
pnpm build
PORT=4191 ORIGIN=http://localhost:4191 node build
```
