# Project 17 — URL Shortener + Click Analytics

> "I need branded short links and click analytics for my campaigns."

A self-hostable URL shortener with per-link click analytics, a Redis-backed
hot counter, and TOTP-based two-factor authentication.

## Stack

- **Backend**: Rust + Axum, Postgres (sqlx), Redis (deadpool-redis), Argon2,
  totp-rs, qrcode.
- **Frontend**: SvelteKit (Svelte 5 runes), adapter-node, phosphor-svelte
  icons.
- **Ports**: backend `3015`, frontend dev `5188`, Playwright preview `4188`.

## Highlights

- **Hot redirect path** at `GET /{slug}`. Async DB read + Redis `INCR` +
  `tokio::spawn` click insert so the 307 response goes out before analytics
  durability work completes.
- **Bot filter** on User-Agent (Googlebot/curl/wget/Python-urllib/...). Bots
  still get the redirect; their clicks just aren't recorded.
- **Redis as counter cache**, optional. If `REDIS_URL` is unset (or the
  server is unreachable at startup), the app runs in DB-only mode — the
  redirect path skips the INCR and the live counter shows as `null`.
- **2FA via TOTP** (`POST /api/2fa/setup` → QR + provisioning URI →
  `POST /api/2fa/verify`). Login becomes a two-stage flow when 2FA is on:
  password stage returns a 5-minute intermediate token; the client trades it
  + a 6-digit code for the real session.
- **10 backup codes**, Argon2-hashed at issuance, single-use.
- **404-not-403 scoping** on stats endpoints — looking at someone else's
  slug never leaks its existence.

See `LESSON.md` for the in-depth walkthrough and `COMMANDS.md` for the
local-dev cheatsheet.

## Quick start

```bash
# Backend
cd backend
sudo -u postgres psql -c "CREATE USER shortener WITH PASSWORD 'shortener_dev_password';"
sudo -u postgres psql -c "CREATE DATABASE shortener OWNER shortener;"
cp .env.example .env
cargo run

# Redis (optional but recommended)
service redis-server start
# or: docker run -p 6379:6379 redis:7-alpine

# Frontend
cd ../frontend
pnpm install
pnpm dev
```

Open <http://localhost:5188>, register, create a link, hit the short URL,
watch the counter tick.
