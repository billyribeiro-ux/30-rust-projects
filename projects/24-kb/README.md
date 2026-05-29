# Project 24 — Hybrid Search Knowledge Base

A public-facing knowledge base where search actually works: Postgres
**FTS** (BM25 + weighted GIN + `ts_headline` snippets) is the primary,
**`pg_trgm`** picks up typos when FTS returns nothing, and **MeiliSearch**
(optional) adds typo-tolerant / fuzzy re-ranking via **Reciprocal Rank
Fusion**.

## What's new

- **`tsvector` GIN with weighted columns**. A trigger rebuilds `tsv`
  from title (`A`), summary (`B`), body (`C`) on every insert/update —
  the app can never forget.
- **`pg_trgm` similarity** for typo fallback. Tested: "postgrss" finds
  "Postgres tuning guide".
- **Reciprocal Rank Fusion** merges FTS + Meili rankings without
  needing per-query weight tuning.
- **`ts_headline`** snippets with `<mark>` tags rendered safely via
  `@html` (the FTS layer is the only HTML source on the search page).
- **Graceful degradation**: backend works with zero search infrastructure.
  Meili off → FTS-only. pg_trgm off (no extension) → no fallback.
- **SEO**: TechArticle + FAQPage JSON-LD, hreflang, sitemap.xml, robots.txt,
  AI-Overview "quick answer" block above the fold.

## Stack delta vs project 23

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres, Argon2, sessions, Axum | Adds `pg_trgm` ext, `tsvector` trigger, search route, sitemap route |
| Frontend | hooks.server.ts | Adds search UI + Article/FAQ JSON-LD + AI-Overview answer block |
| Tests | sqlx integration | Adds search ranking proof (title-match outranks body-match) and typo-fallback proof |

## Layout

```
projects/24-kb/
├── README.md, COMMANDS.md, LESSON.md
├── docker-compose.yml                 postgres:16 + meilisearch (optional)
├── backend/
│   ├── Cargo.toml
│   ├── .env.example
│   ├── migrations/0001_init.sql       pg_trgm + tsv trigger
│   ├── .sqlx/
│   ├── tests/search_flow.rs           3 integration tests
│   └── src/
│       ├── main.rs, lib.rs
│       ├── auth/{hash,session,mod}.rs
│       ├── routes/
│       │   ├── auth.rs
│       │   ├── articles.rs            public read
│       │   ├── search.rs              FTS + trgm + Meili RRF
│       │   ├── admin.rs               CRUD + publish
│       │   └── sitemap.rs             sitemap.xml + robots.txt
│       └── db.rs, error.rs, state.rs
└── frontend/
    ├── package.json                   ports dev 5196, preview 4196
    ├── e2e/kb.spec.ts                 4 specs × 4 viewports = 16 runs
    └── src/
        ├── app.{html,css,d.ts}, hooks.server.ts
        ├── lib/api.ts, api.test.ts, types.ts
        └── routes/
            ├── +layout.{svelte,server.ts}, +page.{svelte,server.ts}    home + search
            └── a/[slug]/+page.{svelte,server.ts}                       article + JSON-LD
```

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **4/4 pass** (Argon2 + FTS title-match ranking + pg_trgm typo fallback + empty query) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **4/4 pass** |
| `pnpm test:e2e` | **16/16 pass** (4 specs × 4 viewports, axe-core wcag2a+aa, sitemap well-formed) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean (2 documented `state_referenced_locally` silences) |

## What you can do now

Replace "we'll add Algolia later" with one Postgres extension you
already have. Add a typo-tolerant fallback for free. Layer in Meili
without rewriting the search code — RRF doesn't care which signal
wins, only the ranks.

## What's next

Project 25 — Course Marketplace with Stripe Connect.
