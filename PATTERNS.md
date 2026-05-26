# Cross-project patterns

The conventions that every project from 01 onwards follows. New projects copy these without thinking. When you're reading the curriculum and a file looks "the same" as one you've already seen, that's because **most code IS the same**, and the parts that AREN'T the same are where the lesson lives.

This doc captures the conventions explicitly so a contributor — human or AI — can scaffold a new project in minutes instead of hours.

---

## Repo layout

```
projects/NN-name/
├── README.md          One-page pitch + layout + quality gates
├── COMMANDS.md        Every shell command in execution order
├── LESSON.md          Line-by-line walkthrough of the new lessons
├── backend/           Rust crate
│   ├── Cargo.toml
│   ├── .env.example
│   ├── migrations/
│   │   └── 0001_init.sql
│   └── src/
│       ├── main.rs    Axum bootstrap + CORS + graceful shutdown
│       ├── db.rs      Pool builder
│       ├── error.rs   AppError + IntoResponse
│       ├── state.rs   (from project 08 onwards) AppState struct
│       └── routes/    One file per resource
└── frontend/          SvelteKit app
    ├── package.json
    ├── svelte.config.js
    ├── tsconfig.json
    ├── vite.config.ts
    ├── playwright.config.ts
    └── src/
        ├── app.html
        ├── app.css
        ├── test-setup.ts
        ├── lib/
        │   ├── api.ts, api.test.ts
        │   ├── types.ts
        │   └── components/
        └── routes/
            └── +layout.svelte, +page.svelte, +page.server.ts
    └── e2e/
        └── NAME.spec.ts
```

## Ports

Each project uses a unique trio so multiple can run side-by-side during development:

| Project | Backend | Dev | Preview |
| --- | --- | --- | --- |
| 01 | 3000 | 5173 | 4173 |
| 02 | 3001 | 5174 | 4174 |
| 03 | 3002 | 5175 | 4175 |
| 04 | 3003 | 5176 | 4176 |
| 05 | 3004 | 5177 | 4177 |
| 06 | 3005 | 5178 | 4178 |
| 07 | 3006 | 5179 | 4179 |
| 08 | 3007 | 5180 | 4180 |
| 09 | 3008 | 5181 | 4181 |
| 10 | 3009 | 5182 | 4182 |
| 11+ | 3010+ | 5183+ | 4183+ |

## Backend conventions

### `Cargo.toml`

Standard deps (every project):
```toml
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

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }

[profile.dev]
opt-level = 0
debug = 1
```

Add per-project crates as needed (`pulldown-cmark`, `ammonia`, `rust_decimal`, `csv`, `url`, `proptest`, `moka`, `wiremock` etc.).

### Error shape

`src/error.rs` follows the project-06+ pattern with field-level errors:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct FieldError { pub field: String, pub message: String }

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation: {0}")] Validation(String),
    #[error("validation (fields)")] Fields(Vec<FieldError>),
    #[error("not found")] NotFound,
    #[error(transparent)] Database(#[from] sqlx::Error),
    // ... add Http, Upstream, etc. as needed
}
```

`IntoResponse` returns:
```json
{ "error": { "code": "validation_failed", "message": "...", "fields": [...] | null } }
```

### `db.rs` — pool builder

Identical across projects:
```rust
let options = SqliteConnectOptions::from_str(url)?
    .create_if_missing(true)
    .journal_mode(WAL)
    .synchronous(Normal)
    .busy_timeout(Duration::from_secs(5))
    .foreign_keys(true);

let pool = SqlitePoolOptions::new()
    .max_connections(8)
    .acquire_timeout(Duration::from_secs(3))
    .connect_with(options).await?;

