# Project 09 — Commands

Ports: backend 3008, dev 5181, preview 4181.

## Scaffold + backend

```bash
mkdir -p projects/09-workouts/{backend/src/routes,backend/migrations,backend/tests,frontend/src/lib/components,frontend/src/routes/workouts/'[id]',frontend/src/routes/workouts/new,frontend/src/routes/exercises,frontend/e2e}

cd projects/09-workouts/backend
# Write Cargo.toml, migrations/0001_init.sql,
# src/{db,error,prs,state,main}.rs + src/routes/*.rs
export DATABASE_URL="sqlite://$(pwd)/workouts.db"
sqlite3 workouts.db < migrations/0001_init.sql
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test       # 20/20 (15 unit + 5 proptest properties)
cargo run --release   # :3008
```

## Smoke

```bash
# Create an exercise
curl -X POST http://localhost:3008/api/exercises \
  -H 'content-type: application/json' \
  -d '{"name":"Bench Press","muscle_group":"chest"}'

# FTS5 search — partial token + rank ordering
curl 'http://localhost:3008/api/exercises?q=bench'

# Log a workout with one set (replace EX_ID)
curl -X POST http://localhost:3008/api/workouts \
  -H 'content-type: application/json' \
  -d '{"name":"Push day","sets":[
    {"exercise_id":"EX_ID","weight_minor":80000,"reps":8,"rir":2}
  ]}'

# Stats (computed across all exercises with the pure PR module)
curl http://localhost:3008/api/stats

# PRs for one exercise (chronological order)
curl http://localhost:3008/api/exercises/EX_ID/prs

# CSV export — note the content-type header
curl -i http://localhost:3008/api/export.csv | head -10
```

## Frontend

```bash
cd projects/09-workouts/frontend
pnpm install
pnpm check        # 0 ERRORS 0 WARNINGS
pnpm test:unit    # 16/16
pnpm dev          # http://localhost:5181 (set --port if vite picks another)
```

## Playwright

```bash
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium
# First time only — generate the visual-regression baseline:
pnpm test:e2e --update-snapshots
# Subsequent runs:
pnpm test:e2e     # 40/40 (5 specs × 4 viewports)
```

The Playwright `webServer` builds + previews the frontend on port 4181. The
backend must be running separately on 3008 — the specs use
`request.post('http://localhost:3008/...')` directly for seeding.
