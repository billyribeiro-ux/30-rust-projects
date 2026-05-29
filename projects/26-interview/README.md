# Project 26 — Live Coding Interview Platform

A two-person live coding workspace: a shared editor (WebSocket
broadcast), a mock code runner (trait-gated for testability), and an
SP-side SAML metadata endpoint that an enterprise IdP can configure
against.

## What's new

- **Axum WebSocket** with a per-interview `tokio::sync::broadcast`
  channel. The first connection allocates the hub; the last to leave
  drops it.
- **Last-write-wins shared editor** — the WebSocket relays JSON
  messages and the server persists `kind: "code"` updates to
  `interviews.code`. Documented limitation: two simultaneous typists
  fight. Upgrading to **`yrs` (Yjs Rust)** for true CRDT semantics is
  the documented next step.
- **Trait-based code runner** (`CodeRunner`) with a `MockRunner` that
  ships by default. Production swaps in a `DockerRunner` calling
  `docker run --rm --network=none …` per the curriculum.
- **SAML SP metadata XML** at `/saml/metadata`. Enterprise customers
  upload this into their IdP to wire SSO. The `acs` endpoint is
  documented but not signature-verified (`samael` integration is the
  documented future-work path).

## Stack delta vs project 25

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres + Argon2 + Axum | Adds Axum `ws` feature, `tokio::sync::broadcast`, `futures-util`, `async-trait`, SAML metadata |
| Frontend | hooks.server.ts | Adds WebSocket `EventTarget`-style code editor sync |
| Tests | sqlx integration | Adds **two-client WebSocket race**: A sends, B receives, server persists |

## Layout

```
projects/26-interview/
├── README.md, COMMANDS.md, LESSON.md
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml
│   ├── .env.example                  SAML_ENTITY_ID, SAML_ACS_URL
│   ├── migrations/0001_init.sql
│   ├── .sqlx/
│   ├── tests/ws_flow.rs              2 integration tests
│   └── src/
│       ├── main.rs, lib.rs           make_hubs() factory
│       ├── auth/{hash,session,mod}.rs
│       ├── routes/
│       │   ├── auth.rs
│       │   ├── interviews.rs         CRUD + state
│       │   ├── ws.rs                 WebSocket broadcast
│       │   ├── executions.rs         CodeRunner trait + MockRunner
│       │   └── saml.rs               SP metadata + ACS stub
│       └── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json                  ports dev 5198, preview 4198
    ├── e2e/interview.spec.ts         3 specs × 4 viewports = 12 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts    wsUrl() helper
        └── routes/
            ├── +layout.{svelte,server.ts}, +page.{svelte,server.ts}    interview list + create
            ├── login/, signup/, logout/
            └── i/[id]/+page.{svelte,server.ts}                          shared editor
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **3/3 pass** (Argon2 + WebSocket two-client + SAML metadata XML) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **5/5 pass** |
| `pnpm test:e2e` | **12/12 pass** (3 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean (2 documented `state_referenced_locally` silences) |

## What's stubbed (and how to upgrade)

| Stubbed | Replacement |
| --- | --- |
| Last-write-wins editor | `yrs` Rust port of Yjs; clients send Y.js binary updates; server applies + rebroadcasts |
| MockRunner | `DockerRunner` calling `docker run --rm --network=none --memory=256m --cpus=1 --read-only --tmpfs /tmp` |
| SAML ACS | `samael` crate: verify signature on `SAMLResponse`, extract NameID, JIT-provision user, create session |

## What you can do now

Run a live shared editor for two participants. Wire your SP into an
enterprise IdP using real SAML metadata. Reason about the upgrade
path to a true CRDT.

## What's next

Project 27 — Realtime Analytics Dashboard with DuckDB and GSAP.
