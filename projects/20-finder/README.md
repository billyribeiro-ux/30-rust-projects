# Project 20 — Geo-aware Restaurant Finder (+ OAuth)

A search-first restaurant finder with **PostGIS** distance queries,
**OAuth 2.0 + PKCE** (Google and GitHub, hand-rolled — the lesson is
the protocol), Restaurant JSON-LD for SEO, and graceful geolocation
degradation.

## What's new in this project

- **PostGIS** + `GEOGRAPHY(Point, 4326)` columns + GiST index;
  `ST_DWithin` for "within N metres of (lat, lng)" and `ST_Distance`
  for ordering nearest-first.
- **OAuth 2.0 Authorization Code + PKCE**, hand-rolled. Both Google
  (with `id_token` nonce verification) and GitHub. State, verifier,
  and nonce are stashed in short-lived HttpOnly cookies; mismatch is
  a 401. PKCE `S256` only — plain is unsafe.
- **Server-rendered first paint** of search results so crawlers get
  `Restaurant` / `ItemList` JSON-LD without executing JS.
- **Graceful geolocation degradation**: permission denied falls back
  to manual entry / canned cities.
- **`test_only` route + `TEST_ONLY_TOKEN` env var** unlocks a
  session-injection endpoint that exists *only* for e2e — guarded by
  a shared secret. Production deployments leave the var unset and
  the route returns 404 to everyone.

## Stack delta vs project 19

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres, Argon2, sessions, Axum, sqlx | Adds PostGIS, the `oauth/` module (pkce, provider trait, registry), `reqwest` for token exchange, `wiremock` for OAuth-provider tests |
| Frontend | hooks.server.ts auth, tsconfig strict, Phosphor | Adds `maplibre-gl` dependency, Restaurant JSON-LD, OAuth provider buttons, server-rendered geo search |
| Tests | Vitest + Playwright × 4 viewports + axe-core | Adds OAuth state/PKCE unit tests on the backend |

## Layout

```
projects/20-finder/
├── README.md
├── COMMANDS.md
├── LESSON.md
├── docker-compose.yml          postgis/postgis:16-3.4-alpine
├── backend/
│   ├── Cargo.toml
│   ├── .env.example            OAUTH_*, OAUTH_REDIRECT_BASE, OAUTH_SUCCESS_REDIRECT
│   ├── migrations/0001_init.sql  postgis extension + places/reviews/oauth_accounts/friend_recs
│   ├── .sqlx/                  offline query cache (committed)
│   └── src/
│       ├── main.rs             port 3019, builds OAuthRegistry from env
│       ├── lib.rs              build_app(state, cors_origin)
│       ├── auth/{hash,session,mod}.rs
│       ├── oauth/
│       │   ├── mod.rs          OAuthRegistry
│       │   ├── pkce.rs         random_token, code_challenge_s256, constant_eq
│       │   └── provider.rs     OAuthProvider trait + Google + Github impls
│       ├── routes/
│       │   ├── auth.rs         password register/login/logout/me
│       │   ├── oauth.rs        /:provider/start + /:provider/callback
│       │   ├── places.rs       ST_DWithin search + detail
│       │   ├── reviews.rs      one review per (place, user) — UPSERT
│       │   ├── recs.rs         friend_recs minimal surface
│       │   └── test_only.rs    POST /login-as (only when TEST_ONLY_TOKEN set)
│       ├── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json            ports: dev 5192, preview 4192
    ├── playwright.config.ts    4-viewport matrix
    ├── e2e/finder.spec.ts      5 specs × 4 viewports = 20 runs
    └── src/
        ├── app.{html,css,d.ts}
        ├── hooks.server.ts
        ├── lib/
        │   ├── api.ts          places/auth/reviews
        │   ├── api.test.ts     6 unit tests
        │   └── types.ts
        └── routes/
            ├── +layout.{svelte,server.ts}
            ├── +page.{svelte,server.ts}    Search page + JSON-LD
            ├── login/+page.{svelte,server.ts}    OAuth + email
            ├── signup/+page.{svelte,server.ts}
            ├── logout/+page.server.ts
            └── places/[id]/+page.{svelte,server.ts}    Detail + Review JSON-LD
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **9/9 pass** (auth hash + email validation + OAuth PKCE + provider URL builders) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **6/6 pass** |
| `pnpm test:e2e` | **20/20 pass** (5 specs × 4 viewports, axe-core wcag2a+aa, JSON-LD assertion) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean (3 documented `state_referenced_locally` silences — see LESSON §B) |

## What you can do now

Run a geo product without buying anything proprietary, and integrate
identity providers correctly — by writing the protocol, not by trusting
a library's defaults.

## What's next

Project 21 builds on this with **Stripe Subscriptions** and magic-link
auth for subscribers (no password required).