sqlx::migrate!("./migrations").run(&pool).await?;
```

### `main.rs` — bootstrap

Every project has the same shape:
- `tracing_subscriber::fmt().with_env_filter(...).init()`
- Read `DATABASE_URL`, `FRONTEND_ORIGIN`, `PORT` env vars with sensible defaults
- Connect pool / build AppState
- CORS with explicit origin (NOT wildcard)
- Nested routers under `/api/...`
- `/healthz` returns `"ok"`
- `TraceLayer::new_for_http()` + `CorsLayer`
- `axum::serve(...).with_graceful_shutdown(shutdown_signal())` — Ctrl-C + SIGTERM

### SQL conventions

- **Primary keys**: `TEXT PRIMARY KEY` storing UUIDv4 strings. Compile-time check sees `Option<String>` (sqlite quirk) — use `.expect("id is non-null primary key")` when unwrapping.
- **Timestamps**: `TEXT NOT NULL` storing ISO-8601 with `%.3fZ` precision. Format/parse helpers in each route file (copy from project 06).
- **Money**: `INTEGER NOT NULL CHECK (amount_minor > 0)`. Never floats.
- **Enums**: `TEXT NOT NULL CHECK (kind IN (...))`. Validate in route too.
- **Aggregates**: `COALESCE(SUM(...), 0) AS "field!: i64"` — sqlx forces the non-null type override.
- **Composite primary keys** for junction tables (no surrogate `id` column).
- **`ON DELETE CASCADE`** for owned child rows. **`ON DELETE RESTRICT`** for cross-cutting refs (e.g., postings → accounts).
- Indexes on every common filter/sort column.

## Frontend conventions

### `package.json` — devDependencies

Every project:
```json
"@axe-core/playwright": "^4.10.2",
"@playwright/test": "^1.60.0",
"@sveltejs/adapter-node": "^5.2.13",
"@sveltejs/kit": "^2.57.0",
"@sveltejs/vite-plugin-svelte": "^7.0.0",
"@testing-library/jest-dom": "^6.9.1",
"@testing-library/svelte": "^5.3.1",
"@types/node": "^25.9.1",
"@vitest/browser": "^4.1.7",
"jsdom": "^29.1.1",
"playwright": "^1.60.0",
"svelte": "^5.55.2",
"svelte-check": "^4.4.6",
"typescript": "^6.0.2",
"vite": "^8.0.7",
"vitest": "^4.1.7"
```

Dependencies (most projects):
```json
"phosphor-svelte": "^3.0.1"
```

### `tsconfig.json` — must use these strict settings

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "exclude": ["e2e/**"],
  "compilerOptions": {
    "rewriteRelativeImportExtensions": true,
    "allowJs": true,
    "checkJs": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "sourceMap": true,
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "moduleResolution": "bundler"
  }
}
```

The `noUncheckedIndexedAccess` is load-bearing — it forces null checks on array index reads.

### `vite.config.ts` — must import from `vitest/config`

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';  // NOT 'vite' — see project-01 LESSON
```

### `playwright.config.ts` — 4-viewport matrix

```ts
projects: [
  { name: 'mobile-portrait-390', use: { ...devices['Desktop Chrome'], viewport: { width: 390, height: 844 }, isMobile: false, hasTouch: true } },
  { name: 'tablet-768', use: { ...devices['Desktop Chrome'], viewport: { width: 768, height: 1024 } } },
  { name: 'laptop-1024', use: { ...devices['Desktop Chrome'], viewport: { width: 1024, height: 768 } } },
  { name: 'desktop-1440', use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } } }
]
```

Always run all 4. Mobile-first responsive design or you fail an axe-core run.

### API client (`src/lib/api.ts`)

Every project:
```ts
const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:NNNN';
type FetchLike = typeof fetch;

async function request<T>(fetcher: FetchLike, path: string, init?: RequestInit): Promise<T> {
  // ... fetch + content-type + 204 short-circuit + error mapping ...
}

