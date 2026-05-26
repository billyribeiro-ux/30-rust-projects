# Project 08 — Lesson

Headline lessons:
1. **External API integration with `reqwest` + `moka` cache** — never hit the upstream twice for the same ISBN in a one-hour window.
2. **`wiremock-rs`** — test outbound HTTP without touching the real Open Library.
3. **Streamed `load` returns** — primary data flushes first; secondary data streams in after.
4. **`$state.raw`** for large arrays we replace wholesale.
5. **Class with rune fields** in a `.svelte.ts` module — modern Svelte 5 reactive object pattern.
6. **`<svelte:boundary>`** containing a failing form so one section's render error doesn't kill the page.

---

## A. Backend

### A.1 — `openlibrary.rs` — the upstream client

```rust
pub struct OpenLibrary {
    client: reqwest::Client,
    base_url: String,
    cache: Cache<String, BookLookup>,
}

impl OpenLibrary {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("reading-tracker/0.1")
            .build()
            .expect("reqwest client builds");
        let cache = Cache::builder()
            .max_capacity(1024)
            .time_to_live(Duration::from_secs(60 * 60))
            .build();
        Self { client, base_url, cache }
    }

    pub async fn lookup_isbn(&self, isbn: &str) -> AppResult<BookLookup> {
        let norm = normalize_isbn(isbn)?;
        if let Some(cached) = self.cache.get(&norm).await {
            return Ok(cached);
        }
        // ... fetch + parse + cache + return
    }
}
```

Four discipline points worth noting:

**1. Timeout, always.** `reqwest::Client::builder().timeout(...)` sets the total request timeout including DNS, TCP, TLS, body. Without it, a hung connection blocks your worker forever. 5 seconds is conservative for an ISBN lookup; real-time-sensitive paths might use 1-2 seconds.

**2. `User-Agent` header.** Open Library asks API consumers to identify themselves. It's good citizenship and helps the upstream throttle bad actors without breaking yours.

**3. `moka` cache.** `Cache::builder().max_capacity(1024).time_to_live(1h).build()` — bounded LRU with TTL. ISBN metadata doesn't change in practice, but we expire after an hour so a restart-and-look-up cycle gives "fresh" data. The cache is `async` (`.get(...).await`) because `moka::future::Cache` supports concurrent gets and writes with proper async signaling.

**4. Normalize before caching.** We `normalize_isbn` (strip hyphens, uppercase X) BEFORE hitting the cache. `978-0-13-468599-1` and `9780134685991` should hit the same cache entry.

### A.2 — `parse_lookup` — the Open Library shape

The Open Library response is heterogeneous and lightly documented. Our parser:

```rust
let key = format!("ISBN:{}", isbn);
let obj = body.get(&key).ok_or(AppError::NotFound)?;
let title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
let author = obj.get("authors").and_then(|v| v.as_array())
    .map(|arr| arr.iter().filter_map(|a| a.get("name").and_then(|n| n.as_str())).collect::<Vec<_>>().join(", "))
    .unwrap_or_default();
let cover_url = obj.get("cover")
    .and_then(|c| c.get("medium").or_else(|| c.get("large")).or_else(|| c.get("small")))
    .and_then(|v| v.as_str()).map(|s| s.to_string());
```

Idioms in play:
- **Empty-body 200 = "not found"**. Open Library returns 200 OK with `{}` when there's no match. We treat `key missing` as `AppError::NotFound`, which maps to HTTP 404 on our API.
- **Defensive `.unwrap_or_default()`**. If `authors` is missing or non-array, we get an empty string. Better than an error — the user might want to fill it in themselves.
- **Cover fallback chain.** `medium` → `large` → `small`. Some books have only small covers; some have only large. The chain ensures we use whatever's available.

The seven unit tests cover the parser exhaustively:
- happy path with all fields
- multiple authors → comma-joined
- cover falls back to small if medium/large absent
- missing key → NotFound
- missing title → Upstream (not NotFound — this is an unexpected upstream shape)

### A.3 — `wiremock-rs` integration tests

`tests/openlibrary_wiremock.rs` spins up a mock HTTP server, registers a route with expectations, and exercises the contract:

