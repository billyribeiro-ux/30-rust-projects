# Curriculum — 30 Projects, Zero to Principal Engineer L7+

> A staircase. Each step is one or two new ideas. By project 30 you can architect, ship, and operate a full multi-tenant SaaS in the way a Distinguished Principal Engineer at Apple / Google / Microsoft / Netflix would.

## How to read this document

For every project you get:

- **Problem** — the real-world pain this product solves.
- **Stack additions** — what is new versus the prior project.
- **Concepts introduced** — the *one or two* lessons this project is really about.
- **Database** — SQLite, Postgres, with which schema.
- **Auth** — none / Argon2 sessions / JWT / OAuth / 2FA / SSO.
- **External services** — Stripe, MinIO, Redis, MeiliSearch, etc.
- **Testing additions** — what new test category appears.
- **DevOps additions** — what new infra primitive (Docker, CI step, migration discipline).
- **Estimated hours** — a calibration for your own pace.
- **Learning outcomes** — what you can build *by yourself* after this project.
- **Dependencies** — which prior projects' code/patterns are reused.

The file is long. That is the point. A syllabus that fits on a postcard is a syllabus you cannot execute.

---

## Shared stack (applies to every project)

- **Frontend**: SvelteKit 2 + Svelte 5 (runes) + TypeScript strict (`noUncheckedIndexedAccess`, `noImplicitOverride`), plain CSS with cascade layers, Phosphor icons via `phosphor-svelte`, mobile-first responsive at 390 / 768 / 1024 / 1440 px.
- **Backend**: Rust 2024 edition, Axum 0.8+, tokio, `sqlx` with compile-time-checked queries (`sqlx::query!`), `thiserror` + `IntoResponse`, `tracing` + `tracing-subscriber` (JSON in prod), graceful shutdown on SIGTERM.
- **Quality**: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `pnpm sv check`, Vitest, Playwright at 4 viewports, Lighthouse ≥ 95 on public projects.
- **Teaching artefacts per project**: `README.md` (pitch), `COMMANDS.md` (every shell command), `LESSON.md` (line-by-line walkthrough). Production source code stays comment-light; teaching lives in `LESSON.md`.
- **Versioning**: latest stable as of May 2026, pulled at scaffold time via `pnpm dlx sv@latest create` and `cargo add <crate>`. Lockfiles committed.
- **Svelte MCP `svelte-autofixer`** must report 0 issues on every `.svelte` file before a project is declared done.

## Database strategy

- **Projects 01–10 — SQLite** (`sqlx::SqlitePool`). Zero-install, file-based. Perfect for one user + one machine. WAL mode, foreign keys on, busy timeout. **For some early data-entry-heavy projects you'll also see how Drizzle ORM would solve the same problem in a SvelteKit-only world** (sidebar contrast, not the main path — the main path remains `sqlx::query!`).
- **Projects 11–30 — Postgres 16** via Docker Compose. Production-grade. As features warrant we add **Redis** (project 17, rate limits + cache), **MinIO** (project 13, S3-compatible object store), **MeiliSearch** (project 24, typo-tolerant search), **MailHog → Resend** (project 11, email).
- **Migrations**: `sqlx migrate` from project 01. Every schema change is a forward migration committed to git. Zero-downtime patterns (expand → backfill → contract) taught in project 22.

## Auth strategy (the Principal-level way)

The teaching ladder mirrors how real enterprise systems grow:

- **Projects 01–10** — no auth. Single-user products. Avoids learning two things at once. (10 SQLite projects in a row build deep fluency in everything *except* auth.)
- **Project 11** — **first auth**: registration + login + logout + email verification + password reset. Argon2id password hashing (`argon2` crate, OWASP 2025 parameters: m_cost 19456, t_cost 2, p_cost 1) + **server-side sessions** (opaque 32-byte tokens stored in Postgres, `HttpOnly; Secure; SameSite=Lax` cookie, 30-day sliding expiry). The default for browser-only apps because it's revocable in one query. This is the foundation reused unchanged in projects 12–30.
- **Project 12** — **bcrypt taught as the legacy-system migration**: an imaginary older table used bcrypt; we dual-verify on login and re-hash to Argon2 on success. The real-world "we acquired a company and inherited their hashes" scenario every Principal engineer faces.
- **Project 13** — **JWT (HS256 then RS256)** for stateless API auth, with proper short-lived access tokens (15 min) + refresh tokens stored in DB. Compared against sessions: when to use which.
- **Project 17** — **2FA via TOTP** (`totp-rs`, QR code via `qrcodegen`, backup codes hashed with Argon2). Required for admin roles.
- **Project 20** — **OAuth 2.0 + PKCE** to Google and GitHub, written by hand the first time so you know the flow, then refactored to `openidconnect` crate.
- **Project 23** — **magic links** (passwordless), email-token sign-in.
- **Project 26** — **SAML / SSO** for enterprise tenants (`samael` crate), JIT user provisioning.
- **Project 28** — **WebAuthn / Passkeys** (`webauthn-rs` crate). The 2026 enterprise expectation.
- **Project 30 (capstone)** — **all of the above, configurable per tenant**, plus audit log, session device list, account takeover detection (impossible-travel checks), GDPR data export.

