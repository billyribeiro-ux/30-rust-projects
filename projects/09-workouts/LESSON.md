# Project 09 — Lesson

Headline lessons:

1. **SQLite FTS5** — virtual table fed by triggers, queried with `MATCH`, ordered by `rank`.
2. **Pure PR-detection function + proptest** — `detect_prs(sets) -> Vec<PrInfo>`, five properties.
3. **Complex `$derived` chains** — a dependency graph of cheap, lazy expressions powering the dashboard widgets.
4. **axe-core as a CI gate** — `enforces-zero-violations.spec.ts` is the named blocker.
5. **CSV export** with RFC-4180 escaping and the correct `text/csv; charset=utf-8` header.
6. **Playwright visual regression** for the `<PrBadge>` component.

Read this file with the source open. Every line of code you see below has a corresponding file.

---

## A. The data model

### A.1 — `migrations/0001_init.sql`

Three core tables, one FTS index, three triggers.

```sql
CREATE TABLE exercises (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    muscle_group  TEXT NOT NULL,
    created_at    TEXT NOT NULL
);
CREATE TABLE workouts (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL DEFAULT '',
    performed_at  TEXT NOT NULL,
    created_at    TEXT NOT NULL
);
CREATE TABLE sets (
    id            TEXT PRIMARY KEY,
    workout_id    TEXT NOT NULL,
    exercise_id   TEXT NOT NULL,
    weight_minor  INTEGER NOT NULL CHECK (weight_minor > 0),
    reps          INTEGER NOT NULL CHECK (reps > 0),
    rir           INTEGER NOT NULL DEFAULT 0 CHECK (rir BETWEEN 0 AND 10),
    set_order     INTEGER NOT NULL,
    FOREIGN KEY (workout_id) REFERENCES workouts(id) ON DELETE CASCADE,
    FOREIGN KEY (exercise_id) REFERENCES exercises(id) ON DELETE RESTRICT
);
```

Three discipline points:

**1. `CHECK (weight_minor > 0)` and `CHECK (reps > 0)`.** The PR detector treats volume `weight * reps` as the metric. If either is zero, the very-first-set-is-always-a-PR property breaks (since 0 is not > 0). The DB rejects bad rows before the pure function sees them — defense-in-depth.

**2. Cascade vs restrict.** Deleting a workout cascades to its sets (the workout is the parent of the sets — no reason to keep orphans). Deleting an exercise is *restricted*: if any set references it, the FK rejects the delete. The route maps that error to HTTP 409. The user has to delete the workouts first.

**3. `set_order`.** Sets within a workout have a stable display order independent of insertion timestamp. `(workout_id, set_order)` would ideally be a unique constraint; we don't add it because the backend always inserts a complete workout atomically and never re-orders.

### A.2 — FTS5 virtual table + triggers

```sql
CREATE VIRTUAL TABLE exercises_fts USING fts5(
    name,
    content='exercises',
    content_rowid='rowid'
);
CREATE TRIGGER exercises_ai AFTER INSERT ON exercises BEGIN
    INSERT INTO exercises_fts(rowid, name) VALUES (new.rowid, new.name);
END;
CREATE TRIGGER exercises_ad AFTER DELETE ON exercises BEGIN
    INSERT INTO exercises_fts(exercises_fts, rowid, name) VALUES('delete', old.rowid, old.name);
END;
CREATE TRIGGER exercises_au AFTER UPDATE ON exercises BEGIN
    INSERT INTO exercises_fts(exercises_fts, rowid, name) VALUES('delete', old.rowid, old.name);
    INSERT INTO exercises_fts(rowid, name) VALUES (new.rowid, new.name);
END;
```

Four points worth slowing down for:

**`content='exercises'`** declares external-content mode. The FTS table doesn't store its own copy of the row; it stores only the tokenized index, and looks the canonical row up by `rowid`. That keeps disk usage low and avoids drift between the table and the index, *provided* the triggers keep them in sync.

**`INSERT INTO exercises_fts(exercises_fts, rowid, name) VALUES('delete', ...)`** is the documented FTS5 command for "remove this rowid from the index without touching the underlying table." It looks like a regular insert, but the magic first column tells SQLite "treat this as a delete." Yes, it's a peculiar API.

