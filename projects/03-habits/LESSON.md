# Project 03 — Lesson

> Read alongside the code. The new ideas in this project are: property-based testing with `proptest`, the SQL "islands and gaps" pattern with window functions, CSS Grid mastery for the calendar layout, keyboard navigation across a 7×7 grid, and `prefers-reduced-motion` honored in CSS.

Project 01 taught the spine. Project 02 added markdown safety, SEO, and a11y enforcement. This project adds **mathematical rigor**: the streak function is a pure function, property-tested with thousands of random inputs, and we use SQL window functions to do the same computation a second way.

If you skipped 01 or 02, go back. This assumes you understand `$state`, `$derived`, `$props`, `sqlx::query!`, `IntoResponse`, the design-token cascade, axe-core a11y testing, and `gotoHydrated`.

---

## A. Backend

### A.1 — `Cargo.toml`

Only one addition over project 02: `proptest` as a dev-dependency.

```toml
[dev-dependencies]
proptest = "1.6"
```

That's it. `proptest` is purely a test crate — it adds nothing to production builds.

### A.2 — `migrations/0001_init.sql`

```sql
CREATE TABLE IF NOT EXISTS habits (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS habit_completions (
    habit_id        TEXT NOT NULL,
    completion_date TEXT NOT NULL,
    PRIMARY KEY (habit_id, completion_date),
    FOREIGN KEY (habit_id) REFERENCES habits(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_completions_date ON habit_completions (completion_date);
```

Lessons:

- **The `habit_completions` table has no `id` column.** The composite primary key `(habit_id, completion_date)` is both the identity and the uniqueness constraint. There is no scenario where you'd insert "this habit was completed on this date" twice — the data model enforces it.
- **`ON DELETE CASCADE`** — deleting a habit removes its completions automatically. Combined with `db.rs`'s `.foreign_keys(true)` (SQLite requires the flag to be on explicitly), the DB does the cleanup for us. **The lesson**: when foreign keys exist, configure cascading behaviour in the schema, not the application. If you forget in code, you leak; if it's in the schema, the DB enforces.
- **`CREATE INDEX … ON habit_completions (completion_date)`** — supports any query that filters or sorts by date across all habits (a future "what did I do on 2026-05-24?" feature). It's cheap insurance.

### A.3 — `src/error.rs` and `src/db.rs`