```rust
let server = MockServer::start().await;
Mock::given(method("GET"))
    .and(path("/api/books"))
    .and(query_param("bibkeys", "ISBN:9780134685991"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({ ... })))
    .expect(1)  // assert exactly one request
    .mount(&server)
    .await;
```

Three reasons this matters:
- **The real Open Library shouldn't be in your CI.** Their service has outages, rate limits, and occasionally changes shape. A mock pins your test against a known fixture.
- **Negative-path testing.** You can register a mock that returns 500, 200-with-empty-body, slow responses, etc., and verify your client handles each gracefully.
- **Request shape verification.** The `query_param(...)` matchers assert the exact URL parameters your code sends. If a refactor breaks the URL, the test fails immediately.

### A.4 — `state.rs` and the AppState

```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub openlibrary: OpenLibrary,
}
```

Axum's `with_state(state)` makes this available to every handler via `State<AppState>`. The struct is `Clone` (cheap — both fields are `Arc` internally), so handlers get their own copy on each request without contention.

This pattern scales: when we add Redis, JWT secrets, an email service, etc., they become fields on `AppState`. The router signature changes from `Router<SqlitePool>` to `Router<AppState>` once and forever.

---

## B. Frontend

### B.1 — `book-model.svelte.ts` — class with rune fields

```ts
// File name MUST end with .svelte.ts to enable rune syntax outside .svelte
// components. This is the modern Svelte 5 pattern; it replaces Svelte 4's
// "writable store + custom set/update + derived store" stack.

export class BookModel {
  id: string;
  title = $state('');
  author = $state('');
  current_page = $state(0);
  pages = $state<number | null>(null);
  // ... other $state fields

  progress = $derived(this.pages && this.pages > 0
    ? Math.min(100, Math.round((this.current_page / this.pages) * 100))
    : 0);

  constructor(book: Book) {
    this.id = book.id;
    this.title = book.title;
    // ...
  }
}
```

What this gives you:
- **Instance-scoped reactive state.** Each `new BookModel(...)` has its own `current_page` slot. Updates to one instance don't affect others.
- **Type-safe `$derived` getters.** `book.progress` is `number`, not `Readable<number>`. No store unwrap.
- **`new BookModel(initial)` is one line.** In Svelte 4 you'd need to write the constructor, separate getters/setters for each field, and a derived store for `progress`. The class is dramatically less code.

The file must end with `.svelte.ts` (not just `.ts`) — Vite/SvelteKit only enables the rune parser for those filenames. If you name it `book-model.ts`, the `$state` calls fail with "$state is not defined".

### B.2 — Where the BookModel is used

In `routes/books/[id]/+page.svelte`:

```ts
const book = new BookModel(data.book);
$effect(() => {
  // Sync from data → model whenever the page re-loads.
  book.title = data.book.title;
  book.author = data.book.author;
  book.status = data.book.status;
  book.current_page = data.book.current_page;
  book.pages = data.book.pages;
});
```

The model is created ONCE per mount. The `$effect` keeps it in sync with the page's `data` prop whenever SvelteKit reruns `load` (after a form action succeeds, for example). Local UI edits — e.g., the user typing a new `current_page` value — mutate the model directly, and the `<input bind:value={book.current_page}>` two-way binding picks them up.

The `svelte-ignore state_referenced_locally` comment on `const book = new BookModel(data.book)` acknowledges the initialization reads `data` outside a reactive scope. The `$effect` below handles updates; the initial read just snapshots the starting state.

### B.3 — `$state.raw` for large replaced arrays

```ts
let highlights = $state.raw<Highlight[]>([]);
let sessions = $state.raw<Session[]>([]);
$effect(() => {
  highlights = data.highlights;
  sessions = data.sessions;
});
```

`$state.raw` is `$state` minus the proxy wrapping. Three properties:
- **Faster reads.** No proxy traps mean reading a field is a direct property access.
- **Mutations don't trigger updates.** `highlights.push(x)` mutates the array in place but Svelte doesn't see it. The only way to update is `highlights = newArray`.
- **Frozen at runtime** (in dev mode) so accidental mutations throw immediately.

Use `$state.raw` when:
- The data is immutable from the component's perspective (server data that you only replace wholesale).
- The data is LARGE (1000+ items) and the proxy overhead matters.
- You want to enforce "only replace, never mutate" at the type-system level.