Every auth-bearing project ships: rate-limited login (token bucket — in-memory `DashMap` until Redis arrives), constant-time password verify, generic "invalid credentials" message (no user-enumeration), email verification, password reset, account deletion with audit trail, CSRF protection via SvelteKit's same-origin check + double-submit token.

## Stripe progression (every step harder, every step real)

Stripe is taught as a series of escalating real-world e-commerce / SaaS scenarios. Each step builds on the last.

- **Project 16** — **Stripe Checkout (one-time)**: digital-product storefront. Webhook → fulfilment → email receipt. Idempotency keys. Test-mode workflow with Stripe CLI. Hand-verified webhook signatures.
- **Project 21** — **Stripe Subscriptions**: newsletter platform with free + paid tiers. Customer Portal. Dunning email flow. Free-trial logic. Plan upgrades / downgrades / proration. Cancel-at-period-end.
- **Project 25** — **Stripe Connect (Express accounts)**: course marketplace. Instructor onboarding (Stripe-hosted KYC). Application fees. Payout dashboard. Refund policy enforcement. Transfer reversals.
- **Project 28** — **Stripe Billing — metered usage**: AI API platform. Usage events posted from worker. Tiered pricing (first 10k requests free, then $0.001 each). Usage caps. Per-tenant invoice line items.
- **Project 30** — **full SaaS billing**: plans + add-ons + seats, proration, tax (Stripe Tax), invoicing, dunning + grace period + downgrade, billing-portal SSO, multi-currency, accounting export to QuickBooks-equivalent CSV.

## GSAP cinematic motion (where Hollywood-grade animation actually serves the product)

- **Project 19** — Kanban board, GSAP timelines for drag-drop physics.
- **Project 24** — image gallery, ScrollTrigger for cinematic scroll.
- **Project 27** — realtime analytics dashboard, GSAP number-roll + stagger reveals on KPI cards.
- **Project 29** — cinematic portfolio + CMS — heavy GSAP, ScrollTrigger pin sequences, SplitText, Lenis smooth scroll, View Transitions.
- **Project 30** — capstone marketing site with cinematic hero, all GSAP techniques combined.

Every GSAP usage respects `prefers-reduced-motion`. No animation for animation's sake.

## SEO baseline (every public project, taught from project 02)

- Semantic HTML, one `<h1>` per page, landmark roles.
- `<svelte:head>` with title (≤ 60 char), description (≤ 155), canonical, OpenGraph, Twitter card, JSON-LD (Article, Product, FAQPage, Organization, Person where applicable).
- `prerender = true` for marketing/static pages (adapter-static fallback or hybrid).
- Generated `sitemap.xml` and `robots.txt`.
- Core Web Vitals: INP < 200ms, LCP < 2.5s, CLS < 0.1.
- 2026 Google updates: AI Overview answer-first content blocks, FAQ schema, `hreflang`, Image SEO with descriptive filenames + alt text + `<picture>` with srcset, accessibility = 100 (Lighthouse).

---

# The 30 projects

## Tier 1 — Foundations (SQLite, no auth)

### Project 01 — TODO Manager

- **Problem**: "I forget what I need to do today."
- **Status**: ✅ Shipped — use this as your reference for what every project's output looks like.
- **Stack additions**: every primitive — SvelteKit, Axum, sqlx, SQLite, Phosphor, plain CSS, Vitest, Playwright.
- **Concepts**: first Svelte 5 runes (`$state`, `$derived`, `$props`), first Axum handler, first `sqlx::query!`, first migration, first form action, first Vitest test, first Playwright test (4 viewports).
- **Database**: SQLite. One table: `todos (id, title, done, created_at, updated_at)`.
- **Auth**: none.
- **External services**: none.
- **Testing**: backend unit (3 tests), frontend unit (4 tests), E2E (8 tests across 4 viewports).
- **DevOps**: `.gitignore`, `.editorconfig`, env via `.env.example`.
- **Hours**: 2.
- **You can now**: build a CRUD app top-to-bottom and explain every layer.
- **Depends on**: nothing.

### Project 02 — Markdown Notes

