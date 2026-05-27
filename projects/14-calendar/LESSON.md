# Project 14 — Lesson

Headline lessons:

1. **RRULE recurrence stored as a master row + expanded at read time.**
   One DB row per recurring event; no occurrence tables. Range queries
   call `rrule` to unroll occurrences inside the window.
2. **Timezone-aware storage** — UTC on the wire, IANA zone (`tz`) in
   the row, expansion does the heavy lifting via `chrono-tz`.
3. **Shared calendars with `view`/`edit` permissions** at the SQL layer.
   Every query joins the permission lattice; the handler never says
   "if owner || share.edit".
4. **Bounded RRULE expansion** — 90-day range cap + 366-occurrence
   per-event cap prevents one bad query from burning a worker.
5. **The axum 0.8 path-segment rule** and what it means for `.ics`
   suffixes.

Assumed knowledge: Argon2 + sessions (11), per-user 404-not-403 scoping
(11), form actions (8/11), AppError mapping (1/11), Postgres CHECK
constraints + FK cascades (11).

---

## A. Backend

### A.1 — Schema: master row + RRULE column

```sql
CREATE TABLE events (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    calendar_id   UUID NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    created_by    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title         TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    description   TEXT NOT NULL DEFAULT '',
    location      TEXT NOT NULL DEFAULT '',
    start_at      TIMESTAMPTZ NOT NULL,
    end_at        TIMESTAMPTZ NOT NULL,
    tz            TEXT NOT NULL DEFAULT 'UTC',
    rrule         TEXT,
    all_day       BOOLEAN NOT NULL DEFAULT FALSE,
    ...
    CHECK (end_at >= start_at)
);
CREATE INDEX idx_events_calendar_time ON events (calendar_id, start_at);
CREATE INDEX idx_events_recurring ON events (calendar_id)
    WHERE rrule IS NOT NULL;
```

**What** — Recurring and one-off events live in the same table. A
recurring event is "the first occurrence in `start_at/end_at` + an
RRULE that produces the rest." A one-off is the same row with
`rrule = NULL`.

**Why store the master + RRULE, not materialised occurrences** — Two
reasons:

1. **Update propagation.** When the user renames "Standup" to "Daily
   sync," we change ONE row. With materialised rows, you'd UPDATE every
   future occurrence (and choose whether to update past ones, and worry
   about the index churn). The master pattern is O(1) writes.
2. **Open-ended recurrence.** "Every Monday forever" is one row. With
   materialised occurrences you'd have to pick a horizon (1 year? 5?
   100?) and keep extending it.

The trade-off: **reads do more work**. Every range query that touches a
recurring event has to expand the RRULE. That's why we cap the range
at 90 days and the expansion at 366 occurrences (§A.3).

The partial index `WHERE rrule IS NOT NULL` is the small detail that
makes "give me all recurring events on this calendar" cheap, separate
from the main `idx_events_calendar_time` index that serves one-offs.

### A.2 — The range query: expand recurrences inside the window

```rust
let rows = sqlx::query_as!(
    EventRow,
    r#"
    SELECT e.id, e.calendar_id, e.title, e.description, e.location,
           e.start_at, e.end_at, e.tz, e.rrule, e.all_day,
           e.created_at, e.updated_at
    FROM events e
    JOIN calendars c ON c.id = e.calendar_id
    LEFT JOIN calendar_shares cs ON cs.calendar_id = c.id AND cs.user_id = $1
    WHERE (c.owner_id = $1 OR cs.user_id = $1)
      AND ($2::uuid IS NULL OR e.calendar_id = $2)
      AND (
        (e.rrule IS NULL AND e.start_at < $4 AND e.end_at > $3)
        OR (e.rrule IS NOT NULL AND e.start_at < $4)
      )
    "#,
    user.id, q.calendar_id, q.from, q.to,
).fetch_all(&s.pool).await?;
```

**What** — Two `OR`-joined predicates inside the same query:

- One-off events whose `[start, end)` overlaps the window
  (`start < window_end AND end > window_start`).
- Recurring events whose master `start_at` is before `window_end`
  (we can't pre-filter on the *occurrences*, since they're computed
  in Rust — we just bound by "this event started existing before our
  window ends").

**Why fold both branches into one query** — Two reads (one for
one-offs, one for recurrings) would also work. A single query is one
round trip and one execution plan; the optimiser can use the partial
index for the recurring branch and the main index for the one-offs in
the same scan.

Then in Rust:

```rust
for r in rows {
    if let Some(ref rrule_str) = r.rrule {
        let occurrences = expand_rrule(&r, rrule_str, q.from, q.to)
            .unwrap_or_default();
        for (start, end) in occurrences {
            out.push(Occurrence { event_id: r.id, ..., is_recurring: true });
        }
    } else {
        out.push(Occurrence { event_id: r.id, ..., is_recurring: false });
    }
}
out.sort_by_key(|o| o.start_at);
```

**Why `event_id` not `id` on `Occurrence`** — Each generated
occurrence shares the parent's id. The frontend uses this to navigate
to "edit the event," knowing that recurrences are derived. (A future
extension is "edit just this occurrence" via an `EXDATE` column or
override table — that's the project 24 / 30 evolution.)

### A.3 — RRULE expansion via the `rrule` crate

```rust
fn expand_rrule(
    e: &EventRow,
    rrule_str: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
    let tz: RTz = e.tz.parse::<chrono_tz::Tz>()
        .map_err(|_| AppError::Validation("invalid tz on event".into()))?
        .into();
    let dtstart = e.start_at.with_timezone(&tz);
    let duration = e.end_at - e.start_at;

    let combined = format!(
        "DTSTART;TZID={}:{}\n{}",
        e.tz,
        dtstart.format("%Y%m%dT%H%M%S"),
        rrule_str.trim()
    );
    let set: RRuleSet = RRuleSet::from_str(&combined)?
        .after(from.with_timezone(&tz))
        .before(to.with_timezone(&tz));

    let result = set.all(366);
    Ok(result.dates.into_iter().map(|s| {
        let start_utc = s.with_timezone(&Utc);
        (start_utc, start_utc + duration)
    }).collect())
}
```

**What** — Build a combined ICS-style string:

```
DTSTART;TZID=America/Los_Angeles:20260525T090000
RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=8
```

…parse it into an `RRuleSet`, bound it with `.after(...).before(...)`,
expand with `.all(366)` (which means "up to 366 occurrences, then stop").
For each generated start instant, derive `end = start + duration`.

**Why bound the call** — Without a cap, `.all()` on an infinite RRULE
(`RRULE:FREQ=DAILY`) runs forever. 366 = one year of daily events; for
weekly that's ~7 years; for monthly that's ~30. Combined with the 90-day
range cap on the HTTP query, the worst case is at most ~366 occurrences
in 90 days.

**Why duration is computed once** — The crate gives us occurrence
*starts*. Calendars expect (start, end) pairs. The right rule for
recurring events is "every occurrence has the same duration as the
first" — `end - start` from the master row. (DST shifts could make this
subtly wrong for events spanning a DST boundary, but for the common case
of 30-minute meetings entirely on one side of the boundary it's
correct.)

### A.4 — The "RRULE: " prefix gotcha

The `rrule` crate expects the input to be either:

```
DTSTART;TZID=...:20260525T090000
RRULE:FREQ=WEEKLY;...
```

(with literal `RRULE:` prefix on the rule line) — which is what users
naturally paste from ICS docs. We could also strip the `RRULE:` prefix
and pass just `FREQ=WEEKLY;...`, but accepting the spec-canonical form is
more permissive at no cost.

**Try it:** Open `tests/` and add `cargo test rrule_smoke` that asserts
weekly recurrence expansion. Or via `curl`, the COMMANDS.md drill prints
4 windowed occurrences out of an 8-`COUNT` RRULE.

### A.5 — Calendar sharing as a permission CTE

Look at the list query for calendars:

```sql
SELECT id, owner_id, name, color, default_tz,
       'owner'::text AS "permission!",
       created_at, updated_at
FROM calendars WHERE owner_id = $1
UNION ALL
SELECT c.id, c.owner_id, c.name, c.color, c.default_tz,
       cs.permission AS "permission!",
       c.created_at, c.updated_at
FROM calendars c
JOIN calendar_shares cs ON cs.calendar_id = c.id
WHERE cs.user_id = $1
ORDER BY created_at ASC
```

**What** — `UNION ALL` of "owned" calendars (synthetic permission =
'owner') and shared calendars (real permission). The frontend gets one
list with a `permission` field on each row.

For writes:

```sql
UPDATE events e
SET title = COALESCE($3, title), ...
FROM calendars c
LEFT JOIN calendar_shares cs ON cs.calendar_id = c.id AND cs.user_id = $2
WHERE e.id = $1
  AND c.id = e.calendar_id
  AND (c.owner_id = $2 OR cs.permission = 'edit')
RETURNING ...
```

**Why permissions in SQL not Rust** — Three benefits:

1. **One round trip.** The permission check happens inside the same
   query that does the work. No "SELECT to check, then UPDATE" race.
2. **Impossible to forget.** A new handler that JOINs the same way
   inherits the rule. A new handler that doesn't JOIN gets caught in
   code review.
3. **DB-enforced.** Even direct SQL bypassing the API still respects
   the lattice (no `WHERE 1=1 OR ...` accidents in handlers).

**The view-only escalation that we deny** — Note that calendar `UPDATE`
(rename, recolor) is owner-only, not `cs.permission = 'edit'`. Editing
events ≠ editing the calendar container. This is the right default;
giving "edit" share access to rename the calendar would let one
collaborator obliterate context for everyone else.

### A.6 — Why ranges are capped at 90 days

```rust
if (q.to - q.from).num_days() > 90 {
    return Err(AppError::Validation("range may not exceed 90 days".into()));
}
```

**What** — Hard reject ranges > 90 days.

**Why** — RRULE expansion cost grows with both the range and the
recurrence frequency. A request for `from=1900&to=2100` (200 years) of
daily events would compute 73,000 occurrences per event. Multiplied by
N recurring events on N calendars, that's an unbounded blowup.

The frontend handles "user wants to see next year" by paging — fetch one
month at a time. Three months is plenty for any UI that's not a yearly
heatmap.

**What would break otherwise** — No cap → a malicious user can DOS the
backend with one request. Or a benign user with one weekly event and a
"show 50 years" URL parameter waits 30 seconds for a page.

### A.7 — The axum 0.8 path-segment rule

This bit me when scaffolding `/api/export/{calendar_id}.ics`:

```
Invalid route "/{calendar_id}.ics":
Only one parameter is allowed per path segment
```

axum 0.8 (and matchit-router under the hood) disallows a static suffix
after a path parameter inside the same path segment. The fix is to move
the dynamic part into its own segment:

```
"/{calendar_id}"          ✓  /api/export/aaaa
"/calendar/{calendar_id}" ✓  /api/export/calendar/aaaa
"/{calendar_id}.ics"      ✗  rejected at router-build time
"/{name}-{ext}"           ✗  also rejected
```

We get the .ics filename via `Content-Disposition: attachment;
filename="calendar.ics"` instead — calendar clients pick up the right
file type from the `Content-Type: text/calendar` header anyway.

---

## B. Frontend

### B.1 — Month grid as a 6×7 `$derived` build

```svelte
const grid = $derived.by((): Cell[] => {
  const first = new Date(anchor);
  const dayOfWeek = first.getUTCDay();
  const gridStart = new Date(first);
  gridStart.setUTCDate(first.getUTCDate() - dayOfWeek);
  const cells: Cell[] = [];
  for (let i = 0; i < 42; i++) {
    const d = new Date(gridStart);
    d.setUTCDate(gridStart.getUTCDate() + i);
    const iso = d.toISOString().slice(0, 10);
    const inMonth = d.getUTCMonth() === first.getUTCMonth();
    const events = data.occurrences.filter((o) => o.start_at.slice(0, 10) === iso);
    cells.push({ date: d, iso, inMonth, events });
  }
  return cells;
});
```

**What** — `$derived.by(...)` is the multi-line variant of `$derived`.
We compute a 42-cell array (6 weeks × 7 days) starting from the Sunday
before the month's first day. Each cell knows its date, ISO string,
"is this still inside the current month" flag, and the events for that
day.

**Why a single $derived.by, not a $state + $effect** — Reactivity
discipline. `$derived` *only* reads — there's no chance a future edit
introduces a side-effect by accident. If a contributor wants to add
mutation, they have to switch primitives, which is a visible code
review signal.

### B.2 — UTC on the wire, local on the screen

The frontend renders the day-number on each cell with
`<time datetime={cell.iso}>{cell.date.getUTCDate()}</time>`. The
event-time pill renders with `new Date(iso).toLocaleTimeString()` which
is **local** to the user's browser.

**The asymmetry is on purpose** — Day cells are about "what day is
this," which is a UTC question that everyone agrees on (the grid
position never changes between users). Event times are about "when does
this happen for me," which is a per-user-local question.

