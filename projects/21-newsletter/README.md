# Project 21 — Newsletter Platform — Stripe Subscriptions #2

A self-hosted newsletter with **Stripe Subscriptions** (Pro tier),
magic-link auth for subscribers, gated posts behind a paywall, RSS
feed, dunning + auto-downgrade.

## What's new in this project

- **Stripe Subscriptions** (not Checkout-once like project 16) —
  recurring billing, lifecycle webhooks (`created`/`updated`/`deleted`),
  invoice paid/failed.
- **Local mirror of Stripe state** in `subscriptions` table; the webhook
  is the only writer. App reads "is this person Pro?" from local state,
  never round-trips to Stripe.
- **Dunning**: `invoice.payment_failed` → `past_due` with timestamp.
  A reaper (`find_past_due_to_downgrade` + `downgrade_to_free`)
  auto-downgrades after a 14-day grace.
- **Magic-link auth** for subscribers (no password). Single-use token,
  10-minute TTL, hashed at rest.
- **Two cookie-scoped session systems**: `app_session` for the author
  (admin/login), `sub_session` for subscribers. A stolen author cookie
  cannot read subscriber-only routes and vice versa.
- **Paywall returns 402** with the truncated body — crawlers + RSS get
  the teaser; signed-in Pro subscribers get the full article.
- **RSS 2.0** at `/feed.xml`, hand-rolled (no crate).
- **Webhook signature verify**: 5-min replay window, constant-time
  compare, multiple `v1=` accepted for secret rotation.

## Stack delta vs project 16

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres, sessions, Argon2, Stripe webhook + idempotency | Adds **subscriptions** schema mirroring Stripe state, **magic_links**, **subscriber_sessions**, the dunning reaper, Markdown rendering with sanitization |
| Frontend | hooks.server.ts auth, JSON-LD | Adds **Article JSON-LD with `isAccessibleForFree`** so Google surfaces the paywall correctly, RSS `<link rel="alternate">`, subscribe form |
| Tests | wiremock'd Stripe, webhook signature suite | Adds dunning grace-period test (15-day-old past_due downgrades cleanly) |

## Layout

```
projects/21-newsletter/
├── README.md
├── COMMANDS.md
├── LESSON.md
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml
│   ├── .env.example          STRIPE_PRICE_ID, STRIPE_WEBHOOK_SECRET, SMTP_URL
│   ├── migrations/0001_init.sql
│   ├── .sqlx/                offline query cache (committed)
│   ├── tests/webhook_flow.rs  4 integration tests
│   └── src/
│       ├── main.rs, lib.rs (build_app)
│       ├── auth/{hash,session,mod}.rs  AuthUser + AuthSubscriber
│       ├── stripe/
│       │   ├── client.rs       hand-rolled checkout + portal POST
│       │   └── webhook.rs      HMAC-SHA256 verify, replay window, rotation
│       └── routes/
│           ├── auth.rs         author register/login/logout
│           ├── subscribe.rs    starts Stripe Checkout OR magic-link only
│           ├── magic.rs        magic-link start + verify
│           ├── posts.rs        public read with entitlement check
│           ├── admin.rs        author CRUD + publish
│           ├── stripe.rs       webhook + dunning helpers
│           └── feed.rs         RSS 2.0
└── frontend/
    ├── package.json            ports: dev 5193, preview 4193
    ├── playwright.config.ts    4-viewport matrix
    ├── e2e/newsletter.spec.ts  4 specs × 4 viewports = 16 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts
        └── routes/
            ├── +layout.{svelte,server.ts}
            ├── +page.{svelte,server.ts}    Home + subscribe form
            └── p/[slug]/+page.{svelte,server.ts}    Post + paywall + Article JSON-LD
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **14/14 pass** (10 unit + 4 integration: signature verify, replay, tamper, key rotation, idempotency, dunning) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **5/5 pass** |
| `pnpm test:e2e` | **16/16 pass** (4 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean (one documented `state_referenced_locally` silence on the post page) |

## What you can do now

Charge $5/mo on the internet correctly. Mirror Stripe state locally so
your app never depends on a network call to know who's paying. Survive
a card failure → grace → downgrade without losing the customer's data.

## What's next

Project 22 — Background Jobs Dashboard. Postgres `SKIP LOCKED`, retry
with backoff, SSE for the dashboard, OpenTelemetry tracing.