- **Problem**: "I draft ideas in seven different apps and lose them all."
- **Stack additions**: `marked` (TS markdown render), `ammonia` (Rust HTML sanitizer), prerendering.
- **Concepts**: full SEO baseline, `<svelte:head>` deep, `<svelte:boundary>` for per-route error containment, server-side markdown sanitization (the XSS lesson), `prerender = true` for the marketing page, first `svelte/transition` (`fade`).
- **Database**: SQLite. `notes (id, title, body_md, body_html, created_at, updated_at)`.
- **Auth**: none.
- **Testing**: snapshot test for HTML sanitization, Playwright a11y assertions via `@axe-core/playwright`.
- **DevOps**: Lighthouse CI added to the local test script.
- **Hours**: 3.
- **You can now**: render user-provided rich text safely, ship an SEO-perfect public page.
- **Depends on**: 01.

### Project 03 — Habit Tracker with Streaks

- **Problem**: "I want to build a daily habit and need to see my streak."
- **Stack additions**: CSS Grid mastery for the calendar.
- **Concepts**: `$derived` for streak calculation, complex SQL (CTE + LAG for streak windows), keyboard navigation in a grid, `prefers-reduced-motion` honored.
- **Database**: SQLite. `habits`, `habit_completions (habit_id, date)`.
- **Auth**: none.
- **Testing**: property-based tests with `proptest` introduced — streak invariants (any sequence of completions yields the right streak).
- **Hours**: 3.
- **You can now**: model time-series data and write window-function-equivalent SQL.
- **Depends on**: 01, 02.

### Project 04 — Pomodoro Timer + Session History

- **Problem**: "I lose focus and want enforced 25-minute blocks with a record."
- **Concepts**: `$effect` with cleanup, `requestAnimationFrame`, audio API, `$inspect` for debugging reactivity, optimistic UI pattern (small dose), `<svelte:window>` for keyboard shortcuts.
- **Database**: SQLite. `sessions (started_at, ended_at, kind, label)`.
- **Auth**: none.
- **Hours**: 3.
- **You can now**: drive UI from time without leaking intervals.
- **Depends on**: 01–03.

### Project 05 — Bookmark Manager with Tags

- **Problem**: "Browser bookmarks are unsearchable; I save links and never see them again."
- **Stack additions**: `SvelteURL`/`SvelteURLSearchParams` from `svelte/reactivity`.
- **Concepts**: many-to-many SQL (`bookmarks ↔ tags` join table), URL-state filters (`?tag=rust&q=axum`), debounced search, `use:clickOutside` action, link-preloading on hover.
- **Database**: SQLite. `bookmarks`, `tags`, `bookmark_tags`.
- **Auth**: none.
- **Testing**: Vitest component test for the search debounce.
- **Hours**: 4.
- **You can now**: design relational data and turn URL into the source of truth.
- **Depends on**: 01–04.

### Project 06 — Expense Splitter (single household)

- **Problem**: "Roommates can't track who owes who."
- **Concepts**: form actions with validation (`valibot` on client, `validator` + `serde` server), money as `i64` cents (no floats — *ever*), `$state.snapshot` for serializing form payloads, `$bindable` for a reusable money input, custom `IntoResponse` error mapping with field-level errors.
- **Database**: SQLite. `members`, `expenses (payer_id, amount_cents, split_kind)`, `expense_shares (expense_id, member_id, share_cents)`.
- **Auth**: none.
- **Testing**: property-based test — for any split, sum of shares == total.
- **Hours**: 4.
- **You can now**: handle money correctly, accept untrusted form input safely.
- **Depends on**: 01–05.

### Project 07 — Personal Finance Ledger

- **Problem**: "I don't know where my money goes."
- **Stack additions**: `rust_decimal`, CSV import (`csv` crate).
- **Concepts**: double-entry accounting basics (debit + credit must sum to zero), `$derived.by` for multi-step ledger derivations, charts drawn on a `<canvas>` from scratch (no chart library), reading reactivity via `$effect.tracking()`, last SQLite project before Postgres.
- **Database**: SQLite. `accounts`, `transactions`, `postings (transaction_id, account_id, amount_decimal, side)`.
- **Auth**: none.
- **Hours**: 5.
- **You can now**: model business-critical numerical data; draw your own visualizations.
- **Depends on**: 01–06.

### Project 08 — Reading Tracker

- **Problem**: "Goodreads is bloated; I want to track what I read and what I learned."
- **Stack additions**: `moka` cache, `reqwest` for outbound API calls.
- **Concepts**: external API integration (Open Library API), server-side caching, `$state.raw` for large immutable cached blobs, streamed `load` returns (book metadata streams in after the title), import progress UI, classes with rune fields (`class Book { progress = $state(0); coverUrl = $derived(...) }`), `<svelte:boundary>` for graceful API-failure handling.
- **Database**: SQLite. `books`, `reading_sessions`, `highlights`.
- **Auth**: none (single-user).
- **Testing**: snapshot tests for the Open Library parser; `wiremock-rs` for outbound HTTP in tests.
- **Hours**: 5.
- **You can now**: integrate third-party APIs without making your UI slow or your tests flaky.
- **Depends on**: 01–07.

