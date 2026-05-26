# Project 05 — Lesson

> Read alongside the code. The headline lessons are **many-to-many SQL** (the right shape, the right joins, the right transactions) and **URL-as-state** (the URL is the source of truth for filters; everything else derives from it).

---

## A. Backend

### A.1 — `Cargo.toml`

One new dep:

```toml
url = "2.5"
```

The `url` crate parses RFC 3986 URLs into structured form. We use it to validate that bookmark URLs are absolute and have `http`/`https` scheme — rejecting `javascript:`, `file:`, `data:`, etc. before they ever touch the DB.

### A.2 — `migrations/0001_init.sql` — the many-to-many shape

```sql
CREATE TABLE IF NOT EXISTS bookmarks (
    id           TEXT PRIMARY KEY,
    url          TEXT NOT NULL,
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    id    TEXT PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS bookmark_tags (
    bookmark_id  TEXT NOT NULL,
    tag_id       TEXT NOT NULL,
    PRIMARY KEY (bookmark_id, tag_id),
    FOREIGN KEY (bookmark_id) REFERENCES bookmarks(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_updated_at ON bookmarks (updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_bookmark_tags_tag ON bookmark_tags (tag_id);
```

The classic three-table many-to-many. Notes:

- **`bookmark_tags` has no `id` column.** The composite primary key `(bookmark_id, tag_id)` is both identity and uniqueness. There's never a reason to have two rows with the same `(bookmark_id, tag_id)` pair.
- **`tags.name` is `UNIQUE`.** Application logic ensures we lowercase-normalize before insert, but the DB enforces no duplicates regardless.
- **`ON DELETE CASCADE` on both FKs.** Delete a bookmark → its junction rows disappear. Delete a tag → same. (We never expose a "delete tag" endpoint right now, but if we did, the schema would handle the cleanup.)
- **`idx_bookmark_tags_tag`** — the secondary index supports the tag-filter query, which joins through `bookmark_tags` on `tag_id`. Without it, every tag filter would be a full table scan.

### A.3 — `bookmarks.rs` — the four-way filter SQL

The list endpoint accepts optional `q` (text) and `tag` (name) filters. We branch on which are present because **sqlx's `query!` macro can't introspect dynamic SQL** — the macro needs to see the literal SQL string at compile time so it can verify the types.

```rust
let rows = match (q_text, tag) {
    (None, None) => { /* base query */ }
    (Some(text), None) => { /* WHERE title LIKE … OR description LIKE … OR url LIKE … */ }
    (None, Some(tag_name)) => { /* JOIN bookmark_tags + tags WHERE name = ? */ }
    (Some(text), Some(tag_name)) => { /* both */ }
};
```

Each branch contains a different SQL string. Code duplication, yes — but the alternative (string-concat dynamic SQL) loses the compile-time type checking, which is the whole reason we picked sqlx.

In project 11 (contacts + dashboard) we'll switch to runtime-built queries with `QueryBuilder` for the cases where dynamic shape is unavoidable. For 4 fixed branches, the match is fine.

#### The base query — `LEFT JOIN` + `GROUP_CONCAT`

```sql
SELECT
    b.id, b.url, b.title, b.description, b.created_at, b.updated_at,
    COALESCE(GROUP_CONCAT(t.name, ','), '') AS "tags!: String"
FROM bookmarks b
LEFT JOIN bookmark_tags bt ON bt.bookmark_id = b.id
LEFT JOIN tags t ON t.id = bt.tag_id
GROUP BY b.id
ORDER BY b.updated_at DESC
```

What's happening:

- **`LEFT JOIN` (not `JOIN`).** A bookmark with zero tags still appears in the results — the right side of the join is NULL, GROUP_CONCAT yields NULL, `COALESCE` turns it into the empty string.
- **`GROUP_CONCAT(t.name, ',')`** — SQLite's aggregate function that concatenates the column with a separator. For a bookmark with tags `["rust", "axum"]` it produces `"axum,rust"` (insertion order, not necessarily what we want — we sort + dedup on the client side via `split_tags`).
- **`GROUP BY b.id`** — collapses the rows from the JOIN back to one row per bookmark.
- **`"tags!: String"`** — the `!:` tells sqlx "force-decode as non-null `String`". The `COALESCE` guarantees non-null at runtime; this annotation makes the type system reflect that.

#### The tag-filter query — double JOIN

```sql
SELECT b.id, ..., COALESCE(GROUP_CONCAT(t.name, ','), '') AS "tags!: String"
FROM bookmarks b
JOIN bookmark_tags bt0 ON bt0.bookmark_id = b.id
JOIN tags t0 ON t0.id = bt0.tag_id AND t0.name = ?1
LEFT JOIN bookmark_tags bt ON bt.bookmark_id = b.id
LEFT JOIN tags t ON t.id = bt.tag_id
GROUP BY b.id
ORDER BY b.updated_at DESC
```

Why **two** sets of `bookmark_tags` + `tags` joins?

- The `bt0` + `t0` join is the **filter**: inner-join on the requested tag name. Only bookmarks tagged with the requested tag survive.
- The `bt` + `t` left-join still collects **all** of each surviving bookmark's tags into the `GROUP_CONCAT`. Otherwise, filtering by tag `rust` would return bookmarks but with only `rust` in their `tags` field — losing the other tags.

This is the canonical pattern: **filter via an inner join, project via a left join with a separate alias.** Memorize it.

#### The transactional `create`

```rust
let mut tx = pool.begin().await?;
sqlx::query!("INSERT INTO bookmarks ...").execute(&mut *tx).await?;
set_bookmark_tags(&mut tx, &id, &tag_names).await?;
tx.commit().await?;
```

The bookmark insert + the (possibly several) tag inserts + the (possibly several) junction inserts all happen in **one transaction**. If anything fails partway through, `tx` is dropped without a commit and SQLite rolls back. **Never** spread related writes across multiple connections / non-transactional queries — partial state is worse than no state.

#### `set_bookmark_tags` — delete-then-insert

```rust
async fn set_bookmark_tags(tx, bookmark_id, tag_names) -> ... {
    sqlx::query!("DELETE FROM bookmark_tags WHERE bookmark_id = ?1", bookmark_id)
        .execute(&mut **tx).await?;
    for name in tag_names {
        let tag_id = upsert_tag(tx, name).await?;
        sqlx::query!("INSERT OR IGNORE INTO bookmark_tags (bookmark_id, tag_id) VALUES (?1, ?2)", ...)
            .execute(&mut **tx).await?;
    }
    Ok(())
}
```

Two ideas:

- **Delete-then-insert is the simplest "set the tags to exactly this list" pattern.** Alternatives (diff old vs new, batch insert missing, batch delete removed) are faster for very large tag lists; for 0–20 tags per bookmark, the simple version is correct and fast enough.
- **`INSERT OR IGNORE`** silently skips rows that would violate the primary-key uniqueness. Since we just deleted everything for this bookmark and then iterate over a deduplicated list, there's no actual collision — but the `OR IGNORE` is cheap insurance against future bugs (e.g., if we ever pass a non-deduped list).

#### `upsert_tag` — read or create

```rust
async fn upsert_tag(tx, name) -> AppResult<String> {
    if let Some(row) = sqlx::query!("SELECT id FROM tags WHERE name = ?1", name)
        .fetch_optional(&mut **tx).await?
    {
        return Ok(row.id.expect("id is non-null primary key"));
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query!("INSERT INTO tags (id, name) VALUES (?1, ?2)", id, name)
        .execute(&mut **tx).await?;
    Ok(id)
}
```

SELECT first, INSERT if missing. Inside a transaction, this has a TOCTOU race against other concurrent transactions — two simultaneous creates could both see "tag doesn't exist", both try to INSERT, and one gets a UNIQUE-violation error.

