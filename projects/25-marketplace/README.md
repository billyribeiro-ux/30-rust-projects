# Project 25 — Course Marketplace (Stripe Connect)

A multi-instructor course marketplace built on **Stripe Connect Express**:
the platform takes a configurable fee, the rest goes directly to the
instructor's connected account. Refunds reverse both the charge and the
fee.

## What's new in this project

- **Connect Express accounts**: account creation, account-link onboarding
  URLs, and the `account.updated` webhook reaction that flips
  `payouts_enabled` when KYC clears.
- **Destination charges with `application_fee_amount`**: the platform
  charges the student, takes its fee, transfers the remainder to the
  instructor's account in one PaymentIntent.
- **`reverse_transfer + refund_application_fee`** on refund — the
  instructor gives the money back, the platform gives the fee back.
- **14-day refund window** enforced server-side (the right place — UI
  hints are nice but the server is the rule).
- **Same webhook discipline** as project 16/21: raw-body extractor,
  HMAC verify, idempotency on `stripe_events`.

## Stack delta vs project 21

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres + Argon2 + Axum + Stripe webhook | Adds Connect Express endpoints (`/v1/accounts`, `/v1/account_links`), destination-charge Checkout, refund-with-reverse-transfer |
| Frontend | hooks.server.ts auth | Adds instructor onboarding entry + buy-with-Stripe form |

## Layout

```
projects/25-marketplace/
├── README.md, COMMANDS.md, LESSON.md
├── docker-compose.yml                 postgres + minio (optional)
├── backend/
│   ├── Cargo.toml
│   ├── .env.example                   STRIPE_*, PLATFORM_FEE_BPS
│   ├── migrations/0001_init.sql
│   ├── .sqlx/
│   ├── tests/connect_flow.rs          3 integration tests
│   └── src/
│       ├── main.rs, lib.rs
│       ├── auth/{hash,session,mod}.rs
│       ├── stripe/
│       │   ├── client.rs              hand-rolled Connect HTTP
│       │   └── webhook.rs              HMAC + replay window + rotation
│       ├── routes/
│       │   ├── auth.rs
│       │   ├── instructors.rs         become-instructor + onboard URL
│       │   ├── courses.rs             CRUD + lessons
│       │   ├── enrollments.rs         destination-charge checkout + refund
│       │   └── stripe.rs              webhook receiver
│       └── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json                   ports dev 5197, preview 4197
    ├── e2e/marketplace.spec.ts        3 specs × 4 viewports = 12 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts
        └── routes/
            ├── +layout.{svelte,server.ts}, +page.{svelte,server.ts}    courses list
            ├── login/, signup/
            └── c/[slug]/+page.{svelte,server.ts}                       detail + buy form
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **5/5 pass** (Argon2 + account.updated + checkout.completed + idempotency + signature mismatch 401) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **4/4 pass** |
| `pnpm test:e2e` | **12/12 pass** (3 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean (one documented `state_referenced_locally` silence) |

## What's stubbed (documented in LESSON.md)

- **HLS transcoding**. The `lessons.hls_manifest` column is wired but
  the encoder isn't shipped — instructors set it via `PATCH` for now.
  Production would call `ffmpeg` via the project-22 job queue.
- **`checkout.session.completed → enrollment` resolution** relies on
  `metadata.course_slug`. The Checkout-creation endpoint doesn't set
  it yet — a TODO in `client.rs`. Tests assert the resolution path
  works when metadata is present.
- **Object-storage download** of video lessons. We use local disk in
  dev; production needs project 15's `object_store` + signed URLs.

## What you can do now

Run a real marketplace where the platform never holds the instructor's
money on its balance sheet. Pay out automatically (Stripe's payout
schedule). Refund cleanly with both sides made whole.

## What's next

Project 26 — Live Coding Interview Platform (CRDT + SAML SSO).
