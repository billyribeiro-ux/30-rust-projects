# Project 11 — Commands

Ports: backend `3010`, frontend dev `5183`, Playwright preview `4183`. Postgres on `5432`, MailHog SMTP on `1025`, MailHog web UI on `8025`.

## 1. Local infra

```bash
cd projects/11-contacts
docker compose up -d                # Postgres + MailHog

# Sanity:
docker compose exec db pg_isready -U contacts -d contacts
open http://localhost:8025          # MailHog web UI
```

If you don't have Docker, install Postgres locally:

```bash
apt-get install -y postgresql-16   # Ubuntu/Debian
service postgresql start
sudo -u postgres psql -c "CREATE USER contacts WITH PASSWORD 'contacts_dev_password' CREATEDB;"
sudo -u postgres psql -c "CREATE DATABASE contacts OWNER contacts;"
```

## 2. Backend

```bash
cd backend
cp .env.example .env                # adjust if needed
export DATABASE_URL="postgres://contacts:contacts_dev_password@localhost:5432/contacts"

cargo build
cargo clippy --all-targets -- -D warnings
cargo test                          # 7/7 (Argon2 + email normalization)
cargo run --release                 # serves on :3010 (auto-runs sqlx migrate)
```

## 3. Smoke test

```bash
curl http://localhost:3010/healthz                  # ok

# Register
curl -s -i -X POST http://localhost:3010/api/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"alice@example.com","password":"correct horse battery staple","name":"Alice"}'
# → 201 with Set-Cookie: contacts_session=... HttpOnly SameSite=Lax Max-Age=2592000

# Grab the cookie value:
SESSION="..."

# Whoami
curl -s -b "contacts_session=$SESSION" http://localhost:3010/api/auth/me

# Create a contact with tags
curl -s -b "contacts_session=$SESSION" -X POST http://localhost:3010/api/contacts \
  -H 'content-type: application/json' \
  -d '{"name":"Bob","email":"bob@acme.com","company":"Acme","tags":["work","important"]}'

# Search via tsvector
curl -s -b "contacts_session=$SESSION" "http://localhost:3010/api/contacts?q=acme"

# Dashboard
curl -s -b "contacts_session=$SESSION" http://localhost:3010/api/dashboard/

# No cookie = 401
curl -s -i http://localhost:3010/api/contacts | head -1
```

The verify email lands at http://localhost:8025 (MailHog).

## 4. Frontend

```bash
cd projects/11-contacts/frontend
pnpm install
pnpm check                          # 0/0
pnpm dev                            # http://localhost:5183
```

Visit `/register`, create an account, you'll land on `/`. Add a contact, search, log out, log back in.

## 5. Playwright

```bash
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium
pnpm test:e2e                       # 28+ runs (7 specs × 4 viewports)
```

Specs include the **permission matrix tests**:
- user A cannot read/PATCH/DELETE user B's contact (always 404 — never confirms row exists)
- every protected endpoint returns 401 without a cookie
- wrong password returns generic 401 message
- axe-core a11y on login + dashboard

## 6. Reset DB

```bash
docker compose down -v              # drops the volume → totally clean
docker compose up -d
# Backend will re-run migrations on next start
```

Or without Docker:
```bash
PGPASSWORD=contacts_dev_password psql -h localhost -U contacts -d contacts \
  -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
# Restart backend; it re-runs migrations
```

## 7. Stop

`Ctrl+C` in each terminal. `docker compose down` to stop infra.