For a single-user MVP, fine. For multi-user, you'd use `INSERT ... ON CONFLICT (name) DO UPDATE SET name = excluded.name RETURNING id` — atomic upsert-and-fetch in one statement. Project 23 (multi-tenant help desk) revisits this with the proper pattern.

### A.4 — Validation: URL, title, description, tags

```rust
fn normalize_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() { return Err(...); }
    if trimmed.chars().count() > 2000 { return Err(...); }
    let parsed = url::Url::parse(trimmed)
        .map_err(|_| AppError::Validation("url must be a valid absolute URL".into()))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::Validation("url must be http or https".into()));
    }
    Ok(trimmed.to_string())
}
```

Four checks: non-empty, length cap, parses as a URL, scheme is http/https. The scheme check is **security-relevant**: a `javascript:` URL stored as a "bookmark" and later clicked through our UI would execute attacker JS in our origin. We reject before storage.

`normalize_tags` lowercases, dedups, restricts to `[a-z0-9-_]`, caps at 20 tags × 40 chars each. Tags become URL path components and CSS class fragments downstream; restrictive normalization avoids escape headaches.

### A.5 — `tags.rs` — count-per-tag query

```sql
SELECT t.name, CAST(COUNT(bt.bookmark_id) AS INTEGER) AS "count!: i64"
FROM tags t
LEFT JOIN bookmark_tags bt ON bt.tag_id = t.id
GROUP BY t.id
ORDER BY COUNT(bt.bookmark_id) DESC, t.name ASC
```

Three things to notice:

- **`LEFT JOIN`** so tags with zero bookmarks still appear (count = 0).
- **`CAST(COUNT(...) AS INTEGER) AS "count!: i64"`** — without the cast, sqlx might infer a different integer width depending on the SQLite affinity. Be explicit.
- **`ORDER BY COUNT(bt.bookmark_id) DESC`** — must repeat the expression, not just `ORDER BY count`. SQLite's prepare step doesn't resolve column aliases in `ORDER BY` when the alias is the result of an aggregate. (This was a real debugging stop in this project — see commit history.)

---

## B. Frontend

### B.1 — `package.json` + configs

Identical to project 04. No new deps.

### B.2 — `src/lib/debounce.ts` + `debounce.test.ts`

```ts
export function debounce<Args extends unknown[]>(
  fn: (...args: Args) => void,
  wait: number
): { (...args: Args): void; cancel(): void } {
  let timer: ReturnType<typeof setTimeout> | undefined;
  function debounced(...args: Args) {
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => { timer = undefined; fn(...args); }, wait);
  }
  debounced.cancel = () => {
    if (timer !== undefined) { clearTimeout(timer); timer = undefined; }
  };
  return debounced;
}
```

The `<Args extends unknown[]>` generic preserves the wrapped function's parameter types. Calling `debounce((s: string, n: number) => ..., 200)` returns a function with signature `(s: string, n: number) => void`. Old JS-style code would just type this as `(...args: any[]) => void` and lose all the safety.

`cancel()` is essential when the component unmounts or when the user clears the input — we don't want a stale call firing 200ms after they navigated away.

#### Testing time-based code

```ts
beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

it('coalesces rapid calls — only the last args are used', () => {
  const fn = vi.fn();
  const d = debounce(fn, 100);
  d('a'); vi.advanceTimersByTime(50);
  d('b'); vi.advanceTimersByTime(50);
  d('c'); vi.advanceTimersByTime(100);
  expect(fn).toHaveBeenCalledTimes(1);
  expect(fn).toHaveBeenCalledWith('c');
});
```

`vi.useFakeTimers()` swaps `setTimeout` for a fake that doesn't actually wait — we manually advance time with `vi.advanceTimersByTime(ms)`. The test runs instantly, not in 200ms. This is the only sane way to test debounce/throttle/intervals.

### B.3 — `src/lib/actions.ts` — `clickOutside`

