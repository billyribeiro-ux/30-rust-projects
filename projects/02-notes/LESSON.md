# Project 02 — Lesson

> Read this **alongside** the code you've just typed. Each section names the file, shows the code, then walks line-by-line through why it is the way it is.

Project 01 taught you the spine — Rust + Axum + sqlx + SQLite on the backend, SvelteKit + Svelte 5 runes + plain CSS on the frontend, tests at every layer. This project layers four new ideas on that spine:

1. **Markdown rendering with server-side HTML sanitization.** This is the XSS lesson. You will learn why "the backend renders, the backend sanitizes" is non-negotiable, and what the consequences are when teams skip it.
2. **The full SEO baseline.** `<svelte:head>`, canonical URLs, OpenGraph, Twitter cards, JSON-LD structured data, `prerender = true` for the marketing page. Every public page on every project from here on follows this template.
3. **Per-route error containment with `<svelte:boundary>`** and a custom `+error.svelte`. One broken note doesn't blank the whole page.
4. **A11y enforcement in CI** via `@axe-core/playwright`. WCAG 2 AA on every public page on every viewport, every test run. Regressions caught before merge.

If you skipped Project 01, go back. This lesson assumes you understand `$state`, `$derived`, `$props`, `sqlx::query!`, `IntoResponse`, the `app.css` design-token layer, and the 4-viewport Playwright matrix.

---

## A. Backend

### A.1 — `Cargo.toml`

`projects/02-notes/backend/Cargo.toml`:

```toml
[package]
name = "notes-backend"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
axum = { version = "0.8", features = ["macros"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal", "net"] }
tower-http = { version = "0.6", features = ["cors", "trace", "set-header"] }
sqlx = { version = "0.8", default-features = false, features = [
  "runtime-tokio", "tls-rustls", "sqlite", "macros", "migrate", "chrono",
] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
pulldown-cmark = { version = "0.13", default-features = false, features = ["html"] }
ammonia = "4"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }

[profile.dev]
opt-level = 0
debug = 1
```

What's new vs project 01:

- **`pulldown-cmark = "0.13"`** — a fast pure-Rust CommonMark parser. We turn off default features (it has many) and opt into only `html` (the HTML renderer). This keeps the dep tree lean.
- **`ammonia = "4"`** — the HTML sanitizer. Used by Mozilla and many production Rust services. It parses HTML with `html5ever` (the same parser Servo uses) and only emits whitelisted tags/attributes/protocols.

Why two crates, not one? `pulldown-cmark` does *not* sanitize; it happily renders inline `<script>` tags that appear in Markdown. We need a separate sanitizer step. This is the correct architecture: render, then sanitize. Never trust the renderer to be a sanitizer.

### A.2 — `migrations/0001_init.sql`

```sql
CREATE TABLE IF NOT EXISTS notes (
    id          TEXT PRIMARY KEY,
    slug        TEXT NOT NULL UNIQUE,
    title       TEXT NOT NULL,
    body_md     TEXT NOT NULL,
    body_html   TEXT NOT NULL,
    excerpt     TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notes_updated_at ON notes (updated_at DESC);
```

Lessons in this schema:

