# Project 10 — Recipe Book with Photos

> "My recipes live in 14 screenshots; I want one searchable cookbook."

A personal recipe collection with photo uploads, public-share links, Recipe JSON-LD for Google rich results, and offline reading via a service worker. Tenth of 30 projects — the LAST single-user SQLite project before we cross into Postgres in project 11.

New lessons (six big ones):

- **First file upload** — multipart POST, server-side mime sniffing via `infer` (NOT client `Content-Type`), 10 MB cap enforced during accumulation, decode + re-encode via `image`, 400×400 JPEG thumbnails.
- **EXIF stripping for privacy** — the decode → encode round-trip drops EXIF/GPS/IPTC chunks "for free". Documented and verified by an integration test that forges an EXIF marker into a JPEG, runs it through the pipeline, and asserts the marker is gone.
- **First remote function** (`$app/server`) — the photo upload UX uses `query` + `form` + `command` from the new SvelteKit remote-function API, demonstrating all three flavors in one feature.
- **Snippets** (`{#snippet}` / `{@render}`) — `RecipeCard` is defined at module level in a `.svelte` file and rendered twice (main grid + related-recipes strip) via `{@render card(recipe)}`.
- **Recipe JSON-LD** — full Schema.org Recipe shape on the detail page (recipeIngredient, recipeInstructions as HowToStep array, prepTime, cookTime, image, author, datePublished, recipeYield) with a Playwright test that parses the `<script type="application/ld+json">` content.
- **First service worker** — `src/service-worker.ts` precaches the SvelteKit build artifacts, network-first-with-fallback on recipe pages, using the `$service-worker` virtual module for build-id-based cache versioning.
- **Signed share URLs** — HMAC-SHA256 over `{slug}|{expires_at_unix}`, base64url-encoded, validated with constant-time `Mac::verify_slice`. 24-hour expiry. Tampering the slug, the exp, or the sig all 404 (deliberately ambiguous so the URL doesn't leak recipe existence).

## Stack

Same as previous projects, plus:
- Backend: `infer` (mime sniffing), `image` (decode/encode/thumbnail), `hmac` + `sha2` + `base64` (signed URLs), `axum::extract::Multipart` and `tower-http::services::ServeDir` for static file serving.
- Frontend: SvelteKit experimental `remoteFunctions` flag, custom service worker.

## Quality gates

- `cargo fmt --check` clean
- `cargo clippy --all-targets -- -D warnings` clean
- `cargo test` → **20/20** (19 unit + 1 integration EXIF-stripping)
- `pnpm check` → **0 ERRORS 0 WARNINGS**
- `pnpm test:unit` → **11/11** (6 jsonld + 5 api client)
- `pnpm test:e2e` → **24/24** (6 specs × 4 viewports)

## What's next

Project 11 — **Contact Manager + Dashboard** — first Postgres, first authentication (Argon2id + sessions + email verification + password reset), first multi-user product. The longest single jump in the curriculum.
