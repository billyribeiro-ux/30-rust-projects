# Project 16 — Digital Product Storefront — Stripe Checkout #1

> "I want to sell a PDF or video file with a checkout link, deliver it after payment,
> and never touch a card number."

A small SvelteKit + Axum app that lists digital products, redirects buyers to
Stripe Checkout, listens for `checkout.session.completed`, and emails the
buyer a signed 24-hour download link. The admin can refund any order from
the dashboard — that revokes the signed link as it goes.

## What's new in this project

- **Stripe Checkout (one-time payment)** — hand-rolled HTTP client, no `stripe-rust` crate.
- **Webhook signature verification** — raw-body Axum extractor, HMAC-SHA256
  over `"{t}.{body}"`, constant-time compare, 5-minute replay window.
- **Idempotency via `stripe_events(event_id PRIMARY KEY)`** — duplicate
  delivery hits the unique constraint and we return 200 without
  re-fulfilling.
- **Signed download URLs** — opaque `<nonce>.<hmac>` tokens with
  SHA-256 of the raw token stored in `download_links`, 24h expiry,
  revoke-on-refund.
- **Customer-email-only flow** — no account needed; the email link IS the
  receipt.
- **Hand-rolled webhook idempotency lesson** — the trade-off between "I
  promise exactly-once delivery" (impossible) and "the DB will reject
  duplicates" (always works).

## Stack delta vs project 14

| | added | removed |
| --- | --- | --- |
| Cargo | `reqwest`, `hmac`, `sha2`, `hex`, `url`, `tokio-util`, `wiremock` (dev) | `chrono-tz`, `rrule` |
| Frontend | — | calendar grid component |

## Layout

```
projects/16-storefront/
├── README.md  COMMANDS.md  LESSON.md
├── docker-compose.yml      Postgres 16 + MailHog
├── data/products/          where uploaded files live (gitignored except .gitkeep)
├── backend/                Axum, Postgres, hand-rolled Stripe client
│   ├── Cargo.toml          (sqlx postgres + json, reqwest, hmac, wiremock)
│   ├── .env.example
│   ├── migrations/0001_init.sql
│   ├── .sqlx/              offline sqlx data (commit this)
│   ├── src/
│   │   ├── main.rs              boot
│   │   ├── lib.rs               re-exports for integration tests
│   │   ├── db.rs                PgPool
│   │   ├── email.rs             SMTP via lettre
│   │   ├── error.rs             AppError + IntoResponse
│   │   ├── signing.rs           HMAC-SHA256 signed download URLs (pure)
│   │   ├── state.rs             AppState
│   │   ├── auth/                admin sessions (reused from project 14)
│   │   ├── stripe/
│   │   │   ├── client.rs        POST /v1/checkout/sessions + /v1/refunds
│   │   │   ├── types.rs         WebhookEvent, CheckoutSession, etc
│   │   │   └── webhook.rs       signature verify (pure, fully tested)
│   │   └── routes/
│   │       ├── public.rs        GET /api/products, POST /api/checkout
│   │       ├── stripe.rs        POST /api/stripe/webhook
│   │       ├── download.rs      GET /d/{token}
│   │       ├── admin.rs         products CRUD, orders list, refund
│   │       └── auth.rs          admin register/login/logout/me
│   └── tests/
│       ├── stripe_client.rs     wiremock-backed
│       └── webhook_flow.rs      idempotency + signed URL round-trip
└── frontend/                    SvelteKit (port 5188 dev, 4188 preview)
    ├── package.json
    ├── playwright.config.ts     4-viewport matrix
    └── src/
        ├── lib/api.ts           typed HTTP client
        └── routes/
            ├── +page.svelte             storefront
            ├── success/+page.svelte     post-checkout
            ├── cancel/+page.svelte
            ├── admin/+page.svelte       products + orders + refund
            └── admin/login/+page.svelte
```

## Ports

| | port |
| --- | --- |
| backend | 3015 |
| frontend dev | 5188 |
| frontend preview | 4188 |
| Postgres | 5437 |
| MailHog SMTP | 1025 |
| MailHog UI | 8025 |

## Quality gates (latest run)

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` | 28 passed (20 lib unit, 4 wiremock, 4 integration) |
| `pnpm check` | 0 errors, 0 warnings |
| `pnpm test:unit` | 4 passed |
| `pnpm test:e2e` | 28 passed (7 specs × 4 viewports) |
| Svelte autofixer | no issues |

## What's next

Project 17 is the URL shortener with Redis + 2FA. Project 21 builds on
this one with **subscription** billing and the customer portal.
