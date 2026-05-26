# Project 12 — Job Application Tracker

> "I'm searching for a job and I lose track of which company I applied to, what stage each application is in, and when I committed to follow up. Spreadsheets break the second a friend wants to peek over my shoulder. I want a single place I trust."

A focused job-application tracker. Add applications, advance them through a
status pipeline (wishlist → applied → screening → interview → offer →
accepted, or rejected/withdrawn), capture notes on the timeline, schedule
follow-up next steps with due dates, get email reminders before things slip,
and subscribe to your next steps in Apple Calendar / Google Calendar via an
ICS feed.

This is the second auth-bearing project, so it inherits the **whole identity
stack from project 11** — Argon2id, opaque session cookies, email verification,
password reset, MailHog locally. Project 12 adds three things on top:

1. **A bcrypt → Argon2 dual-verify migration path.** Pretend you imported users
   from a legacy system that hashed passwords with bcrypt. On login the server
   tries Argon2 first; if there is no Argon2 hash but a `legacy_bcrypt_hash`
   exists, it verifies against bcrypt and, on success, **re-hashes the password
   with Argon2** and nulls the legacy column in the same transaction. The user
   never knows. After they log in once, the legacy hash is gone forever.
2. **A tokio background task** that scans `next_steps` on a tick, finds
   anything due in the next 24 hours that hasn't been reminded yet, sends an
   email, and stamps `reminded_at`. The stamp is the **idempotency key** — if
   the task restarts mid-scan, no one gets a duplicate email.
3. **ICS (RFC 5545) calendar export.** A `/api/export/next-steps.ics`
   endpoint hand-builds a VCALENDAR with one VEVENT per outstanding next step
   (plus a VALARM 30 min before due). Subscribe the URL in any calendar app and
   your follow-ups show up next to the rest of your life.

## Stack

- **Backend** — Rust 2024 + Axum 0.8 + sqlx + Postgres 16. Same `AppError`
  pattern as project 11, same `tower-http` CORS + trace middleware, same
  Argon2id parameters (OWASP 2025 m=19456, t=2, p=1).
- **Frontend** — SvelteKit 2 + Svelte 5 runes + TypeScript strict. Form actions
  for mutations, `+page.server.ts` `load` for reads, `hooks.server.ts` for the
  auth guard, plain CSS with cascade layers, Phosphor icons.
- **Tests** — `cargo test` for backend units, Playwright at 4 viewports for
  E2E + a11y. The permission matrix (A can't see B's row → 404 not 403) is the
  headline test.

## Run it

```bash
docker compose up -d                       # Postgres + MailHog
cp backend/.env.example backend/.env       # then edit DATABASE_URL if needed
cp frontend/.env.example frontend/.env

cd backend && cargo run                    # http://localhost:3011
cd frontend && pnpm install && pnpm dev    # http://localhost:5184
```

Open `http://localhost:5184/register` and create an account. The verification
email lands at `http://localhost:8025` (MailHog inbox). Add a few applications,
schedule a next step with a due date in the next 24 hours, then watch your
backend logs for `reminder sent` lines as the background task fires.

## What you'll learn

Read [LESSON.md](./LESSON.md) for the line-by-line teaching narrative — every
non-obvious choice gets a "what / why / what would break otherwise" callout.
[COMMANDS.md](./COMMANDS.md) is the copy-paste script from `mkdir` to
`git push`. The three lessons that pay back the most thinking time are:

- **The dual-verify migration** (`backend/src/routes/auth.rs` login handler).
  How to roll forward from bcrypt to Argon2 without locking anyone out, and
  why constant-time comparison still matters even when the user doesn't exist.
- **Background tokio task lifecycle** (`backend/src/reminders.rs`). Why we
  spawn with `tokio::spawn`, why the loop swallows errors instead of
  propagating, and why `reminded_at` is the idempotency key (and not a
  separate `reminders_sent` table).
- **ICS export** (`backend/src/routes/export.rs`). Hand-rolling RFC 5545 —
  the line endings, the escape rules, the alarm trigger format — and why we
  return `Response<Body>` directly instead of going through `IntoResponse`.

## Status

Shipped 2026-05-26. All 36 Playwright tests pass (9 tests × 4 viewports).
Backend: 11 unit tests pass, clippy clean.