### Project 09 — Workout Logger with PR Detection

- **Problem**: "I can't tell if I'm getting stronger."
- **Concepts**: complex `$derived` chains, indexed exercise search (SQLite FTS5), axe-core in CI pipeline (a11y regression gate), CSV export, **visual regression tests** with Playwright `toHaveScreenshot()` for the PR badge.
- **Database**: SQLite. `exercises`, `workouts`, `sets (workout_id, exercise_id, weight_cents, reps, rir)`, FTS5 virtual table.
- **Auth**: none (single-user).
- **Hours**: 5.
- **You can now**: model events, derive insights, gate a11y + visual regressions in CI.
- **Depends on**: 01–08.

### Project 10 — Recipe Book with Photos

- **Problem**: "My recipes live in 14 screenshots; I want one searchable cookbook."
- **Stack additions**: **first file upload** (multipart, image-only, mime-sniffed server-side, written to local disk via `tokio::fs`), `image` crate for thumbnail generation, **first remote function** (`$app/server`) for the photo upload UX.
- **Concepts**: multipart parsing in Axum, EXIF stripping for privacy, snippets (`{#snippet}`/`{@render}`) for reusable recipe cards, Recipe JSON-LD for Google rich results, **first service worker** (offline recipe reading), **last SQLite project** — transitions us toward Postgres deliberately.
- **Database**: SQLite. `recipes`, `recipe_images`, `ratings`.
- **Auth**: none. Public sharing via signed URLs with expiry.
- **DevOps**: dev script that resizes images on upload; `cargo run --bin seed` for sample data.
- **Hours**: 7.
- **You can now**: handle file uploads safely; ship rich-result-eligible SEO content.
- **Depends on**: 01–09.

## Tier 2 — Postgres, Auth, Real-time, Files, First Stripe (11–18)

### Project 11 — Contact Manager + Dashboard — **First Postgres, First Auth, the foundation reused forever**

- **Problem**: "My contacts are scattered across 4 apps. I want one place with relationships, notes, last-contacted dates, and reminders — with login so my data is mine."
- **Stack additions**: **Postgres 16 via Docker Compose** (multi-container: postgres + mailhog), **first authentication** (Argon2id + sessions + email verification + password reset), `lettre` for email, `hooks.server.ts` for the auth handle, `+layout.server.ts` with parent inheritance.
- **Concepts**: KPI tile dashboard (recent contacts, follow-ups due, this-week interactions), full-text search via Postgres `tsvector`, soft-delete + restore, contact merge UI, CSV/vCard import, tag system, the **permission-matrix test pattern** (each role × each action × each contact owner) — the Principal lesson "if you can't list your permissions, you don't have an auth system".
- **Database**: Postgres. `users`, `sessions`, `email_tokens`, `password_reset_tokens`, `contacts`, `interactions`, `reminders`, `tags`, `contact_tags`, materialised view for search.
- **Auth**: sessions (Argon2id + opaque 32-byte tokens, `HttpOnly; Secure; SameSite=Lax`, 30-day sliding expiry). This is the reusable auth module for projects 12–30.
- **External services**: Postgres, MailHog. Resend for production.
- **Testing**: `sqlx::test` macro for per-test ephemeral Postgres DBs; full auth-flow E2E; permission-matrix unit tests.
- **DevOps**: `docker-compose.yml` template that every project from 11 onwards extends; `sqlx prepare` for offline CI compile; first GitHub Actions workflow.
- **Hours**: 12 (the longest single jump in the curriculum — the auth + Postgres + dashboard combination is the most concept-dense lesson).
- **You can now**: build a multi-user product the secure, modern way; ship a real dashboard.
- **Depends on**: 01–10.

### Project 12 — Job Application Tracker (+ legacy bcrypt migration lesson)

- **Problem**: "I lose track of which company I applied to, with whom, and what's the next step."
- **Concepts**: status timeline UI, background tokio task for email reminders, ICS calendar export, **`bcrypt` taught as the legacy migration scenario** — we pretend an older system used bcrypt; new logins re-hash with Argon2 (live migration pattern Principal engineers face in real life).
- **Database**: Postgres. `applications`, `application_events`, `next_steps`.
- **Auth**: sessions + the **Argon2 ↔ bcrypt dual-verify** pattern.
- **Hours**: 6.
- **You can now**: migrate sensitive data without forcing user logouts.
- **Depends on**: 11.

### Project 13 — Real-time Chat Rooms (+ first JWT)

