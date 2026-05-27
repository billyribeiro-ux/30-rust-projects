# Project 18 — Polls & Surveys with Live Results

> "I run live audience Q&A during talks; I want a poll page anyone can join via QR code, and bars that animate as votes come in."

Spin up a poll, share a join URL via QR code, watch votes land on a big-screen
"stage" view with bars that spring-animate in real time. Backed by a Postgres
`LISTEN/NOTIFY` -> Server-Sent Events bridge that scales horizontally across
backend processes (unlike a process-local `tokio::sync::broadcast`).

## Ports

- Backend: `3016`
- Frontend dev: `5189`
- Playwright preview: `4189`

## Stack

- **Backend**: axum 0.8, sqlx 0.8 (Postgres), `qrcode` 0.14 for SVG QR codes,
  `tokio-stream` for SSE streams, `PgListener` for `LISTEN`.
- **Frontend**: SvelteKit, Svelte 5 (`Spring` from `svelte/motion` for bar
  animation), `EventSource` for live updates.
- **Auth**: Argon2id passwords + SHA-256-hashed session cookies (reused from
  project 14). Owner-only on `/api/polls` (list+create+close); public on
  `/api/polls/{slug}`, `/qr`, `/vote`, and `/stream`.

## Routes

| Method | Path                          | Auth   | Notes                                |
|--------|-------------------------------|--------|--------------------------------------|
| POST   | `/api/polls`                  | yes    | Create poll w/ slug + options array  |
| GET    | `/api/polls`                  | yes    | List my polls                        |
| GET    | `/api/polls/{slug}`           | no     | Public read (join page)              |
| GET    | `/api/polls/{slug}/qr`        | no     | SVG QR code linking to `/p/{slug}`   |
| POST   | `/api/polls/{slug}/vote`      | no     | `{option_id}` body; dedupe via cookie+IP |
| POST   | `/api/polls/{slug}/close`     | owner  | Flips `is_open=false`                |
| GET    | `/api/polls/{slug}/stream`    | no     | SSE — `counts` events                |

## Frontend pages

- `(app)/+page.svelte` — list my polls
- `(app)/polls/new/+page.svelte` — create form with dynamic option rows
- `(app)/polls/[slug]/+page.svelte` — admin live "stage" view (QR + animated bars)
- `(public)/p/[slug]/+page.svelte` — voter view (big buttons, no auth)

## Quick start

```bash
sudo -u postgres psql -c "CREATE USER polls WITH PASSWORD 'polls_dev_password'; CREATE DATABASE polls OWNER polls;"
cd backend && cp .env.example .env
DATABASE_URL=postgres://polls:polls_dev_password@localhost:5432/polls cargo run
# in another shell:
cd frontend && pnpm install && pnpm dev
```

See [COMMANDS.md](./COMMANDS.md) for the full quality-gate run.
See [LESSON.md](./LESSON.md) for the why-it's-built-this-way write-up.
