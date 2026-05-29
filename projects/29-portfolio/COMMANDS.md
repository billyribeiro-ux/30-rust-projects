# Commands — Project 29 (Cinematic Portfolio)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Postgres

```bash
cd projects/29-portfolio
docker compose up -d    # postgres on :5450
```

## 3. Backend

```bash
cd backend
cp .env.example .env
sqlx migrate run --database-url "postgres://portfolio:portfolio_dev_password@localhost:5450/portfolio"
SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://portfolio:portfolio_dev_password@localhost:5450/portfolio" cargo test
DATABASE_URL="postgres://portfolio:portfolio_dev_password@localhost:5450/portfolio" cargo sqlx prepare
DATABASE_URL="postgres://portfolio:portfolio_dev_password@localhost:5450/portfolio" cargo run
# Backend listens on :3028.
```

## 4. Frontend

```bash
cd projects/29-portfolio/frontend
pnpm install
pnpm check        # 0/0
pnpm test:unit    # 3 pass
pnpm dev          # http://localhost:5201
```

## 5. E2E

```bash
cd projects/29-portfolio/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3028 pnpm test:e2e
# 8/8 (2 specs × 4 viewports)
```

## 6. Seed a post

```bash
curl -c c.txt -X POST http://localhost:3028/api/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"designer@example.com","password":"correct horse battery","name":"Designer"}'

curl -b c.txt -X POST http://localhost:3028/api/cms/posts \
  -H 'content-type: application/json' \
  -d '{
    "slug": "atlas-design-system",
    "title": "Atlas — a design system for billion-dollar startups",
    "summary": "How I scoped, designed, and shipped a 200-component library in 6 months.",
    "hero_image_url": "https://images.unsplash.com/photo-1542831371-29b0f74f9713",
    "body_mdx": "# Atlas\n\nThis was a long project...\n\n## The problem\n\n..."
  }'

# Get the id from the response, then:
curl -b c.txt -X POST http://localhost:3028/api/cms/posts/<id>/publish
```

## 7. Reset DB

```bash
docker compose down -v && docker compose up -d
cd backend
sqlx migrate run --database-url "postgres://portfolio:portfolio_dev_password@localhost:5450/portfolio"
```