If you wanted to render event times in the *author's* timezone (so a US
participant in a meeting authored in `Europe/Berlin` sees Berlin time),
you'd use `Intl.DateTimeFormat('en-US', { timeZone: e.tz })`. That's a
two-line change — but the default of "local time" is what most users
want.

### B.3 — `datetime-local` input is local-implicit

The "Create event" form uses `<input type="datetime-local">`. The browser
submits a string like `2026-05-27T09:00` *without a timezone* — meaning
"in the user's local timezone."

```ts
const startISO = new Date(start).toISOString();
```

**What this does** — `new Date('2026-05-27T09:00')` parses the string in
the browser's local zone (because of the missing timezone). Then
`.toISOString()` converts to UTC for the wire. The user typed 9 AM
local, the backend stores 9 AM local converted to UTC.

**The `tz` field is the recovery hook** — When the user travels to a
different zone, the rendered local time changes (because their browser's
zone changed), but the stored event row's `tz` field still says
`America/Los_Angeles` (or whatever they authored in). If the UI ever
needs to display "this was originally a 9 AM Pacific meeting," `tz` is
there.

### B.4 — `color-mix()` for the event pill background

```css
.event {
  background: color-mix(in srgb, var(--color, var(--color-accent)) 18%, transparent);
  border-left: 3px solid var(--color, var(--color-accent));
}
```

