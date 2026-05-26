# Project 05 — Commands

Same toolchain as 01–04. Ports: backend 3004, dev 5177, preview 4177.

## 1. Scaffold

```bash
mkdir -p projects/05-bookmarks/backend/src/routes \
         projects/05-bookmarks/backend/migrations \
         projects/05-bookmarks/frontend/src/lib/components \
         projects/05-bookmarks/frontend/src/routes \
         projects/05-bookmarks/frontend/e2e
```

## 2. Backend

```bash
cd projects/05-bookmarks/backend
# Write Cargo.toml, migrations/0001_init.sql, src/*.rs per LESSON.md section A.
export DATABASE_URL="sqlite://$(pwd)/bookmarks.db"
sqlite3 bookmarks.db < migrations/0001_init.sql
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test    # 9 tests pass
```

## 3. Backend run + smoke

```bash
cargo run --release   # http://localhost:3004

# Create a bookmark with tags
curl -X POST http://localhost:3004/api/bookmarks \
  -H 'content-type: application/json' \
  -d '{"url":"https://docs.rs/axum","title":"Axum docs","description":"the rust web framework","tags":["rust","axum","web"]}'

# List all bookmarks
curl http://localhost:3004/api/bookmarks

# Filter by tag
curl 'http://localhost:3004/api/bookmarks?tag=rust'

# Search query
curl 'http://localhost:3004/api/bookmarks?q=axum'

# Combined: tag + search
curl 'http://localhost:3004/api/bookmarks?tag=rust&q=web'

# Tags with counts
curl http://localhost:3004/api/tags

# Try a malformed URL — 422
curl -X POST http://localhost:3004/api/bookmarks \
  -H 'content-type: application/json' \
  -d '{"url":"javascript:alert(1)","title":"x","tags":[]}'
# { "error": { "code": "validation_failed", "message": "validation: url must be http or https" } }
```

## 4. Frontend

```bash
cd projects/05-bookmarks/frontend
pnpm install
pnpm check       # 0/0
pnpm test:unit   # 8 tests pass (api + debounce)
pnpm dev         # http://localhost:5177
```

Add 2-3 bookmarks. Type in the search box — notice the URL doesn't update on every keystroke; it commits after 200ms of idle. Click a tag in the sidebar — the URL updates and the grid filters. Use browser back/forward — the filters re-apply.

## 5. Playwright

```bash
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium   # first time only
pnpm test:e2e    # 20 tests pass (5 specs × 4 viewports)
```

Specs:
1. `home page is accessible (axe-core)`.
2. `add a bookmark, see it in the grid, delete it` — full lifecycle through the UI.
3. `search updates the URL after debounce` — seeds via API, types into the search box, waits 350ms, asserts the URL contains `q=` and the matching bookmark is visible.
4. `clicking a sidebar tag filters and adds ?tag= to URL`.
5. `clickOutside closes the Add panel` — opens the Add panel, clicks the page heading outside it, asserts the panel closed.

## 6. Reset DB

```bash
rm -f bookmarks.db bookmarks.db-shm bookmarks.db-wal
sqlite3 bookmarks.db < migrations/0001_init.sql
cargo run --release
```