Identical to project 02 (without the `Conflict` variant — we don't need it here). Reference back if needed.

### A.4 — `src/streaks.rs` — the pure function

This is the most important module in the project. It's a pure function (no side effects, no DB, no I/O) that computes streak info from a list of dates. Pure functions are easy to test — both with handcrafted examples and with random inputs (property-based testing).

```rust
use chrono::{Duration, NaiveDate};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StreakInfo {
    pub current: u32,
    pub longest: u32,
    pub total: u32,
    pub last_completion: Option<NaiveDate>,
}

pub fn compute(completions: &[NaiveDate], today: NaiveDate) -> StreakInfo {
    debug_assert!(
        completions.windows(2).all(|w| w[0] < w[1]),
        "completions must be sorted and unique"
    );

    if completions.is_empty() {
        return StreakInfo { current: 0, longest: 0, total: 0, last_completion: None };
    }

    let mut longest: u32 = 1;
    let mut run: u32 = 1;
    for w in completions.windows(2) {
        if w[1] - w[0] == Duration::days(1) {
            run += 1;
            if run > longest { longest = run; }
        } else {
            run = 1;
        }
    }

    let last = *completions.last().expect("non-empty checked above");
    let yesterday = today - Duration::days(1);
    let current = if last == today || last == yesterday { run } else { 0 };

    StreakInfo {
        current, longest,
        total: completions.len() as u32,
        last_completion: Some(last),
    }
}
```

#### The precondition and `debug_assert!`

> `completions` MUST be sorted ascending and contain no duplicates.

We document the precondition. We enforce it with `debug_assert!` — which runs in debug builds (tests) but compiles to nothing in release. The cost of checking in production is paid only for code paths that are likely to violate the invariant, and the caller here is our own SQL (`ORDER BY completion_date ASC`), so we trust it.

**This pattern is the principal-engineer way to write a hot pure function**: encode the precondition in the doc, check it in debug, trust it in release. Don't pay the runtime cost for invariants you've verified in CI.

#### The algorithm

Walk the dates in order. Track the current run length. Whenever consecutive dates differ by exactly one day, extend the run. Otherwise reset to 1. Track the max ever seen.

Then: if the last completion is today or yesterday, the current streak is the trailing run. Otherwise it's 0 — the streak is broken once two days have passed.

This is O(n) time, O(1) extra space. No SQL, no allocations.

### A.5 — Property-based tests

We have six handcrafted unit tests (empty, single-today, single-yesterday, single-two-days-ago, week-run, old-long-run-then-short). Those catch the cases we thought of.

Then seven `proptest` properties catch the cases we didn't think of:

```rust
proptest! {
    #[test]
    fn longest_never_exceeds_total(dates in arb_completions(), today in arb_date()) {
        let info = compute(&dates, today);
        prop_assert!(info.longest as usize <= info.total as usize);
    }

    #[test]
    fn longest_at_least_current(dates in arb_completions(), today in arb_date()) {
        let info = compute(&dates, today);
        prop_assert!(info.longest >= info.current);
    }

    #[test]
    fn current_zero_unless_recent(dates in arb_completions(), today in arb_date()) {
        let info = compute(&dates, today);
        if let Some(last) = info.last_completion {
            let yesterday = today - Duration::days(1);
            if last != today && last != yesterday {
                prop_assert_eq!(info.current, 0);
            }
        }
    }

    #[test]
    fn current_equals_trailing_run_when_recent(dates in arb_completions(), today in arb_date()) {
        let info = compute(&dates, today);
        if let Some(last) = info.last_completion {
            let yesterday = today - Duration::days(1);
            if last == today || last == yesterday {
                let mut run: u32 = 1;
                for w in dates.windows(2).rev() {
                    if w[1] - w[0] == Duration::days(1) { run += 1; } else { break; }
                }
                prop_assert_eq!(info.current, run);
            }
        }
    }
    // ... three more
}
```

#### What's an "arbitrary"?

The functions `arb_date()` and `arb_completions()` are **strategies** — proptest's term for "thing that generates random values of type T". Each property runs 1024 times by default with fresh inputs. If any case fails, proptest **shrinks** the input — repeatedly tries smaller variations until it finds the minimal failing case. That smallest counterexample is what gets printed.

```rust
fn arb_date() -> impl Strategy<Value = NaiveDate> {
    (2020i32..=2030, 1u32..=12, 1u32..=28)
        .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).expect("clamped to valid"))
}

fn arb_completions() -> impl Strategy<Value = Vec<NaiveDate>> {
    prop::collection::hash_set(0i64..=1000, 0..=200).prop_map(|set| {
        let base = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid");
        let mut v: Vec<NaiveDate> = set.into_iter().map(|n| base + Duration::days(n)).collect();
        v.sort();
        v
    })
}
```

`arb_date` clamps year (2020-2030), month (1-12), day (1-28) so we never construct invalid dates. `arb_completions` samples a hash-set of integer offsets (0-1000 days from 2024-01-01), maps to dates, then sorts. Using a set guarantees uniqueness; sorting establishes the precondition.

#### Why properties are different from unit tests

A unit test says: "for this specific input, the output is X". A property says: "for *every* possible input, the output satisfies P".

Examples in this module:

- **`longest_never_exceeds_total`** — the longest streak can't be longer than the total number of completions. A trivial invariant; if it breaks, your counting is wrong.
- **`longest_at_least_current`** — the all-time longest must be at least the current. If current > longest, your max-tracking is broken.
- **`current_zero_unless_recent`** — if the last completion is older than yesterday, current must be 0. Catches off-by-one bugs in the recency check.
- **`current_equals_trailing_run_when_recent`** — when recent, current must equal the actual trailing run computed by an independent walk. This is the strongest property and the one most likely to find a real bug.

> **Try this** after everything is green: in `compute()`, change `if last == today || last == yesterday` to just `if last == today`. Run `cargo test`. `current_equals_trailing_run_when_recent` fails with a minimal counterexample showing a streak ending yesterday that should still be current. Now you've felt how proptest catches the things you didn't think of.

### A.6 — `src/routes/habits.rs`

Five route handlers (`list`, `create`, `update`, `delete`, `toggle_completion`) plus the bonus `streak_windows` showcase. Two ideas worth dwelling on:

#### The color validation

```rust
const ALLOWED_COLORS: &[&str] = &[
    "#4F46E5", "#059669", "#D97706", "#DC2626",
    "#0284C7", "#7C3AED", "#DB2777", "#0F766E",
];

fn normalize_color(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().to_uppercase();
    if !ALLOWED_COLORS.contains(&trimmed.as_str()) {
        return Err(AppError::Validation(format!(
            "color must be one of: {}",
            ALLOWED_COLORS.join(", ")
        )));
    }
    Ok(trimmed)
}
```

We don't let the user pick "any color". Why?

- **Visual consistency.** Eight curated colors all look good against our backgrounds.
- **Accessibility.** All eight pass WCAG AA contrast against `--color-bg-elev`. An arbitrary user-picked color would not.
- **Security/abuse.** Without validation, a malicious or buggy client could send a color value containing CSS-injection payloads. The string ends up in `style="--habit-color: …"` on the frontend; if it weren't from a whitelist, you'd have to sanitize/escape, and the easy thing is to whitelist.

Same array of colors is exposed to the frontend (`ALLOWED_COLORS` in `lib/types.ts`). Both sides agree. The backend's whitelist is the source of truth; the frontend's is for UI ergonomics.

#### `toggle_completion` — idempotent semantically, mutating physically

```rust
async fn toggle_completion(...) -> AppResult<Json<ToggleResult>> {
    // 1. Verify the habit exists; 404 if not.
    // 2. SELECT to see if there's already a completion for (habit, date).
    // 3. DELETE if there is, INSERT if there isn't.
    // 4. Fetch fresh completions list, compute new streak, return.
}
```

"Toggle" means: the same payload twice returns the same end state. We could've made `POST` always insert and `DELETE` always remove. We went with toggle because the calendar UI has *one* gesture (click a cell) that should *do the right thing* whether the cell was already filled or not. The API mirrors the gesture.

The return value `{ completed: bool, streak: StreakInfo, completions: Vec<NaiveDate> }` tells the client *what just happened* (`completed`) and *what the new state is*. The client can update local UI immediately without a separate `GET /api/habits` round-trip.

### A.7 — `streak_windows` — SQL islands and gaps

This is the SQL star of the project. Same problem (find consecutive-date runs) solved a second way, in SQL.

```sql
WITH numbered AS (
    SELECT
        completion_date,
        ROW_NUMBER() OVER (ORDER BY completion_date) AS rn
    FROM habit_completions
    WHERE habit_id = ?1
),
grouped AS (
    SELECT
        completion_date,
        DATE(completion_date, '-' || (rn - 1) || ' days') AS streak_key
    FROM numbered
)
SELECT
    MIN(completion_date) AS "start!: String",
    MAX(completion_date) AS "end!: String",
    CAST(COUNT(*) AS INTEGER) AS "length!: i64"
FROM grouped
GROUP BY streak_key
ORDER BY MIN(completion_date) DESC
```

#### How it works

The trick: assign each completion a row number `rn` in date order. Then compute `streak_key = completion_date - (rn - 1) days`. Two dates in the same streak (consecutive days) will have the same `streak_key`, because each one is offset by exactly the same `rn - 1` days from the streak's starting day.

Example:
| date       | rn | streak_key (date - (rn-1) days) |
|------------|----|---------------------------------|
| 2026-05-20 | 1  | 2026-05-20 |
| 2026-05-21 | 2  | 2026-05-20 |
| 2026-05-22 | 3  | 2026-05-20 |
| 2026-05-25 | 4  | 2026-05-22 |  ← gap! new streak.
| 2026-05-26 | 5  | 2026-05-22 |

Group by `streak_key` and aggregate → MIN/MAX/COUNT gives you each streak's start, end, and length.

This is one of the most beautiful patterns in SQL. Memorize it.

#### Why `"start!: String"` in sqlx?

When sqlx's `query!` macro infers types, `MIN(completion_date)` returns `Option<String>` because aggregates can be null on empty inputs. The `"start!: String"` syntax tells sqlx: "I know this is non-null in practice (we already filtered `WHERE habit_id = ?1`, and if there are no rows, the outer SELECT has no rows either); decode it as `String`, not `Option<String>`". Saves a `.unwrap()` per row.

The same syntax handles `"length!: i64"` — without the cast, sqlx might infer a different integer width. Be explicit.

#### Why have both Rust and SQL implementations?

- The Rust streak computation runs on every `list`/`toggle` (returned in the API response).
- The SQL `streak_windows` is for a future timeline view ("here's a history of all your streaks, longest first").

The two implementations validate each other: if one diverges, a smoke test that calls both should disagree. (We don't have that smoke test yet — it would belong in an integration test using a real DB. Project 11, when we move to Postgres + `sqlx::test`, will introduce this pattern.)

### A.8 — `src/main.rs`

Identical to project 02's shape. Module list adds `mod streaks;`. Default port is 3002. Default `FRONTEND_ORIGIN` is `http://localhost:5175`.

---

## B. Frontend

### B.1 — `package.json`

Same dependencies as project 02. **No new deps.** This project's lessons are all in CSS, runes, and SQL — nothing new on the JS dep tree.

### B.2 — Configs

Same configs as project 02; ports bumped to 3002 (backend), 5175 (dev), 4175 (preview).

### B.3 — `src/lib`

#### `types.ts`

```ts
export const ALLOWED_COLORS = [
  '#4F46E5', '#059669', '#D97706', '#DC2626',
  '#0284C7', '#7C3AED', '#DB2777', '#0F766E',
] as const;
```

The `as const` makes this a readonly tuple of string literals, not a generic `string[]`. TypeScript can then verify exhaustive checks against the palette.

`StreakInfo`, `Habit`, `ToggleResult` mirror the backend serde structs. `last_completion` is `string | null` (JSON has no `Option`).

#### `api.ts`, `api.test.ts`

Same shape as the previous projects' API clients. The `toggle` endpoint is the only new method; the test file has six tests covering list/create/toggle/remove/error/url-encoding.

### B.4 — `CalendarGrid.svelte` — the load-bearing component

The 7×7 calendar is the heart of the UI. Read this component twice.

#### Building the cells

```ts
function buildCells(): { iso: string; label: string; isToday: boolean }[] {
  const out = [];
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const todayIso = isoDate(today);
  for (let i = 48; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(today.getDate() - i);
    const iso = isoDate(d);
    out.push({
      iso,
      label: d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }),
      isToday: iso === todayIso
    });
  }
  return out;
}
```

We build 49 cells (7 weeks × 7 weekdays). `i = 48` is 48 days ago (oldest); `i = 0` is today (newest). The array runs oldest-to-newest.

`isoDate()` uses `getFullYear/getMonth/getDate` (the local-time getters). Why not `toISOString()`? Because `toISOString` converts to UTC. If a user in Tokyo opens the app at 1 AM JST on May 25 (which is 4 PM May 24 UTC), `toISOString().slice(0,10)` gives `2026-05-24` — but we want `2026-05-25` (the user's local calendar day). The local-getter version respects the user's clock.

This is **the canonical timezone trap in JS apps**. Always use local getters when "what day is it" is the question; use `toISOString()` only when communicating with a server that wants UTC.

#### The CSS Grid

```css
.grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  grid-template-rows: repeat(7, 1fr);
  grid-auto-flow: column;
  gap: 4px;
  width: fit-content;
}
```

Three CSS properties combine here in a non-obvious way:

- `grid-template-columns: repeat(7, 1fr)` — 7 equal columns (= 7 weeks).
- `grid-template-rows: repeat(7, 1fr)` — 7 equal rows (= 7 weekdays).
- **`grid-auto-flow: column`** — when items don't have explicit `grid-row`/`grid-column`, they fill the grid **column-first**, not the default row-first.

So our 49-element array (oldest→newest) fills column 1 top-to-bottom (oldest week, Mon–Sun), then column 2, etc. The bottom-right cell is "today" (which is what we want — newest day in the most prominent position).

If we forgot `grid-auto-flow: column`, the items would fill row-by-row and our "weeks as columns" semantics would break. Tiny CSS rule, huge consequence.

`width: fit-content` makes the grid as wide as it needs to be, so the calendar doesn't stretch to fill a wide container.

The cell size `clamp(18px, 4vw, 26px)` scales smoothly between 390px and 1440px viewports. No JS, no media queries — pure CSS.

#### Keyboard navigation

The grid is **one composite widget** (not 49 individually tab-stopped buttons). WAI-ARIA's "roving tabindex" pattern:

- The "entry point" cell has `tabindex={0}` (we picked the today cell — last in the array).
- All other cells have `tabindex={-1}` (focusable programmatically, not via Tab).
- Inside the grid, arrow keys move focus.

```svelte
<button
  …
  tabindex={i === cells.length - 1 ? 0 : -1}
  onkeydown={(e) => handleKey(e, i)}
>
```

The keyboard handler:

```ts
function handleKey(e: KeyboardEvent, index: number) {
  let next = index;
  switch (e.key) {
    case 'ArrowRight':
      next = Math.min(index + ROWS, cells.length - 1);  // column ↦ column (+7)
      break;
    case 'ArrowLeft':
      next = Math.max(index - ROWS, 0);                  // column ↦ column (-7)
      break;
    case 'ArrowDown':
      next = Math.min(index + 1, cells.length - 1);      // row ↦ row (+1)
      break;
    case 'ArrowUp':
      next = Math.max(index - 1, 0);                     // row ↦ row (-1)
      break;
    case 'Home':
      next = 0;
      break;
    case 'End':
      next = cells.length - 1;
      break;
    default:
      return;
  }
  e.preventDefault();
  const btn = document.querySelector<HTMLButtonElement>(
    `[data-habit="${habitId}"][data-index="${next}"]`
  );
  btn?.focus();
}
```

Index arithmetic mirrors the grid layout: arrow up/down moves ±1 (within a column, i.e., next/previous weekday); arrow left/right moves ±7 (between columns, i.e., next/previous week). The `Math.min/max` clamps to grid bounds so wrapping doesn't happen (some grids wrap; we chose not to, to avoid confusion at the edges).

We use `data-habit` + `data-index` attributes for the targeting selector instead of bind:this on every cell — bind:this would allocate 49 refs per habit per render, and we only need one for navigation.

#### A note on ARIA

I started with `role="grid"` + `role="gridcell"`, but `aria-pressed` is not valid on `gridcell` (axe-core caught this) and the full grid pattern requires `role="row"` wrappers etc. — heavy for a 49-cell heatmap. The simpler approach: `role="group"` on the container (announces "Habit name — 7 weeks of completions" as a region), buttons with `aria-pressed` (standard toggle-button pattern, well-understood by screen readers).

The tradeoff: we lose some semantic precision (a sighted user with a keyboard would learn the arrow-key gesture, but a screen-reader user might tab through individual buttons more conventionally). Acceptable for a personal habit tracker; not acceptable for a calendar widget that needs to be fully WAI-ARIA-compliant. For the latter, see Project 14 (calendar & scheduler).

### B.5 — `+page.svelte` — the dashboard

The aggregate stats use `$derived`:

```ts
const totals = $derived({
  habits: habits.length,
  activeStreaks: habits.filter((h) => h.streak.current > 0).length,
  longest: habits.reduce((m, h) => Math.max(m, h.streak.longest), 0)
});
```

Whenever `habits` changes (after a toggle or delete via the form action), `totals` recomputes. We could have computed these in the backend, but they're cheap on the client and keep the API focused on the source-of-truth data.

The color-picker uses `<input type="radio">` with `bind:group={color}` — Svelte's idiomatic way to bind one value to a group of radios. The CSS hides the native radio with `opacity: 0` and styles the sibling `<span aria-hidden="true">` instead. The native `:focus-visible` on the input shows the outline on the swatch via `+ span[aria-hidden]` selector, so keyboard users still see focus.

The two hidden forms (`toggleForm`, `deleteForm`) are the same pattern as projects 01 and 02 — `bind:this` to the form, set hidden field values via `$state`, then `requestSubmit()` on the next microtask. Reuses the SvelteKit form-action plumbing without any custom fetch code.

### B.6 — Reduced motion in CSS

```css
.cell {
  transition: transform var(--duration-fast) var(--ease-out);
}

@media (prefers-reduced-motion: reduce) {
  .cell {
    transition: none;
  }
}
```

The cell transitions are nice-to-have; for users who've opted out of motion at the OS level, we honor it. Note that `shared/design-tokens.css` already sets `--duration-fast: 0ms` under `(prefers-reduced-motion: reduce)`, so the *value* is zero — but `transition: none` (instead of `transition: transform 0ms`) is the most explicit way to declare intent. Either approach works.

The Playwright test verifies this in `respects prefers-reduced-motion`:

```ts
const context = await browser.newContext({ reducedMotion: 'reduce' });
…
const transition = await cell.evaluate((el) => getComputedStyle(el).transitionDuration);
expect(transition).toBe('0s');
```

`browser.newContext({ reducedMotion: 'reduce' })` opens a fresh context with the system preference simulated. We read `getComputedStyle().transitionDuration` and assert it's `'0s'`. Real evidence that the a11y promise holds.

---

## C. Tests

Tested-by-shape mapping:

| Layer | What it proves |
| --- | --- |
| `cargo test` unit (12) | Streak math handles the obvious cases. Color validation accepts the whitelist and rejects everything else. Name validation respects trim + length. |
| `cargo test` proptest (7) | The streak function holds the invariants on ANY input. Catches bugs the unit tests didn't think of. |
| `vitest` (6) | The fetch client serializes the right body, encodes paths correctly, throws ApiCallError on non-2xx, returns undefined on 204. |
| `playwright` (5 × 4 = 20) | The whole stack: a11y violations are zero, full lifecycle works, validation gates the form, keyboard navigation moves focus per the grid layout, reduced-motion is honored. |

Total: **45+ runs**. Each layer catches different bug classes. Removing any layer leaves a class of bugs uncatchable in CI.

---

## D. Closing — what you can do now

By the end of this project, you can:

- Write pure functions and verify them with property-based tests. You can articulate why property tests catch bugs unit tests miss. You can write a `Strategy` for a custom input shape and shrink to a minimal failing case.
- Implement the SQL islands-and-gaps pattern with window functions. You can read it, write it, and explain why `(date - row_number) groups consecutive dates`.
- Build a keyboard-navigable composite widget with the roving-tabindex pattern. You know what `tabindex={0}` vs `-1` means, why arrow keys move within the widget instead of moving focus out of it, and where the entry cell should live.
- Honor `prefers-reduced-motion` in CSS and test it in Playwright.
- Use `$derived` for arbitrary computed UI state — no boilerplate "subscribe to changes" code.

Open `projects/04-pomodoro/COMMANDS.md` when you're ready. Pomodoro Timer, `$effect` with cleanup, `requestAnimationFrame`, audio API, and `$inspect` for debugging reactivity.