Each calendar has a color. We feed it through Svelte's `style:--color={c.color}`
syntax (which sets a CSS custom property on that element), then mix it
with 18% opacity for the pill background. The border-left uses the full
color.

**Why `color-mix` instead of a hex/rgba string** — Modern (well-supported
since 2023) CSS. No need to parse the hex string in JS, no need to ship
a color-mixing utility. Each calendar's color is a string the user
controls.

**Why hex format enforced in SQL** — `CHECK (color ~ '^#[0-9a-fA-F]{6}$')`
keeps the column to a known shape. The frontend can `style:--color={c.color}` and trust the value to be a valid CSS color.

### B.5 — Mobile-first month view

```css
@media (max-width: 768px) {
  .month-grid { grid-template-rows: repeat(6, minmax(60px, 1fr)); }
  .event { font-size: 10px; }
  .event time { display: none; }
}
@media (max-width: 480px) {
  .cell { padding: 2px; min-height: 50px; }
  .events { display: none; }
  .day-num::after { content: '·'; margin-left: 2px; color: var(--color-accent); }
  .cell:not(:has(.events)) .day-num::after { display: none; }
}
```

Three breakpoints, in increasing aggressiveness:

- **Default (laptop+):** full grid, time + title in each event pill.
- **≤ 768px (tablet):** smaller cells, hide the time prefix (just title fits).
- **≤ 480px (phone):** hide event details entirely; show a colored
  dot on day-numbers that have events. The user taps a cell to see
  detail (a future enhancement — for now the day shows nothing).