- **Problem**: "Slack is overkill for a 3-person project."
- **Stack additions**: **WebSockets** via `axum::extract::ws`, `tokio::sync::broadcast`, **MinIO** for image attachments.
- **Concepts**: presence tracking, message persistence + scrollback, `$effect.root` for chat connection lifetime, `<svelte:window>` for resize/online detection, **JWT introduced** for the bot/integration API (so a chatbot can post messages with a stable token), comparison: when sessions vs when JWT.
- **Database**: Postgres. `rooms`, `messages`, `memberships`. MinIO bucket `chat-uploads`.
- **Auth**: sessions for humans, JWT (HS256, then RS256 in project 17) for bots.
- **Testing**: WebSocket integration tests with `tokio-tungstenite` client.
- **Hours**: 8.
- **You can now**: design real-time products; choose auth strategy by audience.
- **Depends on**: 11–13.

### Project 14 — Calendar & Scheduler

- **Problem**: "My partner and I keep double-booking."
- **Stack additions**: `rrule` crate (RRULE recurrence), `chrono-tz`.
- **Concepts**: recurrence-rule expansion server-side, timezone-correct event display client-side, shared calendars with row-level permissions, drag-to-create event UX.
- **Database**: Postgres. `calendars`, `events`, `event_attendees`, `event_overrides`.
- **Auth**: sessions, with **resource-based authorization** centralized in `authorize(actor, action, resource)` (introduced here, reused everywhere after — Principal-level pattern).
- **Hours**: 8.
- **You can now**: handle time correctly across users and zones.
- **Depends on**: 11–13.

### Project 15 — File Vault with Chunked Uploads

- **Problem**: "Dropbox free tier is too small and I want full control of my files."
- **Stack additions**: `object_store` crate (S3-compatible), virus scanning hook (ClamAV in compose, optional).
- **Concepts**: presigned URLs to MinIO, resumable uploads (tus-style protocol implemented from scratch), mime sniffing, EXIF stripping, folder tree UI, content-addressed storage (dedup via SHA-256).
- **Database**: Postgres. `files`, `file_versions`, `folders`.
- **Auth**: sessions.
- **Hours**: 8.
- **You can now**: handle large file uploads at production scale.
- **Depends on**: 13.

### Project 16 — Digital Product Storefront — **Stripe Checkout #1**

- **Problem**: "I want to sell a PDF or video file with a checkout link, deliver it after payment, and never touch a card number."
- **Stack additions**: **Stripe Checkout (one-time payment)**, `stripe-rust` crate, webhook handler with raw-body Axum extractor.
- **Concepts**: webhook signature verification, idempotency keys, fulfilment after `checkout.session.completed`, email delivery of signed download links (24h expiry), customer email-only flow (no account required), refund processing.
- **Database**: Postgres. `products`, `orders`, `order_items`, `download_links`, `stripe_events` (for idempotency).
- **Auth**: optional account; download links are signed.
- **External services**: **Stripe (test mode + Stripe CLI)**, MinIO.
- **Testing**: Stripe CLI webhook fixtures replayed in integration tests; `wiremock-rs` to mock Stripe API in CI (no live calls).
- **Hours**: 9.
- **You can now**: accept money on the internet, the boring correct way.
- **Depends on**: 11, 15.

### Project 17 — URL Shortener + Analytics (+ Redis, + 2FA)

- **Problem**: "My marketing team needs branded short links with real-time analytics, and abuse protection."
- **Stack additions**: **Redis** (rate-limit token bucket, cache), MaxMind GeoLite geo lookup, **2FA via TOTP** for the admin panel.
- **Concepts**: high-throughput Axum (no SSR overhead — pure JSON), bot filtering by user-agent rules, click analytics aggregation (`tsrange` + `date_trunc`), Core Web Vitals beacon endpoint, **2FA** enrollment + backup codes + recovery flow.
- **Database**: Postgres. `links`, `clicks` (partitioned by day), `users`, `totp_secrets`, `backup_codes`. Redis for rate limits + hot link cache.
- **Auth**: sessions + 2FA required for admin role.
- **Testing**: `bombardier` smoke load test (≥ 5k rps redirect path).
- **Hours**: 9.
- **You can now**: protect a public endpoint; require 2FA for sensitive operations.
- **Depends on**: 11, 13.

### Project 18 — Polls & Surveys with Live Results

- **Problem**: "Showing live audience answers during conference talks."
- **Concepts**: **Postgres `LISTEN/NOTIFY` → SSE** bridge in Axum, animated bar charts using `Spring`/`Tween` from `svelte/motion` (Svelte 5 modern motion API), QR-code generation for join links, anonymous voting with fingerprint dedup.
- **Database**: Postgres. `polls`, `questions`, `options`, `votes`.
- **Auth**: sessions for poll creators; anonymous voters get a signed cookie.
- **Hours**: 6.
- **You can now**: push live updates from DB to browser without WebSockets.
- **Depends on**: 11, 13.

## Tier 3 — Animation, payments, scaled real-world products

### Project 19 — Kanban Issue Tracker (GSAP #1)

