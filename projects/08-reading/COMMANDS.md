# Project 08 — Commands

Ports: backend 3007, dev 5180, preview 4180.

```bash
mkdir -p projects/08-reading/{backend/src/routes,backend/migrations,backend/tests,frontend/src/lib/components,frontend/src/routes/books/'[id]',frontend/e2e}

cd projects/08-reading/backend
# Write Cargo.toml, migrations/0001_init.sql, src/{db,error,openlibrary,state,main}.rs + src/routes/*.rs + tests/openlibrary_wiremock.rs
export DATABASE_URL="sqlite://$(pwd)/reading.db"
sqlite3 reading.db < migrations/0001_init.sql
cargo build
cargo clippy --all-targets -- -D warnings
cargo test     # 13/13 (11 unit + 2 wiremock integration)
cargo run --release   # :3007
```

Smoke:

```bash
# Manual book
curl -X POST http://localhost:3007/api/books -H 'content-type: application/json' \
  -d '{"title":"Project Hail Mary","author":"Andy Weir","status":"reading","pages":476}'

# ISBN lookup (hits Open Library)
curl http://localhost:3007/api/lookup/isbn/9780553418026

# Patch status to finished — sets finished_at automatically
ID=...
curl -X PATCH http://localhost:3007/api/books/$ID -H 'content-type: application/json' \
  -d '{"status":"finished","current_page":476}'
```

Frontend:

```bash
cd projects/08-reading/frontend
pnpm install
pnpm check        # 0/0
pnpm test:unit    # 5/5
pnpm dev          # http://localhost:5180
```

Playwright:

```bash
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium
pnpm test:e2e     # 20/20 (5 specs × 4 viewports)
```