- **Both `body_md` and `body_html` are stored.** The Markdown is the user's source of truth (so editing roundtrips perfectly). The sanitized HTML is what we send to the browser. Storing both costs disk but saves a render+sanitize on every read — and avoids the worse failure mode where a sanitizer-version upgrade silently changes rendered output for old notes.
- **`excerpt` is stored, not computed.** Same reasoning. The plain-text excerpt feeds `<meta name="description">` and `<meta property="og:description">`, plus the list view. Computing it on every read is wasteful and inconsistent (changing the excerpt logic later would change every note's SEO).
- **`slug TEXT NOT NULL UNIQUE`** — the database enforces uniqueness. We allocate slugs in the application layer (next section), but if our app logic has a bug, the DB blocks the duplicate insert with a `UNIQUE constraint failed` error. **Defense in depth**: never rely on application logic alone for invariants the DB can enforce.
- **`CREATE INDEX … ON notes (updated_at DESC)`** — the list view orders by `updated_at DESC`. Without this index, SQLite would do a full table scan + sort. With it, it walks the index in order. Costs one B-tree per row; pays off the first time you have 200 notes.

### A.3 — `src/error.rs`

Same shape as Project 01, with one new variant:

```rust
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            AppError::Database(err) => {
                tracing::error!(error = ?err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        };

        let body = Json(json!({
            "error": { "code": code, "message": self.to_string() }
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

The `Conflict` variant maps to HTTP 409. We'll use it when the slug allocator can't find a unique slug after 1000 attempts (the only realistic way this happens is malicious abuse — see A.6). 409 is the HTTP signal for "your request was well-formed but conflicts with current server state".

### A.4 — `src/db.rs`

Identical to Project 01. Reference back if you need the line-by-line. The key facts to remember:

- `journal_mode(Wal)` — Write-Ahead Logging, so readers don't block writers and vice versa.
- `foreign_keys(true)` — SQLite respects foreign-key constraints **only** if explicitly enabled. We don't have FKs in this project, but enable the flag anyway for consistency and so the discipline becomes automatic.
- `busy_timeout(5s)` — when contention happens, retry for up to 5 seconds before giving up.
- `sqlx::migrate!("./migrations").run(&pool).await?;` — apply pending migrations on every boot. Idempotent. Tracked in the `_sqlx_migrations` table.

### A.5 — `src/markdown.rs` (the security-critical module)

`projects/02-notes/backend/src/markdown.rs`:

```rust
use pulldown_cmark::{Options, Parser, html as cmark_html};

pub struct Rendered {
    pub html: String,
    pub excerpt: String,
}

pub fn render(md: &str) -> Rendered {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(md, opts);
    let mut raw = String::with_capacity(md.len() * 2);
    cmark_html::push_html(&mut raw, parser);

    let safe = sanitizer().clean(&raw).to_string();
    let excerpt = make_excerpt(&safe, 160);

    Rendered { html: safe, excerpt }
}

fn sanitizer() -> ammonia::Builder<'static> {
    let mut b = ammonia::Builder::default();
    b.link_rel(Some("noopener noreferrer nofollow"));
    b.url_relative(ammonia::UrlRelative::Deny);
    b
}
```

Read this carefully — every line is load-bearing.

#### The two-phase render

`render()` does two things and **only** two things:

1. **Parse and render Markdown to HTML** with `pulldown-cmark`. This step is **untrusted**: it cheerfully passes through any inline HTML the user wrote (Markdown spec allows this).
2. **Clean the HTML with `ammonia`**. Only after this step is the HTML safe to send to a browser.

We never let unsanitized HTML reach the database, let alone the browser. The function returns a `Rendered { html, excerpt }` pair, and the route handler stores both alongside the original Markdown.

#### Why these `pulldown-cmark` options?

- `ENABLE_STRIKETHROUGH` — `~~text~~` becomes `<del>text</del>`. Standard GitHub-flavoured Markdown.
- `ENABLE_TABLES` — pipe tables.
- `ENABLE_TASKLISTS` — `- [ ] item` checkboxes.
- `ENABLE_SMART_PUNCTUATION` — `"foo"` → `“foo”`, `--` → `–`, `...` → `…`. Looks nicer in published prose.

The defaults are intentionally minimal because every extension adds parsing cost and surface area. Opt in.

#### Why `ammonia::Builder::default()` and then two overrides?

`ammonia::Builder::default()` ships with a whitelist that already covers our needs (headings, paragraphs, lists, code blocks, links, images, etc.) and **excludes** everything dangerous (`<script>`, `<iframe>`, `<style>`, `<object>`, every `on*` attribute, every URL scheme except `http`, `https`, `mailto`, etc.).

Two refinements on top:

- **`.link_rel(Some("noopener noreferrer nofollow"))`** — every `<a>` we emit gets `rel="noopener noreferrer nofollow"`.
  - `noopener` prevents the linked page from accessing `window.opener` (a classic phishing vector).
  - `noreferrer` strips the Referer header so the destination doesn't learn where the click came from.
  - `nofollow` tells search engines not to pass authority. Useful for a user-generated-content site: keeps spammers from gaming us.
- **`.url_relative(ammonia::UrlRelative::Deny)`** — reject relative URLs entirely. A user-supplied Markdown link like `[click](/admin)` is suspicious in our context; we only allow absolute URLs. If you ever want relative links (say, internal cross-references), use `UrlRelative::PassThrough` (allows) or `UrlRelative::RewriteWithBase` (rewrites against a known base).

#### The XSS test suite

```rust
#[test]
fn strips_script_tags() {
    let r = render("<script>alert('xss')</script>hello");
    assert!(!r.html.contains("<script"));
    assert!(!r.html.to_lowercase().contains("alert"));
    assert!(r.html.contains("hello"));
}

#[test]
fn strips_event_handlers() {
    let r = render("<a href=\"/x\" onclick=\"steal()\">x</a>");
    assert!(!r.html.contains("onclick"));
}

#[test]
fn strips_javascript_urls() {
    let r = render("[bad](javascript:alert(1))");
    assert!(!r.html.contains("javascript:"));
}