```ts
import type { Action } from 'svelte/action';

export const clickOutside: Action<HTMLElement, () => void> = (node, callback) => {
  let cb = callback;
  function handle(event: Event) {
    const target = event.target as Node | null;
    if (target && !node.contains(target)) cb();
  }
  document.addEventListener('pointerdown', handle, true);
  document.addEventListener('focusin', handle, true);
  return {
    update(next: () => void) { cb = next; },
    destroy() {
      document.removeEventListener('pointerdown', handle, true);
      document.removeEventListener('focusin', handle, true);
    }
  };
};
```

Three pieces of the Action contract:

- **Initialization** — the function body runs once when the directive is applied. We attach two listeners (`pointerdown` for clicks, `focusin` for tab-into).
- **`update(next)`** — called when the callback prop changes (e.g., the user passes a new closure). We swap in the new one. Without this, the action would call the stale closure from the first render.
- **`destroy()`** — called when the directive is removed (component unmount or `{#if}` removes the node). We MUST clean up listeners here, or we leak handlers across mounts.

The `true` third arg to `addEventListener` is the **capture phase**. Listening in capture means we get the event before any descendant. Important: if a child stops propagation, we still see it.

**Why `pointerdown` and not `click`?** Click fires after the mouse-up, which is too late — a button inside a popup might `stopPropagation` on click before our listener runs. `pointerdown` fires earlier and is harder to swallow. For keyboard, `focusin` covers it.

**Future direction**: Svelte 5 introduces `{@attach}`-style attachments which replace `use:` actions. Project 19 (kanban) is where we switch to attachments as the main lesson. `use:` actions still work in Svelte 5 and are perfect for this project.

### B.4 — `src/routes/+page.svelte` — URL as state

#### The two-way binding

```ts
let searchInput = $state('');
$effect(() => {
  searchInput = data.q;  // sync from URL → input on navigation (back/forward)
});
```