**The update trigger is delete-then-insert** rather than `UPDATE`. FTS5 external-content tables don't support in-place updates; you must remove the old token set and insert the new one. The trigger does both atomically.

**Why triggers instead of just writing to both tables in app code?** A hand-written discipline that works for the first six months and then breaks the day someone writes a one-off SQL fix in the database CLI. Triggers move that discipline into the schema, where every writer — Rust, sqlite3-cli, Python migration — gets the same behavior for free.

### A.3 — Querying FTS5

`routes/exercises.rs::list` branches on whether the query is present:

```rust
let match_query = build_fts_query(text);
let r = sqlx::query!(
    r#"
    SELECT e.id, e.name, e.muscle_group, e.created_at
    FROM exercises_fts f
    JOIN exercises e ON e.rowid = f.rowid
    WHERE exercises_fts MATCH ?1
    ORDER BY rank
    LIMIT 50
    "#,
    match_query,
)
.fetch_all(&s.pool).await?;
```

Three things to note:

**Join on `rowid`.** External-content FTS gives back rowids, not full rows. We join back to the canonical `exercises` table on `e.rowid = f.rowid`.

**`ORDER BY rank`.** FTS5 exposes a built-in `rank` column that's the BM25-ish relevance score (lower is better). Sorting by `rank` puts the closest matches first — without it you get insertion order, which is useless for autocomplete.

**`build_fts_query`** sanitizes user input. We strip everything except alphanumerics and whitespace, then append `*` to each token. The result is a prefix-match query: `bench` matches `Bench Press` and `Bench Dip`, `pre` matches `Press`. Sanitization matters — without it, an `"` or a `)` from the user could break out of the FTS5 query syntax. We don't expose advanced FTS syntax to the user; they're typing into an autocomplete, not a search-query language.

Empty queries are turned into `"zzznoresultsxyz*"`. FTS5 errors on an empty MATCH; we'd rather return zero rows than 500.

---

## B. The pure PR module — `src/prs.rs`

```rust
pub fn detect_prs(sets: &[SetInput]) -> Vec<PrInfo> {
    let mut out = Vec::with_capacity(sets.len());
    let mut best: i64 = 0;
    for s in sets {
        let volume = s.weight_minor.saturating_mul(s.reps);
        let is_pr = volume > best;
        if is_pr {
            best = volume;
        }
        out.push(PrInfo { volume_minor: volume, is_pr });
    }
    out
}
```

Eight lines. That's the whole feature.

