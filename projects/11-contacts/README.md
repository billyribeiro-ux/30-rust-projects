# Project 11 — Contact Manager + Dashboard

> "My contacts are scattered across 4 apps. I want one place with relationships, notes, last-contacted dates, and reminders — with login so my data is mine."

A tiny single-user CRM. Add contacts with tags, search across name/email/company/notes via full-text search, see a dashboard of stats and follow-ups. Behind the gate of email + password auth.

**Eleventh project — the big shift.** First Postgres, first auth, first Docker Compose, first email service (MailHog). The patterns laid here are reused unchanged through project 30.

## Required NEW lessons

- **Docker Compose** for local infra — Postgres 16 + MailHog. One `docker compose up -d` and you have a fully provisioned local environment.
- **Postgres 16** with `sqlx` — the dialect bump from SQLite means: `now()` instead of CURRENT_TIMESTAMP, `TIMESTAMPTZ` for timezone-aware timestamps, `BYTEA` for bytes, `GENERATED ALWAYS AS ... STORED` columns, `FILTER (WHERE ...)` aggregates, `array_agg(...) FILTER (WHERE ... IS NOT NULL)`, `INTERVAL '7 days'`, and `ON CONFLICT (...) DO UPDATE`.
- **Argon2id password hashing** at OWASP 2025 parameters (`m_cost=19456, t_cost=2, p_cost=1, output=32B`). Round-trip and "malformed hash → internal error" tests.
- **Server-side sessions** — opaque 32-byte tokens, base64url; raw token in HttpOnly + SameSite=Lax + 30-day cookie; only SHA-256 hash stored in DB. Sliding expiry on every request.
- **Email-verify + password-reset tokens** — single-use, atomic consume via `UPDATE ... RETURNING` with `consumed_at IS NULL` guard.
- **Email** with `lettre` — `tokio1-rustls-tls`, MailHog in dev (catches everything at http://localhost:8025).
- **`hooks.server.ts`** — the auth handle that runs on every request, reads the session cookie, calls the backend's `/api/auth/me`, populates `event.locals.user`. The whole frontend reads off that one variable.
- **Cookie forwarding from SvelteKit to the cross-origin backend** — `event.fetch` only forwards cookies same-origin, so we manually attach `cookie:` headers for our backend calls (the `serverFetch` helper).
- **Route groups `(auth)` and `(app)`** with layout guards — unauth users bounce out of `(app)`; auth users bounce out of `(auth)`. Two `+layout.server.ts` files, no other auth checks anywhere.
- **PostgreSQL `tsvector` full-text search** — a GENERATED column with weighted concatenation (A=name, B=email+company, D=notes), GIN index, `@@ plainto_tsquery` for forgiving input, `ts_rank` for relevance.
- **Per-user data scoping enforced server-side** — every contacts query filters by `user_id` from the AuthUser extractor. The Playwright permission-matrix tests prove user A can never see user B's data.
- **The "no user enumeration" discipline** — `/login` runs Argon2 against a dummy hash when the user doesn't exist (constant-time response); `/forgot` always returns 204; `/register` returns a generic 409 on duplicate email.

## Stack

- **Backend**: Rust 2024, Axum 0.8, sqlx + Postgres, argon2, lettre, csv. `axum-extra` cookie jar. `subtle` for constant-time comparison. `sha2` for session-token hashing.
- **Frontend**: SvelteKit 2 + Svelte 5 runes, plain CSS with cascade layers, Phosphor icons.
- **Infra**: Postgres 16 + MailHog via `docker-compose.yml`.

## Quality gates

- `cargo fmt --check` ✅
- `cargo clippy --all-targets -- -D warnings` ✅
- `cargo test` → **7/7** (Argon2 round-trip, password length, malformed-PHC, email normalize × 3)
- `pnpm check` → 0/0
- `pnpm test:e2e` → Playwright permission-matrix specs across 4 viewports

## Layout

```
projects/11-contacts/
├── docker-compose.yml           # Postgres 16 + MailHog
├── backend/
│   ├── Cargo.toml
│   ├── migrations/0001_init.sql
│   ├── src/
│   │   ├── main.rs, db.rs, error.rs, state.rs, email.rs
│   │   ├── auth/{mod,hash,session,tokens}.rs
│   │   └── routes/{mod,auth,contacts,dashboard,tags}.rs
│   └── .env.example
└── frontend/
    └── src/
        ├── hooks.server.ts                   ← the auth handle
        ├── app.d.ts                          ← extends App.Locals
        ├── lib/api.ts, lib/server/api.ts     ← cookie-forwarding helper
        └── routes/
            ├── +layout.{server.ts,svelte}
            ├── (auth)/{+layout.{server.ts,svelte},login,register,forgot}/+page.{server.ts,svelte}
            └── (app)/
                ├── +layout.{server.ts,svelte}    ← auth gate
                ├── +page.{server.ts,svelte}      ← dashboard
                ├── logout/+page.server.ts
                └── contacts/{+page.{...},new/+page.{...},'[id]'/+page.{...}}/
```

## What's next

Project 12 — **Job Application Tracker + bcrypt legacy-migration lesson**. The auth foundation from this project carries over; we add a "we acquired a company and inherited their bcrypt hashes" scenario and the dual-verify pattern.