- **Problem**: "Linear costs money my side project can't justify."
- **Stack additions**: **GSAP** (loaded client-side, respecting `prefers-reduced-motion`), drag-and-drop.
- **Concepts**: `animate:flip` from `svelte/animate`, **GSAP timelines** for cinematic card-drop physics, `{@attach}` attachments (the modern action replacement) for the drag handle, optimistic mutations with rollback on server error, `$effect.pre` for measuring DOM before paint.
- **Database**: Postgres. `boards`, `lists`, `cards`, `assignments`, `comments`.
- **Auth**: sessions; project-level RBAC (viewer / editor / admin).
- **Hours**: 10.
- **You can now**: design tactile, animated UI that respects accessibility.
- **Depends on**: 11, 14.

### Project 20 — Geo-aware Restaurant Finder (+ OAuth)

- **Problem**: "Yelp's UX is hostile and I trust my friends' recommendations more."
- **Stack additions**: **PostGIS**, MapLibre GL JS, **OAuth 2.0 + PKCE** (Google + GitHub, written by hand for the first project, then refactored to `openidconnect`).
- **Concepts**: `ST_DWithin` queries, "near me" with permission graceful degradation, server-rendered first paint of search results for SEO, shallow routing for map popups.
- **Database**: Postgres + PostGIS. `places (geom GEOGRAPHY)`, `reviews`, `friend_recs`, `oauth_accounts`.
- **Auth**: sessions + OAuth — taught as the protocol (PKCE, state, nonce) not just a library call.
- **Hours**: 10.
- **You can now**: integrate identity providers correctly; ship a geo product.
- **Depends on**: 11.

### Project 21 — Newsletter Platform — **Stripe Subscriptions #2**

- **Problem**: "I want to charge $5/mo for my newsletter without paying Substack 10%."
- **Stack additions**: **Stripe Subscriptions**, Customer Portal, RSS feed.
- **Concepts**: subscription lifecycle webhooks (`customer.subscription.created/updated/deleted`), proration on upgrade, free-trial logic, **dunning email flow** (past_due → 14-day grace → auto-downgrade), entitlement middleware (`require_plan(Plan::Pro)`), gated posts.
- **Database**: Postgres. `subscribers`, `subscriptions` (mirror of Stripe state), `posts`, `post_reads`, `stripe_events`.
- **Auth**: sessions; subscribers also use magic link (project 23 pattern teased here).
- **External services**: Stripe + Stripe CLI + Resend.
- **Hours**: 12.
- **You can now**: ship a real recurring-revenue product.
- **Depends on**: 16, 17.

### Project 22 — Background Jobs Dashboard

- **Problem**: "I need to know which of my nightly jobs failed."
- **Concepts**: job queue built on Postgres `SKIP LOCKED` (no Redis dependency), retry with exponential backoff + jitter, worker isolation per queue, dashboard with real-time job stream (SSE), **OpenTelemetry tracing** exported to Tempo/Jaeger, **zero-downtime schema migrations** (expand → backfill → contract) introduced here.
- **Database**: Postgres. `jobs (id, queue, payload, attempts, run_at, locked_until, ...)`, `job_results`.
- **Auth**: sessions, admin-only.
- **Hours**: 10.
- **You can now**: build a real worker tier and observe it in production.
- **Depends on**: 11, 18.

### Project 23 — Multi-tenant Help Desk (+ Magic Links)

- **Problem**: "Small SaaS needs to ticket support with per-customer isolation."
- **Stack additions**: **Postgres Row-Level Security**, tenant subdomain routing, **magic-link passwordless auth** for end-customers.
- **Concepts**: RLS policies enforced at DB level (auth bug → no data leak), tenant model (`tenants`, `memberships`, `invitations`), role-based ACL (admin/agent/customer), SLA breach alerts, canned responses, agent assignment, **audit log table via Postgres trigger** (so the app cannot skip it).
- **Database**: Postgres with RLS. `tenants`, `memberships`, `tickets`, `ticket_messages`, `audit_log`.
- **Auth**: sessions + magic links + invitation tokens.
- **Testing**: parameterized permission-matrix tests across (role × action × resource × tenant).
- **Hours**: 12.
- **You can now**: build a multi-tenant SaaS the safe way.
- **Depends on**: 11, 21.

### Project 24 — Hybrid Search Knowledge Base

- **Problem**: "Internal docs are unsearchable; the search is worse than grep."
- **Stack additions**: **MeiliSearch** for typo-tolerant + semantic, Postgres `tsvector` + `pg_trgm` for keyword.
- **Concepts**: hybrid retrieval (BM25 + dense), query parser, highlighted snippets, breadcrumbs + Article JSON-LD, full SEO playbook (Hreflang, FAQ schema, AI-Overview answer block).
- **Database**: Postgres + MeiliSearch index.
- **Auth**: sessions; published articles are public (SEO matters).
- **Hours**: 10.
- **You can now**: ship search that actually helps users find things.
- **Depends on**: 11, 23.

