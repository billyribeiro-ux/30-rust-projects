# Project 10 — Commands

Ports: backend 3009, frontend dev 5182, Playwright preview 4182.

```bash
mkdir -p projects/10-recipes/{backend/src/routes,backend/migrations,backend/tests,backend/uploads,frontend/src/lib/components,frontend/src/routes/recipes/'[slug]'/edit,frontend/src/routes/share/'[slug]',frontend/e2e}

cd projects/10-recipes/backend
# Write Cargo.toml, migrations/0001_init.sql, src/{db,error,signed,state,uploads,main}.rs + src/routes/*.rs + tests/upload_roundtrip.rs
export DATABASE_URL="sqlite://$(pwd)/recipes.db"
sqlite3 recipes.db < migrations/0001_init.sql
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test     # 20/20 (19 unit + 1 EXIF integration)
cargo run --release   # :3009
```

Smoke (create + upload an image + EXIF check):

```bash
# Create a recipe
RECIPE_ID=$(curl -s -X POST http://localhost:3009/api/recipes \
  -H 'content-type: application/json' \
  -d '{"title":"Apple Pie","description":"Grandmother recipe",
       "ingredients":["2 apples","200g flour","1 tsp cinnamon"],
       "instructions":["Peel apples","Mix","Bake at 200C"],
       "prep_minutes":20,"cook_minutes":45,"servings":8}' | python3 -c "import json,sys;print(json.load(sys.stdin)['id'])")
echo "Recipe: $RECIPE_ID"

# Generate a real JPEG with a forged EXIF marker. We use the project's
# own `image` dep via a one-off `cargo run --example` to avoid extra
# tooling. See `examples/gen_exif.rs` (transient — not committed) for
# the script shape; or do it from any language that can splice bytes:
#   - append "\xFF\xE1...Exif\0\0..." after the JPEG SOI marker (FFD8).
# After generating, the file should contain the bytes `Exif\0\0`
# verifiable with `strings test-with-exif.jpg | grep Exif`.

# Upload it
curl -s -X POST http://localhost:3009/api/recipes/$RECIPE_ID/images \
  -F 'file=@/tmp/test-with-exif.jpg;type=image/jpeg'

# Find the stored file and verify EXIF is GONE
STORED=./uploads/$RECIPE_ID/<image-id>.jpg
strings "$STORED" | grep -i exif    # should print nothing

# Make a signed share link (24h expiry)
curl -s -X POST http://localhost:3009/api/recipes/$RECIPE_ID/share

# Patch with cover_image_id
curl -s -X PATCH http://localhost:3009/api/recipes/$RECIPE_ID \
  -H 'content-type: application/json' \
  -d '{"cover_image_id":"<image-id>"}'
```

Reject paths to verify defenses:

```bash
# Plain text masquerading as JPEG — mime sniff rejects with 415
echo "not an image" > /tmp/fake.jpg
curl -i -X POST http://localhost:3009/api/recipes/$RECIPE_ID/images \
  -F 'file=@/tmp/fake.jpg;type=image/jpeg'    # 415

# Tampered share link → 404
curl -i "http://localhost:3009/api/share/apple-pie?sig=tampered&exp=99999999999"
```

Frontend:

```bash
cd projects/10-recipes/frontend
pnpm install
pnpm check        # 0 ERRORS 0 WARNINGS
pnpm test:unit    # 11/11
pnpm dev          # http://localhost:5182
```

Playwright (needs backend running on :3009):

```bash
# In another shell, run the backend release binary
cd ../backend && DATABASE_URL="sqlite://$(pwd)/recipes.db" \
  FRONTEND_ORIGIN="http://localhost:4182" cargo run --release

# Then in the frontend dir:
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium
VITE_BACKEND_URL=http://localhost:3009 pnpm test:e2e     # 24/24 (6 specs × 4 viewports)
```