export class ApiCallError extends Error {
  status: number;
  fields: { field: string; message: string }[] | null;  // from project 06+
}
```

### Vitest unit tests — the strict-type fetcher dance

In strict mode, Vitest's `mock.calls` types collide with `noUncheckedIndexedAccess`. The idiomatic workaround:

```ts
const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
```

The double cast through `unknown` is documented in project-01's LESSON.

### Playwright + SvelteKit hydration race

`gotoHydrated` helper in every spec:
```ts
async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}
```

`waitForLoadState('networkidle')` is the reliable signal that SvelteKit hydration has run. Without it, clicks on reactive buttons can be dropped before the handler is attached.

### Unique test data

Tests that mutate a shared DB (i.e., E2E specs against a single backend) must use unique names to avoid cross-test bleed:
```ts
function uniq(prefix: string): string {
  return `${prefix} ${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}
```

### A11y is a gate, not a check

Every project's Playwright suite includes:
```ts
test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});
```

Failures block the build. The second arg to `expect` is the assertion message — when it fails, the full violations JSON is printed, not just `expected [] got [Array]`.

## Shared design tokens

`shared/design-tokens.css` provides the CSS custom properties every project uses. Don't modify it for a single project's styling needs — solve them with the existing tokens, or if absolutely necessary, add a token. The shared file passes WCAG 2 AA contrast (`--color-fg-muted` was darkened in project 02 specifically to clear axe-core's contrast check).

## Quality gates

A project is NOT done until all of these pass:

| Gate | Command | Expected |
| --- | --- | --- |
| Rust format | `cargo fmt --check` | exit 0 |
| Rust lint | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Rust tests | `cargo test` | all pass |
| TS check | `pnpm check` | `0 ERRORS 0 WARNINGS` |
| Frontend unit | `pnpm test:unit` | all pass |
| E2E | `pnpm test:e2e` | 4 viewports × specs all pass |
| Svelte autofixer | MCP tool on each `.svelte` | only documented warnings remain |

Document any accepted `svelte-autofixer` warnings in LESSON.md with justification.

## Teaching files — README / COMMANDS / LESSON

- **README**: pitch (one paragraph), new lessons (bulleted), stack delta vs previous projects, quality-gate counts, "what's next".
- **COMMANDS**: every shell command in execution order. Backend first (scaffold, build, smoke), then frontend, then Playwright. Include reset-DB instructions.
- **LESSON**: section A for backend, section B for frontend, section C for tests, section D ("what you can do now"). For each section, show the code block then explain line-by-line. The depth bar: a learner who types every command and reads the LESSON can rebuild the project from memory.

## What MUST NOT change across projects

- The CORS origin must be the specific frontend origin, never `*`.
- The default port for backend should never collide with another project's preview port.
- The `tsconfig.json` strict settings.
- The Playwright 4-viewport matrix.
- The `shared/design-tokens.css` palette unless you have a WCAG-related reason and it doesn't break any prior project's tests.

## Common pitfalls and their fixes

- **"This reference only captures the initial value of `data`"** — Svelte 5 warning. Either silence with `// svelte-ignore state_referenced_locally` (when intentional) or refactor to `let x = $state(''); $effect(() => { x = data.foo; })`.
- **`pnpm check` fails with "Object literal may only specify known properties, and 'test' does not exist"** — your `vite.config.ts` imports `defineConfig` from `'vite'`. Fix: import from `'vitest/config'`.
- **Playwright clicks dropped on reactive buttons** — hydration race. Use `await page.waitForLoadState('networkidle')` after `goto`.
- **`getByLabel` matches multiple elements** — your label text appears elsewhere (e.g., as `aria-label="Tags"` on a list inside a card). Scope to the form: `panel.getByLabel(/^Title/)`.
- **`<canvas role="img">` fails axe-core** — canvas is implicitly interactive. Wrap in `<div role="img" aria-label="...">` instead.
- **`SUM(...)` returns `Option<i64>`** — empty aggregate is NULL. `COALESCE(SUM(...), 0) AS "field!: i64"` forces non-null.
- **sqlx `query!` macro errors on `SELECT 1`** — type infers as `()`, which doesn't implement Decode. Use `SELECT id` from a known column instead.

## Adding a new project

1. Copy the previous project's directory as a starting template.
2. Update Cargo.toml package name, port numbers, env defaults, CORS origin.
3. Write the migration; bootstrap with `sqlite3 NAME.db < migrations/0001_init.sql`.
4. Build out the new lessons in order: pure modules first (testable in isolation), then routes, then frontend, then E2E.
5. Run gates iteratively as you go, not just at the end.
6. Write the teaching files LAST so they reflect what was actually built.
