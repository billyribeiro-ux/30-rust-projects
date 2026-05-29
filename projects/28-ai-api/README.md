# Project 28 — AI Inference API (+ WebAuthn passkeys + metered billing groundwork)

A developer-facing `POST /v1/inference` endpoint, gated by API keys
hashed at rest, rate-limited per minute, with usage tracked for
**Stripe Metered Billing** (groundwork shipped, Stripe Meter Events
mock documented).

Dashboard auth ships **WebAuthn passkeys** (`webauthn-rs`) plus
password fallback.

## What's new in this project

- **WebAuthn / Passkeys** — `webauthn-rs` 0.5. Register-start emits
  the `CreationChallengeResponse`, the browser calls
  `navigator.credentials.create()`, register-finish persists the
  resulting credential. Login mirrors. State is stored in
  `webauthn_states` with a 5-10 minute TTL.
- **API keys hashed at rest** — `sk_live_<43-char-base64url>`. Stored
  only as `SHA-256(secret)`. The plaintext is shown once at creation
  and never again. Prefix (first 8 chars) is kept visible for UI
  identification.
- **Per-key RPM gating** — a single `SELECT COUNT(*)` over the last
  60s of `usage_events` decides 200 vs 403.
- **`usage_events` table** — one row per call, with a partial index
  on `reported_to_stripe_at IS NULL` so the reconciler can batch up
  unreported events efficiently (the Stripe Meter Events POST is
  documented in LESSON §A6).
- **Free tier + per-call billing** as configuration — first N free,
  then `PER_CALL_CENTS` per call. The dashboard shows the estimate.

## Stack delta vs project 21

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres + Argon2 + Axum + sessions | Adds `webauthn-rs`, API key management, usage tracking, RPM gate |
| Frontend | hooks.server.ts | Adds API key list / create / revoke UI + usage dashboard |
| Tests | sqlx integration | Adds **revoke → 401**, **RPM threshold → 403** |

## Layout

```
projects/28-ai-api/
├── README.md, COMMANDS.md, LESSON.md
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml                     webauthn-rs 0.5
│   ├── .env.example
│   ├── migrations/0001_init.sql       webauthn_credentials, api_keys, usage_events
│   ├── .sqlx/
│   ├── tests/inference_flow.rs        2 integration tests
│   └── src/
│       ├── main.rs, lib.rs            build_webauthn() factory
│       ├── auth/{hash,session,mod}.rs
│       ├── routes/
│       │   ├── auth.rs                password fallback
│       │   ├── passkeys.rs            register/login start+finish
│       │   ├── keys.rs                CRUD + authenticate() helper
│       │   ├── inference.rs           POST /v1/inference
│       │   └── usage.rs               this-month summary
│       └── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json                   ports dev 5200, preview 4200
    ├── e2e/ai-api.spec.ts             3 specs × 4 viewports = 12 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts
        └── routes/
            ├── +layout.{svelte,server.ts}, +page.server.ts → /keys or /login
            ├── login/, signup/, logout/
            ├── keys/+page.{svelte,server.ts}                API-key management + usage
            └── passkeys/+page.svelte                        docs page
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **3/3 pass** (Argon2 + key-create-call-revoke + RPM 403) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **5/5 pass** |
| `pnpm test:e2e` | **12/12 pass** (3 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean |

## What's stubbed (documented in LESSON.md)

- **Stripe Meter Events** — the `usage_events` schema is the input. A
  background reaper batches `reported_to_stripe_at IS NULL` rows and
  POSTs them to `https://api.stripe.com/v1/billing/meter_events` with
  an idempotency key. Implementation is one route + a tokio task
  similar to project 22's worker.
- **Real LLM provider** — `mock_complete` returns lorem-ipsum. Swap
  for an OpenAI-compatible client (project 28 spec).
- **Passkey UI flow** — the browser-side ceremony is documented; the
  dashboard ships only the API surface, not the JS that drives
  `navigator.credentials.*`.

## What you can do now

Ship a real developer API with metered billing groundwork. Issue API
keys that can't be reconstructed from your database. Add passkeys as
an enterprise-grade auth option.

## What's next

Project 29 — Cinematic Portfolio + Headless CMS (GSAP #3).