#[test]
fn adds_noopener_to_links() {
    let r = render("<a href=\"https://example.com\">x</a>");
    assert!(r.html.contains("rel=\"noopener noreferrer nofollow\""));
}
```

These four are non-negotiable. If you ever swap sanitizers, refactor the pipeline, or upgrade `ammonia`, these tests catch a regression *before* it ships. This is what "security as a test" looks like — make the dangerous behaviour fail at CI time, not at incident-response time.

Try this experiment after you've got the project green: delete the `sanitizer().clean(&raw).to_string()` line and replace it with just `raw`. Three of the four tests turn red immediately. Now you've felt the cost of forgetting.

#### `make_excerpt` and the off-by-one trap

```rust
fn make_excerpt(html: &str, max_chars: usize) -> String {
    let mut buf = String::with_capacity(max_chars * 4 + 8);
    let mut count = 0usize;
    let mut in_tag = false;
    let mut prev_space = false;
    for ch in html.chars() {
        if in_tag {
            if ch == '>' { in_tag = false; }
            continue;
        }
        if ch == '<' { in_tag = true; continue; }
        if ch.is_whitespace() {
            if !prev_space && count > 0 {
                buf.push(' ');
                count += 1;
                prev_space = true;
                if count >= max_chars { buf.push('…'); break; }
            }
            continue;
        }
        prev_space = false;
        buf.push(ch);
        count += 1;
        if count >= max_chars { buf.push('…'); break; }
    }
    buf.trim().to_string()
}
```

Strips HTML tags, collapses whitespace, truncates to `max_chars` graphemes with an ellipsis. Two implementation notes worth your attention:

- **An explicit `count` variable**, not `buf.chars().count()`. Calling `chars().count()` inside the loop would be O(n) per iteration → O(n²) total. For a 100KB note that means 10 billion char operations. Track the count incrementally.
- **The bound is checked after every push, including the space**. The first draft of this code checked only after non-space pushes — which lets one more char slip through when we pushed a space at exactly `count = max - 1`. The original test `assert!(r.excerpt.chars().count() <= 161)` caught it because the actual length was 162. Read your assertions hard; they are telling you something.

#### `slugify`

```rust
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_dash = true;
    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() { "note".to_string() } else { trimmed }
}
```

ASCII-only slugs. "Hello World!" → `hello-world`. "—Foo   Bar—" → `foo-bar`. "!!!" → `note` (fallback). The fallback case matters: an emoji-only title would otherwise produce an empty slug that breaks the URL.

For an internationalized app you'd reach for the `slug` crate (which transliterates Cyrillic, Chinese, etc. into ASCII via `unidecode`). For an English-first MVP this 12-line function is enough and has zero deps.

### A.6 — `src/routes/notes.rs`

The route handlers. Three concepts worth dwelling on:

#### The slug allocator

```rust
async fn unique_slug(pool: &SqlitePool, base: &str) -> AppResult<String> {
    for n in 0u32..1000 {
        let candidate = if n == 0 {
            base.to_string()
        } else {
            format!("{base}-{n}")
        };
        let row = sqlx::query!("SELECT id FROM notes WHERE slug = ?1", candidate)
            .fetch_optional(pool)
            .await?;
        if row.is_none() {
            return Ok(candidate);
        }
    }
    Err(AppError::Conflict("could not allocate unique slug".into()))
}
```

Read the SQL: `SELECT id` — not `SELECT 1`. Sqlx's compile-time macro infers the column type. With `SELECT 1`, the column has no declared type and sqlx falls back to `()` (unit), which doesn't implement `Decode<Sqlite>`. The compiler error is cryptic. `SELECT id` works because `id` is a real `TEXT` column. Quirk to remember.

The loop tries `base`, then `base-1`, `base-2`, …, up to `base-999`. If after 1000 candidates none is free, we return `AppError::Conflict` (HTTP 409). In practice this never fires — even if someone systematically created notes named "ideas", we'd handle 999 of them before failing. The cap exists so a malicious bot can't pin the request open.

Two notes on correctness:

- **There's a TOCTOU race** here. Between the `SELECT` and the subsequent `INSERT` in `create`, another request could grab the slug. The DB's `UNIQUE` constraint would then reject our insert with a `sqlx::Error::Database` (mapped to HTTP 500). For a single-user notes app this is fine. For a multi-tenant SaaS you'd wrap allocator+insert in a transaction with `INSERT … ON CONFLICT DO NOTHING RETURNING …` and retry the allocator. Project 23 (multi-tenant help desk) revisits this.
- **The DB still has the final word.** The `UNIQUE` index means even if our allocator has a bug, we never insert a duplicate. Defense in depth.

#### The `create` handler

```rust
async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateNote>,
) -> AppResult<(StatusCode, Json<Note>)> {
    let title = normalize_title(&payload.title)?;
    let body_md = normalize_body(&payload.body_md)?;
    let rendered = render(&body_md);

    let id = Uuid::new_v4().to_string();
    let slug = unique_slug(&pool, &slugify(&title)).await?;
    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        r#"
        INSERT INTO notes (id, slug, title, body_md, body_html, excerpt, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
        "#,
        id, slug, title, body_md, rendered.html, rendered.excerpt, now_str,
    )
    .execute(&pool)
    .await?;

    let note = Note { id, slug, title, body_md, body_html: rendered.html, excerpt: rendered.excerpt, created_at: now, updated_at: now };
    Ok((StatusCode::CREATED, Json(note)))
}
```

Order of operations matters:

1. **Validate the inputs** (`normalize_title`, `normalize_body`). Reject early. Never run expensive work (Markdown render, DB write) on a payload that's going to fail.
2. **Render and sanitize Markdown** **before** the DB write. This puts the sanitization on the write path, not the read path. Every read is just a `SELECT` — cheap and consistent.
3. **Allocate a unique slug** (one DB roundtrip per attempt — usually one).
4. **Insert**, with the sanitized HTML and the computed excerpt.
5. **Return the full `Note`** to the client so it can navigate to the new URL without a follow-up `GET`.

The `?7, ?7` in the SQL — same parameter index twice — saves binding the timestamp twice. Tiny optimization but reads cleanly.

#### `update` and the no-op preservation

```rust
let new_title = match payload.title.as_deref() {
    Some(t) => normalize_title(t)?,
    None => existing.title,
};
let new_body_md = match payload.body_md.as_deref() {
    Some(b) => normalize_body(b)?,
    None => existing.body_md,
};
let rendered = render(&new_body_md);
```

`PATCH` semantics: only update what was provided. If the caller sends only `{ "title": "New title" }`, we keep the existing body. If neither field is provided, we still re-render (because we trust the sanitizer is the source of truth, and re-rendering with a newer ammonia version is the right behavior). Re-rendering is cheap, so we always do it.

### A.7 — `src/main.rs`

Identical pattern to Project 01 but with three changes:

- Module list adds `mod markdown;`.
- Default DB filename is `notes.db`; default port is `3001` (so backend 02 doesn't clash with backend 01 if you have both running).
- Default `FRONTEND_ORIGIN` is `http://localhost:5174` (dev) and CORS allows that origin.

