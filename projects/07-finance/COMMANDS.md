# Project 07 — Commands

Ports: backend 3006, dev 5179, preview 4179.

```bash
mkdir -p projects/07-finance/{backend/src/routes,backend/migrations,frontend/src/lib/components,frontend/src/routes,frontend/e2e}

# Backend
cd projects/07-finance/backend
# Write Cargo.toml, migrations/0001_init.sql, src/{db,error,ledger,main}.rs + src/routes/*.rs
export DATABASE_URL="sqlite://$(pwd)/finance.db"
sqlite3 finance.db < migrations/0001_init.sql
cargo build
cargo clippy --all-targets -- -D warnings
cargo test     # 10/10
cargo run --release   # :3006
```

Smoke:

```bash
# Create accounts
CK=$(curl -s -X POST http://localhost:3006/api/accounts -H 'content-type: application/json' \
  -d '{"name":"Checking","kind":"asset","color":"#4F46E5"}' | jq -r .id)
GR=$(curl -s -X POST http://localhost:3006/api/accounts -H 'content-type: application/json' \
  -d '{"name":"Groceries","kind":"expense","color":"#DC2626"}' | jq -r .id)

# Spend $42.50 — Checking -4250, Groceries +4250
curl -X POST http://localhost:3006/api/transactions -H 'content-type: application/json' \
  -d "{\"description\":\"shopping\",\"postings\":[
    {\"account_id\":\"$CK\",\"amount_minor\":-4250},
    {\"account_id\":\"$GR\",\"amount_minor\":4250}
  ]}"

# Unbalanced transaction → 422 with field error
curl -X POST http://localhost:3006/api/transactions -H 'content-type: application/json' \
  -d "{\"postings\":[
    {\"account_id\":\"$CK\",\"amount_minor\":100},
    {\"account_id\":\"$GR\",\"amount_minor\":50}
  ]}"
# { "error": { "code": "validation_failed", "fields": [{"field":"postings","message":"debits must equal credits (sum: 150 minor units, must be 0)"}] } }

# CSV import (asset_account_name must match an existing asset account)
curl -X POST http://localhost:3006/api/imports -H 'content-type: application/json' \
  -d "{\"asset_account_name\":\"Checking\",\"csv\":\"date,description,amount,counterparty_account\\n2026-05-24,Groceries,-42.50,Groceries\\n\"}"
```

Frontend:

```bash
cd projects/07-finance/frontend
pnpm install
pnpm check        # 0/0
pnpm test:unit    # 3/3
pnpm dev          # http://localhost:5179
```

Playwright:

```bash
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium
pnpm test:e2e     # 20/20
```