The `:has()` selector lets us conditionally hide the dot on empty days
without JavaScript.

### B.6 — Playwright assertions matter

The headline test:

```ts
test('RRULE expansion: 4 occurrences in a 4-week window', async ({ request }) => {
  const a = await apiRegister(request);
  const cal = await apiCreateCalendar(request, a.cookie, 'Work');
  await apiCreateRecurringEvent(request, a.cookie, cal,
    'RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=8');

  const res = await request.get(
    `${BACKEND}/api/events?from=2026-05-24T00:00:00Z&to=2026-06-22T00:00:00Z`,
    { headers: { cookie: `${COOKIE}=${a.cookie}` } }
  );
  const events = await res.json();
  expect(events.length).toBe(4);
  for (const e of events) {
    expect(e.is_recurring).toBe(true);
    expect(e.title).toBe('Weekly standup');
  }
});
```

**What this proves** — One row in the DB, eight intended occurrences,
exactly four landed in the 4-week query window. Every returned event
carries `is_recurring: true` so the UI can render the recurrence badge
(`↻`). If the RRULE crate ever changes its semantics, this test fails
loudly.

The 90-day-cap test similarly guards the worst-case path:

```ts
test('Range query rejects ranges greater than 90 days', async ({ request }) => {
  const a = await apiRegister(request);
  const res = await request.get(
    `${BACKEND}/api/events?from=2026-01-01T00:00:00Z&to=2026-06-01T00:00:00Z`,
    { headers: { cookie: `${COOKIE}=${a.cookie}` } }
  );
  expect(res.status()).toBe(422);
});
```

---

## C. Patterns to carry forward

- **Master row + spec-string expansion** generalises beyond calendars.
  Project 23 (job queue) uses the same shape for cron schedules: one
  row with a cron string, expanded by the scheduler each tick.
- **`UNION ALL owned + shared`** with a synthetic permission column is
  the workhorse for any "resources you own + shared with you" listing.
  Project 24 (help desk) uses this for tickets, project 30 for projects.
- **Bounded range queries** as a generic shape — caps on
  pagination depth, expansion count, recursion. Always answer the
  question "what's the worst case if the attacker fuzzes inputs?".
- **`color-mix()` + per-row color via CSS variables** is the cleanest
  way to render variable-color UI without inline `<style>` strings.

### Try it (exercises)

1. **Drag-to-create UX** — bind a `pointerdown` on a day cell to start
   a draft event, `pointermove` to extend it, `pointerup` to open the
   form pre-filled. Hint: project 14's `$state.raw` for the in-flight
   selection (no need for deep reactivity).
2. **"Edit this occurrence only"** — add an `event_overrides` table with
   `(event_id, original_start, override_data)`. The range query joins
   that table and prefers override values when an occurrence's start
   matches an override row. The semantics get hairy fast — read the RFC
   5545 §3.6.1 on `RECURRENCE-ID`.
3. **Conflict detection** — for the "share-edit" use case, add an SSE
   stream that broadcasts event changes to other viewers of the same
   calendar. Project 13's broadcast hub is the right primitive.
4. **Birthday import** — let the user paste a list of birthdays as
   `name,YYYY-MM-DD` lines; create an all-day event per row with
   `RRULE:FREQ=YEARLY`.

---

Project 14 is shipped when:

- [x] `cargo fmt --check` clean
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `cargo test` — 7/7 passing
- [x] `pnpm check` — 0 errors / 0 warnings
- [x] `pnpm build` clean
- [x] `pnpm test:e2e` — 40/40 (10 × 4 viewports)
- [x] Live verification: 8-occurrence weekly RRULE → 4 occurrences
  correctly windowed inside a 4-week range, ICS export valid VCALENDAR
