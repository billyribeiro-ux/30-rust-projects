# Project 06 — Commands

Ports: backend 3005, dev 5178, preview 4178.

## Scaffold + backend

```bash
mkdir -p projects/06-splitter/{backend/src/routes,backend/migrations,frontend/src/lib/components,frontend/src/routes,frontend/e2e}
cd projects/06-splitter/backend
# Write Cargo.toml + migrations/0001_init.sql + src/{db,error,splitter,main}.rs + src/routes/*.rs
export DATABASE_URL="sqlite://$(pwd)/splitter.db"
sqlite3 splitter.db < migrations/0001_init.sql
cargo build
cargo clippy --all-targets -- -D warnings
cargo test     # 10/10 (7 unit + 3 proptest)
cargo run --release   # listens on :3005
```

## Smoke test

```bash
# Create two members
A=$(curl -s -X POST http://localhost:3005/api/members \
  -H 'content-type: application/json' \
  -d '{"name":"Alice","color":"#4F46E5"}' | jq -r .id)
B=$(curl -s -X POST http://localhost:3005/api/members \
  -H 'content-type: application/json' \
  -d '{"name":"Bob","color":"#059669"}' | jq -r .id)

# Alice pays $30 split equally
curl -X POST http://localhost:3005/api/expenses -H 'content-type: application/json' -d "{
  \"payer_id\":\"$A\",\"amount_cents\":3000,\"description\":\"dinner\",
  \"split_kind\":\"equal\",
  \"shares\":[{\"member_id\":\"$A\",\"value\":0},{\"member_id\":\"$B\",\"value\":0}]
}"

# Balances
curl http://localhost:3005/api/balances
# { "balances": [...], "settlements": [{"from":"Bob","to":"Alice","cents":1500}] }
```

## Frontend

```bash
cd projects/06-splitter/frontend
pnpm install
pnpm check        # 0/0
pnpm test:unit    # 14/14
pnpm dev          # http://localhost:5178
```

## Playwright

```bash
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium
pnpm test:e2e     # 16/16 (4 specs × 4 viewports)
```

Specs:
1. axe-core a11y on home page
2. Seed members + expense via API, see balance breakdown and suggested settlement
3. Submit button disabled when amount is zero
4. Equal split preview shows correct per-member amount + enables submit
