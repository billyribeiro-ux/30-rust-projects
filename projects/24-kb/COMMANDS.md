# Commands — Project 24 (Hybrid Search KB)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Start Postgres (+ optional Meili)

```bash
cd projects/24-kb
docker compose up -d db    # mandatory
docker compose up -d meili # optional — for typo-tolerant fallback
```

## 3. Backend

```bash
cd backend
cp .env.example .env
# To enable Meili, set MEILI_URL and MEILI_KEY in .env. Backend
# degrades to FTS+trgm-only when blank.

sqlx migrate run --database-url "postgres://kb:kb_dev_password@localhost:5445/kb"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://kb:kb_dev_password@localhost:5445/kb" cargo test
DATABASE_URL="postgres://kb:kb_dev_password@localhost:5445/kb" cargo sqlx prepare

DATABASE_URL="postgres://kb:kb_dev_password@localhost:5445/kb" cargo run
# Backend listens on :3023.
```

## 4. Seed sample articles

```bash
psql "postgres://kb:kb_dev_password@localhost:5445/kb" <<'SQL'
INSERT INTO articles (slug, title, summary, body_md, body_html, published_at) VALUES
  ('postgres-tuning', 'Postgres tuning guide',
   'Top knobs that move the needle on a real workload.',
   '# Postgres tuning\n\nShared buffers, effective_cache_size, work_mem...',
   '<h1>Postgres tuning</h1><p>Shared buffers, effective_cache_size, work_mem...</p>',
   now()),
  ('kafka-intro', 'Kafka basics for beginners',
   'A 5-minute on-ramp.',
   '# Kafka\n\nProducers, consumers, topics, partitions.',
   '<h1>Kafka</h1><p>Producers, consumers, topics, partitions.</p>',
   now());
SQL
```

## 5. Frontend

```bash
cd projects/24-kb/frontend
pnpm install
pnpm check          # 0/0
pnpm test:unit      # 4 pass
pnpm dev            # http://localhost:5196
```

## 6. End-to-end

Backend must be running.

```bash
cd projects/24-kb/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3023 pnpm test:e2e
# 16/16 (4 specs × 4 viewports)
```

## 7. Optional — wire Meili

```bash
# Index articles into Meili.
ART_JSON=$(psql -t "postgres://kb:kb_dev_password@localhost:5445/kb" -c \
  "SELECT json_agg(row_to_json(a)) FROM (SELECT id, slug, title, summary FROM articles WHERE published_at IS NOT NULL) a;")
curl -X POST -H 'authorization: Bearer dev_master_key' \
  -H 'content-type: application/json' \
  -d "$ART_JSON" http://localhost:7700/indexes/articles/documents

# Set MEILI_URL + MEILI_KEY in .env and restart the backend.
```

## 8. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://kb:kb_dev_password@localhost:5445/kb"
```
