# Commands — Project 20 (Geo-aware Restaurant Finder + OAuth)

Type these in order.

## 1. Prereqs

```bash
rustup default stable
corepack enable
corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
# Docker for postgis/postgis:16 — or install Postgres + postgresql-16-postgis-3 system-wide.
```

## 2. Start Postgres (with PostGIS)

```bash
cd projects/20-finder
docker compose up -d db
# Or, on host: postgres + `CREATE ROLE finder ... ; CREATE DATABASE finder OWNER finder;`
#              then `CREATE EXTENSION postgis;` in the finder db.
```

## 3. Backend

```bash
cd backend
cp .env.example .env
# Edit .env. To enable OAuth in dev, register OAuth apps at the provider consoles
# (see .env.example for callback URLs) and fill in GOOGLE_/GITHUB_ vars.

sqlx migrate run --database-url "postgres://finder:finder_dev_password@localhost:5441/finder"

# Optional: seed a few rows so the homepage isn't empty.
psql "postgres://finder:finder_dev_password@localhost:5441/finder" <<'SQL'
INSERT INTO places (name, cuisine, address, geom) VALUES
  ('Pad Thai Place', 'thai',     '101 Mott St, New York, NY',     ST_SetSRID(ST_MakePoint(-74.0,    40.72), 4326)::geography),
  ('Ramen House',    'japanese', '202 St Marks Pl, New York, NY', ST_SetSRID(ST_MakePoint(-73.987,  40.728),4326)::geography),
  ('Taqueria SF',    'mexican',  '300 Valencia St, San Francisco',ST_SetSRID(ST_MakePoint(-122.42,  37.766),4326)::geography);
SQL

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://finder:finder_dev_password@localhost:5441/finder" cargo test
DATABASE_URL="postgres://finder:finder_dev_password@localhost:5441/finder" cargo sqlx prepare

# Run the server (port 3019).
DATABASE_URL="postgres://finder:finder_dev_password@localhost:5441/finder" cargo run
```

## 4. Frontend

```bash
cd projects/20-finder/frontend
pnpm install
pnpm check          # 0 errors, 0 warnings
pnpm test:unit      # 6 pass
pnpm dev            # http://localhost:5192
```

## 5. End-to-end (Playwright)

Backend must be running, and **TEST_ONLY_TOKEN must be set on the backend** so
the e2e suite can inject sessions (this avoids driving a real OAuth flow).

```bash
# Terminal A — backend
cd projects/20-finder/backend
DATABASE_URL="postgres://finder:finder_dev_password@localhost:5441/finder" \
  TEST_ONLY_TOKEN=e2e-secret \
  FRONTEND_ORIGIN=http://localhost:4192 \
  cargo run

# Terminal B — e2e
cd projects/20-finder/frontend
npx playwright install chromium
TEST_ONLY_TOKEN=e2e-secret VITE_BACKEND_URL=http://localhost:3019 pnpm test:e2e
# 20/20 (5 specs × 4 viewports)
```

In production, leave `TEST_ONLY_TOKEN` **unset**. The route returns 404 to
every request when the env var is absent.

## 6. OAuth setup (Google + GitHub test creds)

Google:
1. Console → Create a project → OAuth 2.0 Client ID → "Web application".
2. Authorized redirect URI: `http://localhost:3019/api/auth/oauth/google/callback`.
3. Copy Client ID / Client Secret into `backend/.env`.

GitHub:
1. Settings → Developer settings → OAuth Apps → New OAuth App.
2. Authorization callback URL: `http://localhost:3019/api/auth/oauth/github/callback`.
3. Copy Client ID + Client Secret into `backend/.env`.

## 7. Reset DB

```bash
docker compose down -v && docker compose up -d db
cd backend && sqlx migrate run --database-url "postgres://finder:finder_dev_password@localhost:5441/finder"
```