## Tier 4 — Distinguished work (multi-tenant, enterprise auth, full billing)

### Project 25 — Course Marketplace — **Stripe Connect #3**

- **Problem**: "Instructors want to sell courses on my platform and get paid out."
- **Stack additions**: **Stripe Connect Express accounts**, HLS video transcoding (`ffmpeg` from tokio).
- **Concepts**: Express account onboarding (Stripe-hosted KYC), application fees, payout schedule + dashboard, refund policy enforcement, instructor balance, video upload + HLS segmentation + signed manifest URLs.
- **Database**: Postgres. `instructors`, `courses`, `lessons`, `enrollments`, `connect_accounts`, `payouts`.
- **Auth**: sessions, OAuth, role-based (instructor / student / admin).
- **External services**: Stripe Connect, MinIO, ffmpeg.
- **Hours**: 14.
- **You can now**: run a marketplace with split payments.
- **Depends on**: 15, 16, 21, 23.

### Project 26 — Live Coding Interview Platform (+ SAML/SSO)

- **Problem**: "I run technical interviews and need a shared editor + safe code execution + recording."
- **Stack additions**: CodeMirror 6 via `{@attach}` (the modern Svelte action replacement), **CRDT** (`yrs` — Yjs Rust port) over WebSocket, sandboxed code execution (`docker run --rm` per submission, gVisor/firecracker noted), **SAML / SSO** for enterprise customers (`samael` crate).
- **Concepts**: collaborative editing without conflicts (CRDT theory), interviewer/candidate role split, session recording + playback, code execution sandbox hardening (resource limits, time limits, network isolation).
- **Database**: Postgres. `interviews`, `participants`, `executions`, `recordings`, `saml_idps`, `jit_provisioning_rules`.
- **Auth**: sessions + SSO; per-tenant SSO config.
- **Hours**: 16.
- **You can now**: support enterprise customers; run untrusted code safely.
- **Depends on**: 13, 14, 23.

### Project 27 — Realtime Analytics Dashboard (GSAP #2)

- **Problem**: "Founders need a live KPI wall for the all-hands."
- **Stack additions**: **DuckDB** via `duckdb` crate for embedded OLAP, **GSAP cinematic** number-roll, gradient sweep, stagger reveals.
- **Concepts**: ingest path (events → Postgres → ETL into DuckDB), WS-pushed KPI cards, dark/light theme via `prefers-color-scheme` + CSS custom properties, `<svelte:boundary>` per widget so one broken KPI doesn't kill the page.
- **Database**: Postgres for OLTP, DuckDB for OLAP queries.
- **Auth**: sessions, viewer/editor roles.
- **Hours**: 10.
- **You can now**: design a real ingest → analytics → presentation pipeline.
- **Depends on**: 22, 23.

### Project 28 — AI Inference API — **Stripe Metered Billing #4** (+ WebAuthn/Passkeys)

- **Problem**: "I want to expose an AI endpoint and charge by usage, like OpenAI."
- **Stack additions**: **Stripe Billing meter events**, **WebAuthn / Passkeys** (`webauthn-rs`) for the developer dashboard, API-key management with hashed storage.
- **Concepts**: usage events emitted from the worker, reconciled hourly with Stripe meter, tiered pricing (first 10k free → $0.001 per call), per-tenant rate limits, usage caps with graceful 429, invoice line-item validation against local usage table, **passkey enrollment + login** (the 2026 enterprise expectation, no password fallback).
- **Database**: Postgres. `api_keys (hash, scopes, ...)`, `usage_events`, `webauthn_credentials`, `invoices_local` (reconciliation source).
- **Auth**: passkeys + API keys; admin-only TOTP.
- **External services**: Stripe Billing, an LLM endpoint (mocked in tests, real key in dev).
- **Hours**: 14.
- **You can now**: monetize a developer API end-to-end; ship modern passwordless auth.
- **Depends on**: 17, 21, 22, 25.

### Project 29 — Cinematic Portfolio + Headless CMS (GSAP #3 — heavy)

- **Problem**: "Designers want a Hollywood-grade portfolio site without paying $50/mo for a CMS."
- **Stack additions**: `mdsvex` for MDX content, **heavy GSAP** (ScrollTrigger pin sequences, SplitText word-by-word reveals, Lenis smooth scroll), **View Transitions API**.
- **Concepts**: full prerendered static site (`adapter-static` fallback), CMS panel for the designer (publishes Markdown to a Postgres-backed store, rebuilds the static site via worker), perfect Core Web Vitals (LCP < 1.5s on 4G), AI Overview answer blocks, hreflang for international, `<svelte:element>` for CMS-driven dynamic tags, `$host` for publishing the design system as web components consumable by non-Svelte sites.
- **Database**: Postgres. `posts`, `assets`, `revisions`, `webhooks`.
- **Auth**: sessions, single-user CMS.
- **Hours**: 12.
- **You can now**: ship a cinematic site that wins Awwwards and ranks #1 on Google.
- **Depends on**: 02, 22, 24.

