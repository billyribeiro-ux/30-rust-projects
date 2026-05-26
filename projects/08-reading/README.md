# Project 08 — Reading Tracker

> "Goodreads is bloated; I want to track what I read and what I learned."

A tiny reading tracker. Add books manually or look them up by ISBN via Open Library. Track reading status (want_to_read / reading / finished), log per-session pages-read + duration, save highlights with optional page references.

Eighth of 30 projects. New lessons:

- **External API integration with `reqwest`** — typed JSON client, timeouts, user-agent header.
- **`moka` server-side cache** — bounded LRU + TTL for outbound HTTP responses. The Open Library client checks the cache before hitting the network.
- **`wiremock-rs` for testing outbound HTTP** — spin up a mock server in tests, point the client at it, assert request/response behavior without touching the real upstream.
- **Streamed `load` returns** — primary data (books list) flushes immediately; secondary data (stats) streams in after via `{#await data.streamed.stats}` with a skeleton placeholder.
- **`$state.raw`** — for the highlights/sessions arrays that we only ever REPLACE (never mutate piecewise). Skips proxy wrapping; faster, accidental piecewise mutations would throw.
- **Class with rune fields** in a `.svelte.ts` module — `BookModel` declares `$state` instance fields and `$derived` getters. Modern Svelte 5 replacement for the legacy writable-store + custom-set + derived-store pattern.
- **`<svelte:boundary>`** containing a failing form so a render error in the lookup section doesn't crash the whole page.

## Stack

Same as previous projects, plus `reqwest`, `moka` (backend) and `wiremock-rs` (dev-dep). No new frontend deps.

## Quality gates

- `cargo test` → **13/13** (11 unit + 2 wiremock integration)
- `pnpm check` → 0/0
- `pnpm test:unit` → **5/5**
- `pnpm test:e2e` → **20/20** (5 specs × 4 viewports)
- svelte-autofixer clean

## What's next

Project 09 — **Workout Logger with PR Detection** — last single-user SQLite project before file uploads land in project 10. Complex `$derived` chains, SQLite FTS5 full-text search on exercise names, axe-core in CI, CSV export.
