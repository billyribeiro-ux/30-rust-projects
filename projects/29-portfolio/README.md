# Project 29 — Cinematic Portfolio + Headless CMS (GSAP #3)

A designer-portfolio site that *feels* cinematic on first paint:
GSAP intro on the hero, IntersectionObserver-driven reveal-on-scroll
for each case study, View Transitions API for route changes, and a
Postgres-backed CMS so the owner can ship from any device.

## What's new in this project

- **`{@attach}`** Svelte 5 attachment syntax to invoke `revealOnScroll`
  on each work card when it enters the viewport.
- **Lazy-loaded GSAP** — same pattern as project 27. Reduced-motion
  mode skips the import entirely.
- **CSS-only view-transition setup** — `:global(html) {
  view-transition-name: root }` is the load-bearing line. The browser
  takes care of the rest when navigation happens via standard `<a>`.
- **MDX-ish content pipeline** — `pulldown-cmark` + `ammonia` on the
  server, rendered HTML stored on write. The frontend ships zero
  Markdown parser.
- **CreativeWork JSON-LD** + hreflang per post for SEO.

## What's intentionally simplified vs the curriculum spec

The full spec called for `mdsvex`, ScrollTrigger pin sequences,
SplitText word-by-word reveals, Lenis smooth scroll. This project ships:

- Markdown via the backend (HTML pre-rendered on save), not `mdsvex`
  (which compiles MDX into Svelte components at build time).
- IntersectionObserver-driven reveal-on-scroll, not ScrollTrigger
  (which lives behind GSAP Club).
- View Transitions API, not Lenis (which adds smooth-scroll behaviour).

The pattern is the same; the upgrade path is documented in LESSON.md.

## Stack delta vs project 24

| Layer | Same | Different |
| --- | --- | --- |
| Backend | Postgres + Argon2 + Axum + sessions + Markdown sanitise | Adds CMS-style admin endpoints (create/update/publish) |
| Frontend | hooks.server.ts, prerender-friendly pages | Adds **GSAP reveal-on-scroll** + **view-transition-name** |

## Quality gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test` | **3/3 pass** (Argon2 + post lifecycle + sanitiser strips script) |
| `pnpm check` | **0 ERRORS 0 WARNINGS** |
| `pnpm test:unit` | **3/3 pass** |
| `pnpm test:e2e` | **8/8 pass** (2 specs × 4 viewports, axe-core wcag2a+aa) |
| `cargo sqlx prepare` | `.sqlx/` committed |
| Svelte autofixer | clean (one documented silence on the post page) |

## What you can do now

Ship a portfolio that *feels* premium without a JS bundle that
punishes mobile users. Add cinematic polish that respects
`prefers-reduced-motion`. Build a CMS where the editor and the
deployable site are one process.

## What's next

Project 30 — the SaaS Capstone. The integration of every primitive
in this curriculum.