Read `src/main.rs` once. Nothing in it is new. The lesson is: **boilerplate that works should stay boring**. Every project from here onwards uses the same `main.rs` shape (mod list + tracing init + DB connect + CORS + router + serve + graceful shutdown). If yours doesn't, your project will have unique production bugs.

---

## B. Frontend

### B.1 — `package.json`

Two additions over project 01:

- **`marked`** (dependency) — Markdown renderer for the *client-side preview only*. Importantly, the rendered notes that hit the page are pre-rendered by the **backend** with ammonia; `marked` only previews what the user is currently typing. The user can only XSS themselves with their own preview, so this is safe.
- **`@axe-core/playwright`** (devDependency) — the official Deque axe rule engine wrapped as a Playwright fixture. Reports WCAG 2 A and AA violations as test failures.

Every other dep is unchanged from project 01.

### B.2 — Configs (skim)

`svelte.config.js`, `tsconfig.json`, `vite.config.ts`, `.gitignore` — identical to project 01. Reference back if needed. The lesson there explained:

- `runes: ({ filename }) => …` — enables runes mode for our project files, leaves `node_modules` alone (so non-runes deps still compile).
- `strict` + `noUncheckedIndexedAccess` + `noImplicitOverride` — the TypeScript settings that catch real bugs without false positives.
- `defineConfig` imported from **`vitest/config`**, not `vite` — the `test` key is unknown to the vanilla `vite` re-export.

`playwright.config.ts` is the same 4-viewport matrix, just on port `4174` (so it doesn't clash with project 01's `4173`).

### B.3 — `src/lib`

#### `types.ts`

```ts
export type NoteSummary = {
  id: string;
  slug: string;
  title: string;
  excerpt: string;
  updated_at: string;
};

export type Note = {
  id: string;
  slug: string;
  title: string;
  body_md: string;
  body_html: string;
  excerpt: string;
  created_at: string;
  updated_at: string;
};

export type ApiError = {
  error: { code: string; message: string };
};
```

