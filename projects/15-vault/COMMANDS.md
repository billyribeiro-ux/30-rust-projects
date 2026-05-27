# Project 15 — every command, in order

## Backend

```bash
cd projects/15-vault

# Start postgres (5436) and optionally MinIO (9011/9012).
docker compose up -d db          # MinIO is optional — local FS is the default

cd backend
cp .env.example .env
# Edit .env if your postgres port or storage backend differs.

# Build / smoke
cargo build
DATABASE_URL=postgres://vault:vault_dev_password@localhost:5436/vault \
    cargo run                    # listens on 0.0.0.0:3014
curl http://localhost:3014/healthz   # "ok"

# Tests (the macros need a live DB to type-check queries)
DATABASE_URL=postgres://vault:vault_dev_password@localhost:5436/vault \
    cargo test
DATABASE_URL=postgres://vault:vault_dev_password@localhost:5436/vault \
    cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Refresh the offline sqlx cache after schema/query changes
DATABASE_URL=postgres://vault:vault_dev_password@localhost:5436/vault \
    cargo sqlx prepare
git add .sqlx/
```

## Frontend

```bash
cd projects/15-vault/frontend
pnpm install
pnpm check                       # 0 errors 0 warnings
pnpm test:unit                   # vitest unit tests

# Dev server (needs backend on :3014)
pnpm dev                         # http://localhost:5187

# Preview build (used by Playwright's webServer)
pnpm build
pnpm preview                     # http://localhost:4187

# E2E across 4 viewports — backend must be on :3014
pnpm test:e2e
```

## Reset DB during development

```bash
docker compose down -v && docker compose up -d db
# Sqlx migrations re-apply on the next `cargo run`.
```

## Verify an upload deduplicates

```bash
COOKIE='...'  # value from /api/auth/register's Set-Cookie

# Two POSTs with the same bytes:
PAYLOAD=$(printf 'identical bytes %s' "$(date +%s)")
for name in a.txt b.txt; do
    UID=$(curl -s -X POST http://localhost:3014/api/uploads \
        -H "cookie: app_session=$COOKIE" -H 'content-type: application/json' \
        -d "{\"filename\":\"$name\",\"size\":${#PAYLOAD},\"folder_id\":null,\"content_type\":\"text/plain\"}" \
        | jq -r .upload_id)
    curl -s -X PATCH http://localhost:3014/api/uploads/$UID \
        -H "cookie: app_session=$COOKIE" \
        -H 'upload-offset: 0' --data-binary "$PAYLOAD" -o /dev/null
done

# Check the file_versions table — should be ONE row, not two:
PGPASSWORD=vault_dev_password psql -h localhost -p 5436 -U vault -d vault \
    -c 'SELECT sha256, size FROM file_versions ORDER BY created_at DESC LIMIT 2;'
```