- **URL → input**: the `$effect` runs whenever `data.q` (which comes from `+page.server.ts`'s load, which reads `url.searchParams.get('q')`) changes. This handles browser back/forward and programmatic navigation.
- **Input → URL**: `onSearchInput` updates `searchInput` instantly, then calls `commitSearch` (debounced) which calls `applyFilters` → `goto`. `goto` updates the URL, which re-runs `load`, which updates `data.q`. The `$effect` then sets `searchInput = data.q`, but the value is the same, so no flicker.

There's a subtle race: if the user types "abc" → debounce fires with "ab" → goto updates URL → `data.q = "ab"` → effect would overwrite `searchInput` to "ab" even though the user has typed "c". In practice the debounce is short enough (200ms) and the network roundtrip is local enough that this rarely surfaces. A more robust solution would track an "expecting" generation counter and skip the effect-sync while typing.

#### `applyFilters` — the URL writer

```ts
function applyFilters(next: { q?: string; tag?: string }) {
  const url = new URL(page.url);
  const q = (next.q ?? activeQ()).trim();
  const tag = (next.tag ?? activeTag).trim();
  if (q) url.searchParams.set('q', q); else url.searchParams.delete('q');
  if (tag) url.searchParams.set('tag', tag); else url.searchParams.delete('tag');
  void goto(`${url.pathname}${url.search}`, { keepFocus: true, noScroll: true, replaceState: false });
}
```

Three options to `goto` worth knowing:

- **`keepFocus: true`** — don't move focus to the page body. Without this, every keystroke in the search box would re-focus the body on URL change, taking focus out of the input. The user couldn't type.
- **`noScroll: true`** — don't scroll to the top. Without this, filtering would jump the user out of their current scroll position. Annoying.
- **`replaceState: false`** — adds a new history entry. Want this for tag clicks (so back goes back through filter changes). For search keystrokes, you might prefer `replaceState: true` (so the entire typing session is one history entry). Reasonable people disagree; we picked the simpler choice.

#### The "live region" announcement

```svelte
<p class="count" aria-live="polite">
  {bookmarks.length} {bookmarks.length === 1 ? 'result' : 'results'}
</p>
```

When the user filters by tag or types a search, the result count updates. `aria-live="polite"` makes screen readers announce the change. "5 results" → "1 result" tells them the filter worked.

### B.5 — The Add panel + `use:clickOutside`

```svelte
{#if addOpen}
  <section
    class="add-panel"
    aria-label="Add bookmark"
    use:clickOutside={() => (addOpen = false)}
  >
    <form ...>
```

Two pieces:

- **The `{#if addOpen}` guard** mounts and unmounts the panel based on state. When `addOpen` flips false, Svelte calls the action's `destroy` — cleaning up the document listeners. No leak.
- **`use:clickOutside={() => (addOpen = false)}`** — pass an inline closure. Each render creates a new closure, but the action's `update(next)` swaps it in. (If we forgot the `update` in the action, the first closure would forever close the panel — fine, but also stale-capturing other state if it ever changed.)

There's a subtle bug-prevention trick: the click that *opens* the panel happens on the "Add bookmark" button outside the panel. When that click fires, the panel doesn't exist yet, so `clickOutside` isn't attached. By the time the panel mounts, the click event has finished propagating. So the opening click doesn't trigger an immediate close.

If you ever build a pattern where the toggle button is INSIDE the popover (a self-contained menu), you'd need a different approach (toggle-on-button-click + close-on-outside-click + start the outside listener on a microtask delay).

### B.6 — `BookmarkCard.svelte` — `data-sveltekit-preload-data="off"`

```svelte
<a href={bookmark.url} target="_blank" rel="noopener noreferrer" data-sveltekit-preload-data="off">
```

SvelteKit's layout enables hover-preload globally (`data-sveltekit-preload-data="hover"` on `<body>`). For internal app links, that's great — the next page loads instantly. For external bookmarks, preloading would mean **fetching every URL the user hovers**, blasting through CORS errors and wasting their bandwidth.

`data-sveltekit-preload-data="off"` on this `<a>` opts it out. The standard `target="_blank" rel="noopener noreferrer"` opens the link in a new tab and prevents the linked page from accessing `window.opener`.

---

## C. Tests

| Layer | Coverage |
| --- | --- |
| cargo (9) | URL validation (https/http/empty/scheme), tag normalization (dedup, lowercase, special chars, empties), GROUP_CONCAT split helper. |
| vitest debounce (3) | Single trailing call, coalescing rapid calls, cancel(). All use `vi.useFakeTimers`. |
| vitest api (5) | list, list with q+tag (URLSearchParams encoding), create POST body, remove 204, ApiCallError on non-2xx. |
| playwright (5 × 4 = 20) | a11y, full lifecycle (add → list → tags visible → delete), search debounce + URL update, tag click → URL update, clickOutside closes panel. |

---

## D. Closing — what you can do now

- Design and query a many-to-many SQL schema. You understand `GROUP_CONCAT`, the inner-join-for-filter + left-join-for-projection pattern, transactional multi-write inserts, and `INSERT OR IGNORE` for idempotency.
- Treat the URL as the canonical source of state for filters and search. You know how to debounce keystrokes into URL writes, sync URL changes back to local UI state, and use `goto` with `keepFocus`/`noScroll` to avoid the common UX traps.
- Write a typed Svelte action with `Action<HTMLElement, T>`. You understand the `init / update / destroy` lifecycle and the capture-phase listener pattern.
- Test debounce-style code with fake timers — `vi.useFakeTimers()` + `vi.advanceTimersByTime(ms)`.
- Opt links out of SvelteKit's hover-preload when they point off-site.

Open `projects/06-splitter/COMMANDS.md` next — Expense Splitter. Form actions with field-level validation (`valibot` on client, `validator` + `serde` server-side), money as `i64` cents (no floats — ever), property tests on the splitter math, `$state.snapshot` for serializing form payloads.
