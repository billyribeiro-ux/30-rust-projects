# Commands — Project 26 (Live Coding Interview Platform)

## 1. Prereqs

```bash
rustup default stable
corepack enable && corepack use pnpm@latest
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## 2. Start Postgres

```bash
cd projects/26-interview
docker compose up -d    # postgres on :5447
```

## 3. Backend

```bash
cd backend
cp .env.example .env

sqlx migrate run --database-url "postgres://interview:interview_dev_password@localhost:5447/interview"

SQLX_OFFLINE=true cargo build
SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL="postgres://interview:interview_dev_password@localhost:5447/interview" cargo test
DATABASE_URL="postgres://interview:interview_dev_password@localhost:5447/interview" cargo sqlx prepare

DATABASE_URL="postgres://interview:interview_dev_password@localhost:5447/interview" cargo run
# Backend listens on :3025.
```

## 4. Frontend

```bash
cd projects/26-interview/frontend
pnpm install
pnpm check        # 0/0
pnpm test:unit    # 5 pass
pnpm dev          # http://localhost:5198
```

## 5. E2E

```bash
cd projects/26-interview/frontend
npx playwright install chromium
VITE_BACKEND_URL=http://localhost:3025 pnpm test:e2e
# 12/12 (3 specs × 4 viewports)
```

## 6. Try the live editor

```bash
# Open two browser tabs on the same /i/<id> URL.
# Type in one; the other updates within ~50ms.
# Refresh either tab; `interviews.code` is persisted.
```

## 7. Configure an IdP against the SP metadata

```bash
curl http://localhost:3025/saml/metadata > sp-metadata.xml
# Upload sp-metadata.xml into your IdP (Okta, Azure AD, OneLogin, etc.).
# When the IdP POSTs to /saml/acs, the request currently returns 501.
# Implement signature verification with the `samael` crate (LESSON §B).
```