**Why pure?** Three reasons.
- *Testability.* No database, no HTTP, no clock. The proptest harness can churn through hundreds of cases per property in milliseconds.
- *Composability.* The same function powers the `read` handler (one workout's badges), the `prs` endpoint (history for one exercise), and the stats aggregator (count across all exercises). Three call sites, one implementation.
- *Refactor immunity.* If we add session-level PRs ("best ever single set", "best ever 5-rep set", etc.) the existing code stays exactly the same — we just add new pure functions.

**`saturating_mul`.** A real lifter's weight in grams is bounded — `1000 kg × 100 reps = 100_000 × 100 = 10_000_000`. That's nowhere near `i64::MAX`. But a property-tested function should never UB. `saturating_mul` clamps to `i64::MAX` on overflow rather than wrapping (which would falsely flag a tiny set as a PR after wrap). The cost is one CPU instruction.

**Strictly greater, not "greater or equal".** That's a product decision. Matching your previous max isn't *getting stronger*; you have to beat it. The unit test `equal_volume_does_not_count_as_pr` pins this.

### B.1 — The proptest properties

Five properties, each one paragraph in plain English:

```rust
proptest! {
    #[test]
    fn prs_are_monotone(sets in arb_sets()) {
        let out = detect_prs(&sets);
        let pr_volumes: Vec<i64> =
            out.iter().filter(|p| p.is_pr).map(|p| p.volume_minor).collect();
        for w in pr_volumes.windows(2) {
            prop_assert!(w[1] > w[0]);
        }
    }
    // first_set_is_always_pr
    // pr_count_leq_total
    // output_aligned_with_input
    // deterministic
}
```

`arb_sets()` is the strategy: vectors of up to 50 sets, each with `weight ∈ [1, 1_000_000]`, `reps ∈ [1, 30]`, timestamps sorted ascending. Bounds keep the property runs fast and ensure `weight * reps` stays comfortably inside `i64`.

**`prs_are_monotone`** is the headline property — the whole reason PR detection is interesting. The volumes flagged as PR must be strictly increasing. This catches a class of bugs the unit tests would miss: a regression that re-flagged the same volume as a PR twice would fail every single proptest case.

**`first_set_is_always_pr`** — every non-empty input has at least one PR. Together with `prs_are_monotone`, it gives a strong invariant: any non-empty workout produces a PR sequence that starts somewhere and only goes up.

**`pr_count_leq_total`** — trivial bound, useful as a smoke test that the output shape is sane.

**`output_aligned_with_input`** — `out.len() == sets.len()` and `out[i].volume_minor == sets[i].weight_minor * sets[i].reps`. Catches order-shuffling bugs.

**`deterministic`** — re-running `detect_prs` on the same input yields the same output. Should be obvious from the source, but proptest is cheap and the property catches mutable-state regressions.

### B.2 — Wiring the pure module into routes

Three call sites:

- `routes/exercises.rs::prs` — fetches all sets for one exercise, runs `detect_prs`, returns only the PRs.
- `routes/workouts.rs::read` — for each exercise referenced in the workout, fetches its full history *up to and including* this workout's `performed_at`, runs `detect_prs`, then marks just the sets that belong to *this* workout. Subtle: a set in workout W is a PR if it beat every prior set INCLUDING earlier sets in W itself. Sort order is `(performed_at ASC, set_order ASC)`, which respects within-workout ordering.
- `routes/stats.rs::read` — iterates all exercises, counts PRs across the whole history.

All three handlers parse rows, build `Vec<SetInput>`, call `detect_prs`, and consume the `Vec<PrInfo>`. No SQL in the PR logic; no PR logic in the SQL.

---

## C. The CSV export — `routes/export.rs`

```rust
pub async fn export_csv(State(s): State<AppState>) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query!(/* sets joined to workouts + exercises, ASC */).fetch_all(&s.pool).await?;

    let mut body = String::with_capacity(rows.len() * 80 + 80);
    body.push_str("performed_at,workout_name,exercise_name,muscle_group,weight_minor,reps,rir,set_order\n");
    for r in rows {
        let _ = writeln!(body, "{},{},{},{},{},{},{},{}",
            csv_escape(&r.performed_at), csv_escape(&r.workout_name),
            csv_escape(&r.exercise_name), csv_escape(&r.muscle_group),
            r.weight_minor, r.reps, r.rir, r.set_order);
    }

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv; charset=utf-8"));
    headers.insert(header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"workouts.csv\""));
    Ok((headers, body))
}
```

Three points:

**The content-type header is load-bearing.** Browsers display `text/plain` inline; `text/csv` triggers the "save as" dialog (combined with `content-disposition: attachment`). The `charset=utf-8` matters because exercise names can include accented characters (`"Hála press"`, `"über squat"`) — without the charset, Excel would mojibake them.

**RFC-4180 escaping.** A field gets wrapped in `"..."` if it contains a comma, quote, CR or LF; embedded quotes are doubled. Four unit tests pin those four cases. The user-controlled fields (workout name, exercise name) can contain any of those characters; the numeric fields can't.

**`String::with_capacity`.** A back-of-envelope pre-allocation saves the body builder from N reallocations as it grows. `rows.len() * 80 + 80` is a generous upper bound; if the actual size is smaller, allocator slack is wasted but the writes are still linear.

**Why not stream?** For one user with thousands of sets, the whole CSV is under 100 KB. Streaming would let us handle millions of rows, but we're a SQLite single-user app. Project 17 onwards introduces streaming bodies when it actually matters.

---

## D. Frontend — complex `$derived` chains

`src/routes/+page.svelte` is the dashboard. The script section:

```ts
let { data }: PageProps = $props();
const workouts = $derived(data.workouts);

const totalSets   = $derived(workouts.reduce((acc, w) => acc + w.set_count, 0));
const totalVolume = $derived(workouts.reduce((acc, w) => acc + w.total_volume_minor, 0));

const oneWeekAgo  = $derived(Date.now() - 7 * 24 * 60 * 60 * 1000);
const last7Workouts = $derived(
  workouts.filter((w) => new Date(w.performed_at).getTime() >= oneWeekAgo)
);
const volumeThisWeek = $derived(
  last7Workouts.reduce((acc, w) => acc + w.total_volume_minor, 0)
);
const averageSetsPerWorkout = $derived(
  workouts.length === 0 ? 0 : Math.round(totalSets / workouts.length)
);
const recentWorkouts = $derived(workouts.slice(0, 5));
```

Read top-to-bottom and you see a DAG:

```
data.workouts → workouts ─┬→ totalSets ──→ averageSetsPerWorkout
                          ├→ totalVolume
                          ├→ last7Workouts → volumeThisWeek
                          └→ recentWorkouts
```

Touching `data.workouts` (e.g., after a SvelteKit `invalidate` or a form submit) invalidates `workouts` synchronously. Anything depending on `workouts` is marked stale. When the template next renders the expression — say `{totalSets}` — Svelte traverses the chain, computes the leaves, and caches the values. Nothing recomputes until it's read; nothing reads until it's drawn.

Compare to React's `useMemo` ladder, where you have to manually list dependencies. Svelte tracks reads through `$derived` automatically — adding a new dependency is just using the variable inside the expression.

The `$derived` runes are *lazy*. If the user is on mobile and `averageSetsPerWorkout` is below the fold, it won't compute until scroll. The cost of declaring 6 derived chains where the page might only need 2 is essentially zero.

### D.1 — The autocomplete debounce — when `$effect` is the right tool

`routes/workouts/new/+page.svelte` runs an FTS5 search per keystroke, debounced:

```ts
let query = $state('');
let suggestions = $state<Exercise[]>([]);
let searchTimer: ReturnType<typeof setTimeout> | null = null;

$effect(() => {
  const q = query;                    // capture for cleanup
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(async () => {
    if (!q.trim()) {
      suggestions = data.exercises.slice(0, 8);
      return;
    }
    try {
      suggestions = await exercisesApi.list(fetch, q);
    } catch {
      suggestions = [];
    }
  }, 150);
  return () => { if (searchTimer) clearTimeout(searchTimer); };
});
```

This is the kind of effect svelte-autofixer suggests rewriting with `$derived`. Three reasons we don't:

- **It's async.** `$derived` is purely synchronous. You can't put `await` in it.
- **It has side effects.** `setTimeout` schedules work. `$derived` should be a pure mapping from inputs to outputs.
- **It has cleanup.** Cancelling the pending timer on each keystroke is essential for debounce; `$derived` has no teardown lifecycle.

The intentional pattern is documented in this file because the autofixer flagged it as a malpractice suggestion — it's the right tool for async + cleanup + side effect, all three together.

### D.2 — Weight as i64 minor units

`src/lib/weight.ts` is small but principled.

```ts
export function formatWeight(minor: number): string {
  if (!Number.isFinite(minor)) return '—';
  return `${(minor / 1000).toFixed(1)} kg`;
}
export function parseWeightKg(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const n = Number(trimmed);
  if (!Number.isFinite(n) || n <= 0) return null;
  return Math.round(n * 1000);
}
```

Same lesson as the money formatter in project 06 (`splitter`) — the on-the-wire format is **integer minor units**. Grams here, cents there. `80.5 kg` becomes `80_500`. `Math.round(n * 1000)` is the only place a float ever touches the value; from there on it's `i64` through Rust and `number` (which is a 53-bit safe integer for our range) through TypeScript.

`formatVolume` switches to tonnes·rep above 1000 kg·rep — once an exercise's history reaches the megagram-rep range, kg is the wrong unit.

---

## E. The PrBadge component & visual regression

`src/lib/components/PrBadge.svelte` is **deterministic by design**:

```svelte
<span class="pr-badge" aria-label="Personal record" role="img">
  <span class="dot" aria-hidden="true"></span>
  <span class="text">PR</span>
</span>
```

```css
.pr-badge {
  /* fixed colors, fixed dimensions, no animations */
  height: 18px;
  font-size: 11px;
  font-weight: 700;
  background: hsl(38 92% 45%);
  ...
}
```

No `prefers-color-scheme`, no `prefers-reduced-motion`, no time-of-day theme, no random pulse. The badge looks exactly the same on every render. That makes it suitable as the anchor for `e2e/visual-pr-badge.spec.ts`:

```ts
await expect(badge).toHaveScreenshot('pr-badge.png', { maxDiffPixelRatio: 0.02 });
```

`maxDiffPixelRatio: 0.02` tolerates 2% pixel drift from font subpixel rendering across OS/Chromium versions. The snapshot lives next to the spec file in `e2e/visual-pr-badge.spec.ts-snapshots/` and *is committed to git* — that's the whole point. If a future PR changes the badge accidentally, Playwright fails.

How do we get the badge onto the page deterministically? The spec creates an exercise with a unique name (per-run timestamp + viewport-name suffix to avoid races between parallel runs) and logs one set against it. The first-set-is-always-a-PR invariant guarantees the badge renders.

---

## F. axe-core as a CI gate — not a hint

`e2e/enforces-zero-violations.spec.ts` runs `@axe-core/playwright` on three routes:

```ts
test(`axe-core: ${route} has zero WCAG 2 A/AA violations`, async ({ page }) => {
  await gotoHydrated(page, route);
  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations,
         JSON.stringify(results.violations, null, 2)).toEqual([]);
});
```

The filename is intentional. When a teammate sees `enforces-zero-violations.spec.ts: FAILED` in CI logs they know — without reading the code — that this is the accessibility gate, not a flaky integration test. The failure message JSON-stringifies every violation so the fix isn't a treasure hunt.

Two real bugs this gate caught while writing this project:
- **`<dl><div>...</div></dl>`** — axe flagged the `<div>` wrappers around `<dt>/<dd>` as breaking the description-list semantics. WCAG 1.3.1 (Info and Relationships). The fix was a CSS Grid that lays out direct `dt`/`dd` children into a 4-column or 8-column layout depending on viewport.
- **`autofocus` attribute** — flagged by svelte-check (not axe), but the issue lives in the same axis: a static autofocus on initial mount steals focus from screen readers reading the page header. We replaced it with a Svelte action that fires only when the picker modal opens.

If you make `enforces-zero-violations` a warning-only check, you will lose accessibility coverage within three sprints. Make it a blocker.

---

## G. Quality gates — what we ran

Run these from the project root, in order:

```bash
# Backend
cd backend
export DATABASE_URL="sqlite://$(pwd)/workouts.db"
sqlite3 workouts.db < migrations/0001_init.sql   # only if not present
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                                        # 20/20

# Frontend
cd ../frontend
pnpm install
pnpm check                                        # 0 ERRORS 0 WARNINGS
pnpm test:unit                                    # 16/16

# Playwright (needs backend running)
cd ../backend && cargo run --release &           # :3008
cd ../frontend
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers pnpm exec playwright install chromium
pnpm test:e2e --update-snapshots                  # first time only
pnpm test:e2e                                     # 40/40 stable
```

### G.1 — svelte-autofixer suggestions accepted

The MCP `svelte-autofixer` returned ZERO `issues` for every component. It returned two `suggestions`:

- **`bind:this` on the hidden delete-form** in `exercises/+page.svelte` and `+page.svelte` could be an action or attachment. Kept as-is for consistency with projects 06–08, which all use the same pattern. The action/attachment rewrite is a different pattern lesson that belongs in a later project.
- **`$effect` containing `setTimeout` + `clearTimeout`** in `workouts/new/+page.svelte`. Documented above (§D.1) — async with cleanup is exactly the case `$effect` is for; `$derived` doesn't work here.

Both intentional; both documented; both compiling under `0 ERRORS 0 WARNINGS`.

---

## H. What you can build alone after this project

- A SQLite-backed app with full-text search over user-typed names.
- A pure, property-tested algorithm that drives multiple UI features.
- A multi-page SvelteKit app with derived dashboard widgets.
- A CSV export endpoint with correct content-type and escaping.
- A Playwright suite that uses both axe-core and visual regression.
- The discipline to treat accessibility as a gate and PR detection as math, not branches of HTTP handler code.

Next: project 10. Then file uploads, auth, multi-user — the world gets harder fast.