### Project 30 — SaaS Capstone — Multi-tenant Project Management (GSAP marketing site, full Stripe billing, all auth strategies)

- **Problem**: "Combines everything you've learned into a real product you could ship and charge for tomorrow."
- **Concepts**: every primitive of the curriculum, integrated:
  - **Multi-tenant** with Postgres RLS, tenant subdomain routing.
  - **Auth menu per tenant**: passwords + sessions, magic links, OAuth (Google/GitHub), SAML for enterprise, WebAuthn passkeys, mandatory 2FA for owners.
  - **Stripe Billing**: plans + seats + add-ons + metered overages + Stripe Tax + dunning + grace + auto-downgrade + invoicing + accounting export.
  - **Projects / tasks / comments / attachments / real-time presence** (reuse 13, 14, 15, 19).
  - **Search** (reuse 24).
  - **Background jobs + observability** (reuse 22).
  - **Rate limiting + abuse detection** (reuse 17 + impossible-travel checks).
  - **Audit log via Postgres trigger** (reuse 23).
  - **GSAP cinematic marketing site** (reuse 29 patterns).
  - **Perfect SEO**, Lighthouse 100s.
  - **Deployable** in one `docker compose up`; horizontal scaling notes; Kubernetes manifest as appendix.
- **Database**: Postgres + Redis + MinIO + MeiliSearch + Tempo. Backups (`pg_dump` → S3, PITR via WAL archiving).
- **Auth**: all of the above, configurable per tenant.
- **External services**: Stripe (full), Resend, Sentry-compatible error endpoint, OTLP-compatible trace backend.
- **Testing**: full pyramid + load tests (`wrk`, `bombardier`), chaos test (drop Redis mid-request, kill a worker, etc.).
- **DevOps**: GitHub Actions CI matrix, image build, security scanning (Trivy), Dependabot/Renovate, container hardening (distroless), CSP strict-dynamic with nonces.
- **Hours**: 25–35.
- **You can now**: architect, build, ship, and operate a real SaaS. You are the senior engineer in the room.
- **Depends on**: all of the above.

---

## Cross-cutting tracks (taught across multiple projects)

| Track | Introduced | Mastered by |
| --- | --- | --- |
| Svelte 5 runes — `$state`, `$derived`, `$effect`, `$props` | 01 | 06 |
| Advanced runes — `$state.raw`, `$state.snapshot`, `$derived.by`, `$effect.pre`, `$effect.root`, `$effect.tracking`, `$bindable`, `$inspect`, `$host` | 06–29 | 30 |
| SvelteKit data — `load`, form actions, remote functions, hooks | 01–11 | 23 |
| `svelte/motion` + `svelte/transition` + `svelte/animate` + `svelte/easing` | 02 | 27 |
| Snippets / boundaries / attachments / shallow routing / snapshots / view transitions / service worker | 02–29 | 30 |
| Plain CSS — cascade layers, container queries, custom properties, View Transitions | 01 | 29 |
| Phosphor icons | 01 | 30 |
| `sqlx::query!` + migrations + zero-downtime | 01 → 22 | 30 |
| Postgres deep — `LISTEN/NOTIFY`, `SKIP LOCKED`, `tsvector`, PostGIS, RLS, partitioning | 11 → 24 | 30 |
| Testing pyramid — `cargo test`, `proptest`, `sqlx::test`, Vitest, Playwright × 4 viewports, `axe-core`, visual regression, load | 01 → 22 | 30 |
| Auth — Argon2, bcrypt-migration, sessions, JWT, 2FA, OAuth+PKCE, magic links, SAML, WebAuthn | 11 → 28 | 30 |
| Stripe — Checkout, Subscriptions, Connect, Metered, Tax, full Billing | 16, 21, 25, 28, 30 | 30 |
| Observability — `tracing`, OTLP, request IDs, Sentry-compatible errors, health probes | 22 | 30 |
| GSAP cinematic motion | 19, 24, 27, 29 | 30 |
| DevOps — Docker Compose, GitHub Actions, image scanning, secrets, Renovate, distroless | 11 → 22 | 30 |

---

## Estimated total

Roughly **240–280 hours** of focused work to complete all 30 projects. At 10 hours/week → 6 months. At 20 hours/week → 3 months. Going faster is the wrong goal. Reading the LESSON files without typing the code is the wrong goal. The point is to type every line, fail every error, read every stack trace.

By project 30, you will have written, with your own hands, every primitive a Distinguished Principal Engineer at a top-tier tech company uses on a Tuesday afternoon.

Welcome. Open `projects/01-todo/COMMANDS.md` and begin.