Use plain `$state` when:
- You need to mutate fields piecewise (`book.current_page = 100`).
- You want Svelte to track granular changes (e.g., updating one item in an array via index assignment).

### B.4 — Streamed `load` returns

`routes/+page.server.ts`:

```ts
export const load: PageServerLoad = async ({ fetch }) => {
  const books = await booksApi.list(fetch);
  const stats = computeStats(books);
  return {
    books,
    streamed: {
      stats: new Promise<typeof stats>((resolve) => {
        setTimeout(() => resolve(stats), 50);
      })
    }
  };
};
```

When SvelteKit sees a promise in a nested object (`streamed.stats`), it:
1. Sends the synchronous parts of the response right away (HTML up to the closing tag of the synchronous slot).
2. Streams the promise body as a separate chunk under the response's `Transfer-Encoding: chunked`.
3. Resolves the promise in the browser's `data` object as the chunk arrives.

The page uses `{#await data.streamed.stats}`:

```svelte
{#await data.streamed.stats}
  <section class="stats" aria-busy="true">
    <div class="skel"></div> ...
  </section>
{:then stats}
  <dl class="stats">
    <div><dt>Books</dt><dd>{stats.total}</dd></div>
    ...
  </dl>
{:catch err}
  <p class="error">Stats unavailable: {(err as Error).message}</p>
{/await}
```

The `:catch` branch is essential. Streamed promises can fail (network error mid-stream, server-side exception in the promise body) and you want a visible error, not a permanent loading spinner.

**When to use streamed returns**: the user can act on the primary data before secondary data finishes loading. Example: showing the books grid the moment the server has them, while expensive aggregate stats compute. For a 50ms delay (our setTimeout) it's overkill, but for a 500ms cross-service call to an analytics backend it's the difference between a 100ms TTFB and a 600ms one.

### B.5 — `<svelte:boundary>` around the lookup form

```svelte
<svelte:boundary onerror={(e) => console.error('add-book section error', e)}>
  <form method="POST" action="?/lookupIsbn" ...>
    ...
  </form>
  {#snippet failed(err, reset)}
    <div class="error" role="alert">
      <p>The book-lookup section crashed: {(err as Error).message}</p>
      <button type="button" onclick={reset}>Try again</button>
    </div>
  {/snippet}
</svelte:boundary>
```

This catches **render-time** errors inside the boundary (the form). Form-action failures are handled separately via the `lookupError` derived. The boundary is there for the "the form itself throws while rendering" case — e.g., a downstream component bug.

Boundaries should wrap **one logical section at a time**. Don't wrap the whole page in one boundary — if something throws, you blank the whole page. Wrap individual panels so the page degrades gracefully: one section fails, the rest still works.

---

## C. Tests

| Layer | Coverage |
| --- | --- |
| cargo (13 = 11 unit + 2 wiremock) | ISBN normalization (hyphens, uppercase X, length checks). Open Library parser (happy / multi-author / cover-fallback / missing key / missing title). Wiremock integration test: happy GET; empty-body 200 = not found. |
| vitest (5) | API client list/update/encoded-path/lookup; ApiCallError on upstream 502. |
| playwright (20 = 5 × 4) | a11y, manual add → grid → detail open, status tab persists across reload, streamed stats panel renders, custom 404 for unknown book id. |

---

## D. Closing — what you can do now

- Integrate any third-party JSON API with `reqwest` + a bounded `moka` cache. You understand timeouts, user-agent etiquette, and the empty-body-200 idiom.
- Test outbound HTTP without touching the real upstream via `wiremock-rs`.
- Stream secondary data with `{#await data.streamed.x}` in SvelteKit. You know when it's worth it.
- Reach for `$state.raw` vs `$state` based on usage shape (replace-wholesale vs mutate-piecewise).
- Build reusable reactive models with classes in `.svelte.ts` modules. You understand the modern Svelte 5 alternative to Svelte 4 stores.
- Contain render-time errors in narrow `<svelte:boundary>` sections so one bug doesn't blank the page.

Project 09 — **Workout Logger with PR Detection** — complex `$derived` chains, SQLite FTS5 full-text search, axe-core in CI as a gate not just a check, CSV export.