The split between `NoteSummary` (the list view's row shape) and `Note` (the full record) is the lesson: **don't ship body_md/body_html over the wire when you only need the title and excerpt.** Bandwidth and memory both matter. The backend's `list` endpoint returns `NoteSummary`; only `read` returns the full `Note`.

#### `api.ts`

Same shape as project 01's `todosApi`. The interesting line is the URL encoding:

```ts
read: (fetcher: FetchLike, slug: string) =>
  request<Note>(fetcher, `/api/notes/${encodeURIComponent(slug)}`),
```

The slug is allocated by us (so it's already URL-safe), but we still `encodeURIComponent` for defense-in-depth: if a future change loosened the slug rules, the API client wouldn't suddenly start producing broken URLs.

#### `markdown.ts`

```ts
import { marked } from 'marked';

marked.setOptions({ gfm: true, breaks: false });

/**
 * Client-side preview render. The author is previewing their own input,
 * so XSS is not a concern here (they would only attack themselves).
 * Persistence + display of others' notes is rendered by the backend,
 * which sanitizes with ammonia. Never feed `preview()` output into the DOM
 * of a page that shows OTHER users' notes.
 */
export function preview(md: string): string {
  return marked.parse(md, { async: false }) as string;
}
```

Read the docstring. Twice. **This function is safe for preview; it is not safe for display of stored content.** In this project, `preview()` is only used in `/notes/new` on the user's own input. The `/notes/[slug]` page renders the backend's pre-sanitized HTML — not this function's output. If a future contributor refactors `[slug]/+page.svelte` to use `preview()`, every stored note becomes an XSS vector. The docstring is the warning.

The `{ async: false }` cast comes from marked's API: by default it returns `Promise<string>` to support async extensions. Synchronous mode is faster and the result type is just `string` (with a manual cast).

#### `api.test.ts`

Six tests; same shape as project 01's. The pattern worth noting:

```ts
const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
expect(init.method).toBe('POST');
expect(init.body).toBe(JSON.stringify({ title: 'Note', body_md: 'hi' }));
```

Same `unknown` double-cast as project 01 — strict TypeScript + Vitest mock typing collide here, the cast through `unknown` is the idiomatic escape. The destructuring `[, init]` skips the URL (first element of the tuple) because this test only cares about the request init.

### B.4 — Components

#### `Icon.svelte`

Identical to project 01.

#### `NoteCard.svelte`

```svelte
<script lang="ts">
  import type { NoteSummary } from '$lib/types';

  type Props = { note: NoteSummary };
  let { note }: Props = $props();

  const updated = $derived(formatDate(note.updated_at));

  function formatDate(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '';
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }
</script>

<article class="card">
  <a href="/notes/{note.slug}" class="link">
    <h2>{note.title}</h2>
    <p class="excerpt">{note.excerpt}</p>
    <time class="updated" datetime={note.updated_at}>{updated}</time>
  </a>
</article>
```

Three small ideas:

- **Whole-card link**: the `<a>` wraps the whole card content. The CSS gives it `display: block; color: inherit;` so it looks like a card, but the entire surface is clickable and is one focusable element (vs. having a separate "Read more" link). Better keyboard navigation, fewer tab stops.
- **`<time datetime={iso}>`** is the semantic HTML for dates. The human-readable text inside (`May 24, 2026`) is for display; the machine-readable `datetime` attribute is for screen readers, calendars, search engines, and `<time>`-parsing libraries.
- **`-webkit-line-clamp: 2; line-clamp: 2;`** in the CSS truncates the excerpt to 2 lines with ellipsis. Both prefixed and unprefixed because browser support is uneven and the unprefixed version is the future-proofing.

The first version of this file used `color: var(--color-fg-subtle)` for `.updated` — axe-core caught a 2.4:1 contrast ratio (fails WCAG AA). Switched to `--color-fg-muted` and adjusted the muted token to `hsl(220 9% 38%)` in `shared/design-tokens.css` to meet 4.5:1. **A11y enforcement caught a real bug.** Worth the cost.

### B.5 — `+layout.svelte`

```svelte
<script lang="ts">
  import '../app.css';
  import { Notepad } from 'phosphor-svelte';
  import Icon from '$lib/components/Icon.svelte';

  let { children } = $props();
</script>

<svelte:head>
  <title>Notes — Markdown drafts that won't get lost</title>
  <meta name="description" content="A tiny, fast Markdown notes app. Draft, sanitize, share. Your ideas, in one place." />
  <link rel="canonical" href="https://notes.example.com/" />
  <meta property="og:type" content="website" />
  <meta property="og:title" content="Notes — Markdown drafts that won't get lost" />
  <meta property="og:description" content="A tiny, fast Markdown notes app. Draft, sanitize, share." />
  <meta name="twitter:card" content="summary" />
</svelte:head>

<a class="skip-link" href="#main">Skip to main content</a>

<nav class="topnav" aria-label="Primary"> … </nav>

<div id="main">
  {@render children()}
</div>
```

The `<svelte:head>` in the layout sets the **defaults** for every page. Per-page `<svelte:head>` blocks override the title and description. The pattern means a page only needs to provide what differs.

The first line in the body is a **skip-link**:

```svelte
<a class="skip-link" href="#main">Skip to main content</a>
```

Hidden until focused (CSS pulls it `translateY(-150%)` off-screen by default). A keyboard user lands on it as the first focusable element on every page, and pressing Enter jumps past the nav to the `#main` div. This is a WCAG 2.4.1 requirement (Bypass Blocks). It's free to add and a huge a11y win.

`<nav aria-label="Primary">` — the label disambiguates this nav from any other (like a footer nav). Screen readers announce "Primary navigation, list, 3 items".

### B.6 — Home page (`/`)

```svelte
<script lang="ts">
  import { fade } from 'svelte/transition';
  …
  let { data }: PageProps = $props();
  const notes = $derived(data.notes);
</script>

…

{#each notes as note (note.id)}
  <li in:fade={{ duration: 180 }}>
    <NoteCard {note} />
  </li>
{/each}
```

**First `svelte/transition`** in the curriculum. `in:fade` runs only when the element *enters* the DOM (when the page first renders, when a new note is created). 180ms is the duration; it's short enough to feel snappy, long enough to register as a transition.

`{ duration: 180 }` is the only option we set. `fade` also accepts `delay` and `easing`. Default easing is `cubicOut`, which is what you want most of the time.

Why use a transition at all on the initial render? Because when the user creates a new note and SvelteKit re-runs `load`, the new note appears in the list — and the fade signals "this is the new one." Without the fade it would just pop in. Subtle, but the kind of polish that distinguishes the apps you remember.

Note that this respects the **`prefers-reduced-motion`** override in `shared/design-tokens.css`: the design tokens set `--duration-fast`, `--duration-base`, `--duration-slow` to `0ms` when the user has reduced-motion preference. The Svelte transition's `duration: 180` doesn't pick that up automatically — for full a11y you'd read the preference and short-circuit the transition. Project 19 (kanban tracker, the first heavy-motion project) shows the canonical pattern. For this project, 180ms is gentle enough to not bother people.

### B.7 — `/about` — the prerendered marketing page

`projects/02-notes/frontend/src/routes/about/+page.ts`:

```ts
export const prerender = true;
```

That one line tells SvelteKit: at build time, render this page to a static HTML file. At request time, the server just serves the file. No JavaScript runs server-side. Response in milliseconds. Cacheable forever at the edge.

You can verify it worked by looking at `.svelte-kit/output/prerendered/pages/about.html` after `pnpm build`. The HTML is fully rendered, including the JSON-LD `<script>` tag.

Why is this important? The marketing page is what Google indexes, what Twitter scrapes for the link preview, what people see first. It should load instantly even when the backend is down. Prerendering trades a bit of build time for production reliability and SEO.

#### The JSON-LD pattern

```svelte
<svelte:head>
  …
  {@html `<script type="application/ld+json">${JSON.stringify(jsonld)}</` + `script>`}
</svelte:head>
```

Why the awkward string concatenation `</` + `script>`? Because if the source file contains the literal substring `</script>`, **Vite's HTML parser** treats it as the end of the surrounding Svelte `<script>` tag and chaos follows. Splitting it across two strings prevents that.

The `{@html …}` is necessary because SvelteKit's `<svelte:head>` doesn't allow a literal `<script>` child element (it would interpret it as a Svelte component script). `@html` injects raw markup. **And** — important — this `@html` is safe because the input is `JSON.stringify(jsonld)` where `jsonld` is a static object literal we control. There's no user input here. This pattern is the documented way to add JSON-LD in SvelteKit.

The JSON-LD itself follows schema.org. `@type: 'WebApplication'` tells Google "this is a web app", which influences how it appears in search results. The `offers` block, with `price: '0'`, is the magic that gets the "Free" badge in some search surfaces.

### B.8 — `/notes/new` — the editor with live preview

The big idea here is the **Edit / Preview tab toggle**:

```svelte
let mode = $state<'edit' | 'preview'>('edit');
const previewHtml = $derived(mode === 'preview' ? preview(body_md) : '');
```

`mode` is the only `$state` that controls which subtree renders. `previewHtml` is `$derived` so that it only recomputes when *either* `mode` or `body_md` changes. When mode is `'edit'`, `preview()` is not called at all — no wasted work.

The template:

```svelte
{#if mode === 'edit'}
  <label class="field">
    <span>Body (Markdown)</span>
    <textarea name="body_md" bind:value={body_md} … ></textarea>
  </label>
{:else}
  <input type="hidden" name="body_md" value={body_md} />
  <section class="preview" aria-label="Markdown preview">
    {#if body_md.trim().length === 0}
      <p class="hint">Nothing to preview yet.</p>
    {:else}
      <div class="prose">{@html previewHtml}</div>
    {/if}
  </section>
{/if}
```

The subtle bit: when we switch to Preview mode, the `<textarea>` leaves the DOM (form submission would lose the value). We replace it with `<input type="hidden" name="body_md" value={body_md} />` so the form still submits the user's content. The `body_md` `$state` is the source of truth either way.

The `aria-label="Markdown preview"` on the `<section>` gives screen readers a meaningful label without a visible heading.

**The `@html previewHtml` is safe because `previewHtml` is the user's own current input.** If they manage to inject `<script>`, they're only XSS-ing themselves. The moment they save, the backend takes the raw `body_md` (not the preview HTML) and runs *its* sanitizer. The browser preview is never written to the database.

#### A11y wart: tabs without `aria-controls`

I cheated slightly on the tab pattern. A fully ARIA-compliant tablist requires each tab to have `aria-controls="<panel-id>"` and the panel to have `role="tabpanel"`. I omitted the panel-id binding because the panel changes shape between edit and preview, and the complexity wasn't worth it for a 2-tab toggle. Axe doesn't fail on this. A purist would tighten it. The tradeoff is documented here so you can choose.

### B.9 — `/notes/[slug]` — render the sanitized HTML

The single most important line in this project:

```svelte
<div class="prose">{@html note.body_html}</div>
```

Read it again. `{@html}` is rendering server-sanitized HTML straight into the DOM. This is **safe** because, and only because:

- The HTML was generated by `pulldown-cmark` on the backend.
- The HTML was passed through `ammonia` on the backend before being stored.
- The HTML in the database is therefore guaranteed safe HTML, validated by our XSS test suite (A.5).
- We render it untouched on the frontend — we don't run a *second* sanitizer (which would risk double-encoding, e.g., turning `&amp;` into `&amp;amp;`).

Any time you write `{@html user_input}` you must be able to point to the sanitization step that made `user_input` safe. If you can't, you have an XSS bug. This is the most important security lesson in the entire curriculum so far.

#### `<svelte:boundary>` for per-route resilience

```svelte
<svelte:boundary onerror={(e) => console.error('note render boundary', e)}>
  <main> …happy path… </main>

  {#snippet failed(err, reset)}
    <main>
      <div class="boundary-error" role="alert">
        <h2>Something went wrong rendering this note.</h2>
        <p>{(err as Error).message}</p>
        <button type="button" onclick={reset}>Try again</button>
      </div>
    </main>
  {/snippet}
</svelte:boundary>
```

What `<svelte:boundary>` does:

- If anything inside the boundary throws during render or in an effect, Svelte catches it.
- The `onerror` handler runs (here, just a console log — in production you'd ship to Sentry).
- The `failed` snippet renders with the error and a `reset` callback.
- The user gets a recoverable in-page error instead of a blank page.

The pattern is "**graceful degradation at the smallest meaningful unit**". The layout, the nav, the page header still render. Only the note body falls back. The user can click "Try again" to retry the render without leaving the page.

For comparison, `+error.svelte` (next section) handles errors *thrown during `load`*. `<svelte:boundary>` handles errors during *render*. Both exist because failures happen at both stages.

#### The Delete form with `confirm()`

```svelte
<form
  method="POST"
  action="?/remove"
  onsubmit={(e) => {
    if (!confirm(`Delete "${note.title}"? This cannot be undone.`)) e.preventDefault();
  }}
>
  <button type="submit" class="danger">Delete</button>
</form>
```

Browser-native confirmation. Two reasons to prefer it over a custom modal here:

1. **Zero JS** if disabled. `confirm()` works without our `onsubmit` running, but the form still submits — which is bad. So actually you'd want server-side double-submit protection in a real app. For an MVP this is fine.
2. **Visually obvious** to the user that this is a destructive action. The native dialog is impossible to miss.

In Project 11 (contact manager) we'll switch to a proper in-page confirm dialog with `<dialog>` element + focus trap. The progression matches real-world product evolution.

### B.10 — `+error.svelte` (the 404 page)

```svelte
<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head>
  <title>{page.status} — Notes</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<main>
  <p class="status">{page.status}</p>
  <h1>{page.error?.message ?? 'Something went wrong'}</h1>
  …
</main>
```

SvelteKit auto-renders this whenever a route's `load` throws or calls `error()`. Our `+page.server.ts` does:

```ts
if (err instanceof ApiCallError && err.status === 404) {
  error(404, 'Note not found');
}
```

That triggers this `+error.svelte` with `page.status = 404` and `page.error.message = 'Note not found'`. The user sees a friendly page instead of a blank screen.

`<meta name="robots" content="noindex" />` tells search engines not to index 404 pages — they shouldn't be in the search index pretending to be real content.

The custom error page lives at `notes/[slug]/+error.svelte`, so it only handles errors for that route. Putting one at the root (`src/routes/+error.svelte`) would be the global fallback for all routes. We could add one later when there are more route trees.

---

## C. Tests

### C.1 — Vitest unit tests

`src/lib/api.test.ts` covers the API client: list, read, create (with body verification), 204 handling, error throwing with status, slug URL encoding. Six tests, same shape as project 01. Vitest auto-discovers them via the `include: ['src/**/*.test.ts']` in `vite.config.ts`.

If you've forgotten the `unknown` double-cast trick, look back at project 01's LESSON, section C.

### C.2 — Playwright + axe-core

The new piece. The whole spec is in `e2e/notes.spec.ts`. The key import:

```ts
import AxeBuilder from '@axe-core/playwright';
```

A typical a11y test:

```ts
test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa'])
    .analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});
```

`AxeBuilder({ page })` injects the axe-core script into the page. `.withTags(['wcag2a', 'wcag2aa'])` restricts the rule set to WCAG 2 Level A and AA (skipping AAA, which is stricter than most products target). `.analyze()` runs the rules and returns a result object.

The assertion is `.toEqual([])` — zero violations. The `JSON.stringify(...)` second arg to `expect()` is the **assertion message**: if it fails, Playwright prints the full violations dump, not just "expected `[]` got `[<huge object>]`". Without it you'd be debugging blind.

#### The hydration race — `gotoHydrated`

```ts
async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}
```

SvelteKit ships an SSR'd HTML page first; only after the JS bundle loads does it hydrate (attach event handlers, run `$effect`, etc.). Playwright's auto-waiting *does* wait for elements to be visible, enabled, and stable — **but it does not wait for hydration**. If a test clicks a reactive button before hydration completes, the click hits a DOM element that has no event handler, and the click is silently dropped.

`waitForLoadState('networkidle')` is the conventional fix: wait until the page has gone 500ms without any network activity. By that point the bundle is loaded and hydration has run. The 5–10ms cost on a local network is negligible; the test reliability boost is enormous.

Tests that *only* read the DOM (like the "empty title blocks save" test) don't need to wait — the disabled state was rendered server-side.

#### The lifecycle test

```ts
test('full lifecycle: create, view, delete', async ({ page }) => {
  await gotoHydrated(page, '/notes/new');
  await page.getByLabel(/title/i).fill('Notes E2E sample');
  await page.getByLabel(/body/i).fill('# A heading\n\nThis is a **bold** sample.\n\n- one\n- two');
  await page.getByRole('button', { name: /save note/i }).click();

  await expect(page).toHaveURL(/\/notes\/notes-e2e-sample(?:-\d+)?$/);
  await expect(page.getByRole('heading', { level: 1, name: 'Notes E2E sample' })).toBeVisible();
  await expect(page.getByRole('heading', { level: 1, name: 'A heading' })).toBeVisible();
  await expect(page.locator('strong', { hasText: 'bold' })).toBeVisible();

  const noteAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(noteAxe.violations, JSON.stringify(noteAxe.violations, null, 2)).toEqual([]);

  await page.waitForLoadState('networkidle');
  page.once('dialog', (d) => d.accept());
  await page.getByRole('button', { name: /delete/i }).click();
  await expect(page).toHaveURL('/');
});
```

- **The URL regex `/\/notes\/notes-e2e-sample(?:-\d+)?$/`** accepts the bare slug *or* a numbered variant. Repeat runs of the test (without a DB reset) accumulate notes named "Notes E2E sample", so the second run gets `notes-e2e-sample-1`, the third `notes-e2e-sample-2`, etc. The regex tolerates this so the test stays green across runs.
- **The strong-tag assertion** (`page.locator('strong', { hasText: 'bold' })`) proves that the Markdown was actually rendered to HTML. If the backend skipped rendering, we'd see literal `**bold**` text. This is the end-to-end proof that the whole pipeline works.
- **`page.once('dialog', (d) => d.accept())`** registers a one-shot handler for the next browser dialog (our `confirm()`). Without it, Playwright would block on the dialog and time out.

The other four specs are simpler: a11y on home, a11y on about, preview tab renders, validation blocks empty title, 404 page shows for unknown slug. Read them all.

---

## D. SEO checklist + Lighthouse interpretation

Open `http://localhost:4174/about` and run Lighthouse from DevTools. You should see:

- **Performance ≥ 98** — prerendered HTML, no JS shipped for this page beyond the layout's nav.
- **A11y 100** — axe is happy (and so is Lighthouse, which uses a subset of axe).
- **Best Practices 100** — no console errors, HTTPS not required on localhost, no deprecated APIs.
- **SEO 100** — title length OK, description length OK, canonical present, viewport meta present, JSON-LD present.

Then run Lighthouse against the home page (`/`). You should see:

- **Performance ≥ 95** — slightly less than prerendered, because the page is SSR'd on each request and ships hydration JS.
- **A11y 100, SEO 100, Best Practices 100** — same as `/about`.

And against a note page (`/notes/<slug>`):

- **Performance ≥ 95** — SSR + hydration; the note's HTML is already in the response, no waterfalls.
- **A11y 100** — assuming the note's Markdown doesn't include something pathological (like a heading at level 8 — but ammonia strips those anyway).
- **SEO 100** — title, description, canonical, OpenGraph, JSON-LD Article all set per the layout + page heads.

If a number drops, the audit tells you what failed. Common gotchas (and the fix):

- **LCP > 2.5s** — usually a font, an image, or a render-blocking CSS file. Inline critical CSS (we already do, via the layout import).
- **CLS > 0.1** — images without `width`/`height`, web fonts swapping, ads. We have none of these.
- **INP > 200ms** — JavaScript work in the click handler. Ours just sets state, so this is a non-issue.

Project 27 (realtime analytics dashboard) revisits Lighthouse with more demanding budgets. For now, getting the four 95+/100 numbers is the bar.

---

## E. Closing — what you can do now

By the end of this project, you can:

- Render and store user-provided Markdown safely. You understand why "render then sanitize" is the rule, and you have automated tests that fail if a future change weakens the pipeline.
- Ship a full SEO baseline on every public page: title, description, canonical, OpenGraph, Twitter card, JSON-LD. Lighthouse SEO 100 is the default, not a celebration.
- Prerender static pages with `export const prerender = true`. You know how to inspect the output.
- Contain in-page errors with `<svelte:boundary>` and route-load errors with `+error.svelte`. Your apps degrade gracefully — broken parts stay broken, working parts stay working.
- Enforce WCAG 2 AA via `@axe-core/playwright` in CI. You can read a Lighthouse a11y report and fix what it tells you. You understand the hydration race and the `waitForLoadState('networkidle')` fix.
- Allocate URL slugs with collision retry, knowing both the DB-level uniqueness constraint and the application-level allocator are there for a reason.

Open `projects/03-habits/COMMANDS.md` when you're ready. Habit tracker, calendar grid, complex SQL window functions, property-based testing with `proptest`. We keep climbing.
