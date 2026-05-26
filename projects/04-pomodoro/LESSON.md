# Project 04 — Lesson

> Read alongside the code. This project's headline lesson is **`$effect` with cleanup driving a `requestAnimationFrame` loop**, with `untrack` for the read-write-same-variable case. That pattern shows up in every project that animates anything: timers, scroll-driven UIs, drag gestures, real-time charts. Get it right once and it's yours forever.

---

## A. Backend

### A.1 — `Cargo.toml`

Identical to project 03 minus `proptest`. No new deps.

### A.2 — `migrations/0001_init.sql`

```sql
CREATE TABLE IF NOT EXISTS sessions (
    id                 TEXT PRIMARY KEY,
    kind               TEXT NOT NULL CHECK (kind IN ('work', 'short_break', 'long_break')),
    label              TEXT,
    planned_seconds    INTEGER NOT NULL,
    actual_seconds     INTEGER NOT NULL,
    started_at         TEXT NOT NULL,
    ended_at           TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions (started_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_kind ON sessions (kind);
```

The new SQL idea: **`CHECK (kind IN (...))`**. The DB rejects any insert with a `kind` value outside the whitelist. We also validate in `normalize_kind` on the backend — defense in depth. If the app code has a bug, the DB stops the bad row.

Two indexes:
- `started_at DESC` for the history list query (`ORDER BY started_at DESC LIMIT 50`).
- `kind` for the stats endpoint (`WHERE kind = 'work'`).

`planned_seconds` (what the user signed up for: 1500 for a 25-min Pomodoro) vs `actual_seconds` (what they actually did before reset/skip) gives us the data to surface "completion rate" or "you bailed out 4 times today" later. We don't expose that UI yet, but the data is captured.

### A.3 — `src/error.rs` and `src/db.rs`

Identical to project 03. Reference back.

### A.4 — `src/routes/sessions.rs`

Four endpoints:
- `GET /api/sessions?limit=N` — list, default 50, clamped 1..=500.
- `POST /api/sessions` — record a completed session.
- `DELETE /api/sessions/:id`
- `GET /api/sessions/stats` — aggregate counts.

The validation block in `create`:

```rust
let kind = normalize_kind(&payload.kind)?;
let label = payload.label.as_deref().map(normalize_label).transpose()?;
validate_seconds(payload.planned_seconds, "planned_seconds")?;
validate_seconds(payload.actual_seconds, "actual_seconds")?;
if payload.ended_at < payload.started_at {
    return Err(AppError::Validation("ended_at must be >= started_at".into()));
}
```

Five things checked before we touch the DB:
- Kind is in the whitelist.
- Label is ≤ 120 chars (if present).
- Both seconds are in 0..=12h.
- The end timestamp is not before the start.

Cheap to do. Saves us from inserting nonsense rows that confuse the stats query later.

#### The stats query — `COALESCE` is the move

```rust
sqlx::query!(
    r#"
    SELECT COALESCE(SUM(actual_seconds), 0) AS "focus!: i64"
    FROM sessions
    WHERE kind = 'work' AND started_at >= ?1
    "#,
    today_start,
)
```

Why `COALESCE(SUM(...), 0)`? Because `SUM` on zero rows returns `NULL`, not `0`. Without `COALESCE`, the column type is `Option<i64>`, and we'd need `.unwrap_or(0)` everywhere. By coalescing in SQL, the column is non-null and sqlx infers `i64` directly. (The `"focus!: i64"` annotation tells sqlx to force-decode as non-null.)

This is a foundational SQL idiom. **`SUM`/`AVG`/`MAX` on empty sets return NULL** — every Postgres / SQLite query that aggregates user data should `COALESCE`.

#### Why server-side "today" is approximate

`today_utc_iso()` returns midnight UTC of the current UTC day. If a user is in Tokyo (UTC+9) and looks at their dashboard at 6 AM JST (= 9 PM UTC the previous day), our backend's "today" doesn't match their wall clock. For an unauthenticated single-user MVP, this is acceptable. Project 14 (calendar & scheduler) introduces proper per-user timezone handling.

### A.5 — `src/main.rs`

Same shape as previous projects. Module list is unchanged from project 02. Default port: 3003. CORS origin: `http://localhost:5176`.

---

## B. Frontend

### B.1 — `package.json` + configs

No new deps. All lessons in this project are runes/CSS/Web Audio — pure browser APIs.

### B.2 — `src/lib/types.ts`

```ts
export type SessionKind = 'work' | 'short_break' | 'long_break';

export const KIND_SECONDS: Record<SessionKind, number> = {
  work: 25 * 60,
  short_break: 5 * 60,
  long_break: 15 * 60
};
```

`Record<SessionKind, number>` is the TypeScript idiom for "an object with exactly these keys, each mapping to a number". TypeScript verifies all three keys are present and rejects any extra. Same for `KIND_LABEL` and `KIND_COLOR`. Three parallel tables keyed by the same union type — clean.

### B.3 — `src/lib/chime.ts` — Web Audio API

```ts
let ctx: AudioContext | null = null;

function getCtx(): AudioContext | null {
  if (typeof window === 'undefined') return null;
  const Ctor = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!Ctor) return null;
  if (!ctx) ctx = new Ctor();
  return ctx;
}

export function playChime(): void {
  const c = getCtx();
  if (!c) return;
  if (c.state === 'suspended') void c.resume();
  playTone(c, 880, c.currentTime, 0.28);          // A5
  playTone(c, 659.25, c.currentTime + 0.16, 0.42); // E5 — a fifth below
}

function playTone(c: AudioContext, freq: number, startAt: number, durationSec: number) {
  const osc = c.createOscillator();
  const gain = c.createGain();
  osc.type = 'sine';
  osc.frequency.setValueAtTime(freq, startAt);
  gain.gain.setValueAtTime(0.0001, startAt);
  gain.gain.exponentialRampToValueAtTime(0.25, startAt + 0.02);    // attack
  gain.gain.exponentialRampToValueAtTime(0.0001, startAt + durationSec);  // decay
  osc.connect(gain).connect(c.destination);
  osc.start(startAt);
  osc.stop(startAt + durationSec + 0.05);
}
```

Three ideas in this file:

1. **Lazy singleton AudioContext.** Each call to `new AudioContext()` allocates a small audio graph; we only want one. The lazy init guards against SSR (`typeof window === 'undefined'`) and old WebKit (`webkitAudioContext`).

2. **The `suspended` state.** Browsers (especially mobile Safari) start `AudioContext` in `suspended` state until a user gesture happens. The fix is `c.resume()`. We `void` the promise because we don't need to await — the chime is fire-and-forget. (We only call `playChime` from a finished session, which always happens after the user clicked Start, so the gesture-unlock has happened.)

3. **The chime itself.** Two sine waves, A5 (880 Hz) then E5 (659 Hz, a perfect fifth below). Each has an envelope: ramp gain from near-zero up to 0.25 in 20ms (attack), then exponentially decay back to near-zero over the rest of the duration. Real instruments work like this; without an envelope, sine waves "click" abruptly when they start/stop.

`exponentialRampToValueAtTime` requires non-zero values (you can't ramp to 0), hence the `0.0001` start/end. Tiny detail, gotcha if you ever try to set it to 0.

### B.4 — `src/routes/+page.svelte` — the timer

This is the most concept-dense single file in the curriculum so far. Read it in pieces.

#### Reactive state

```ts
let mode = $state<SessionKind>('work');
let label = $state('');
let running = $state(false);
let elapsedMs = $state(0);
let startedAt: string | null = $state(null);
```

Five `$state` bindings:
- `mode` — the current session kind (drives total time, color, label).
- `label` — the optional text label the user typed.
- `running` — whether the clock is ticking.
- `elapsedMs` — how many milliseconds have elapsed in this run.
- `startedAt` — ISO timestamp captured the first time the timer started this run (used as `started_at` when we record).

#### Derived clock + progress

```ts
const totalSec = $derived(KIND_SECONDS[mode]);
const remainingSec = $derived(Math.max(0, totalSec - Math.floor(elapsedMs / 1000)));
const progress = $derived(Math.min(1, elapsedMs / (totalSec * 1000)));
```

All read-only from `mode` and `elapsedMs`. Whenever either changes, derived values recompute. We never set them directly. **The lesson**: derived values mean fewer bindings, fewer chances to forget to update something.

#### The timer effect — the headline pattern

```ts
$effect(() => {
  if (!running) return;

  let raf = 0;
  const startPerf = performance.now();
  const startElapsed = untrack(() => elapsedMs);
  const totalMs = untrack(() => KIND_SECONDS[mode]) * 1000;

  function tick(now: number) {
    const next = startElapsed + (now - startPerf);
    if (next >= totalMs) {
      elapsedMs = totalMs;
      running = false;
      finishSession();
      return;
    }
    elapsedMs = next;
    raf = requestAnimationFrame(tick);
  }

  raf = requestAnimationFrame(tick);
  return () => cancelAnimationFrame(raf);
});
```

Five things going on. Each one matters.

**1. Early return guards re-entry.** The effect runs initially, but `running` is `false`, so we return immediately. When the user clicks Start, `running` becomes `true`, the effect re-runs, and now we schedule the RAF.

**2. We snapshot `performance.now()` and `elapsedMs` once at startup.** `startPerf` is the high-resolution timestamp when the run began. `startElapsed` is how much was elapsed at that moment (zero on first start; >0 if the user paused and resumed).

**3. `untrack()` is the magic word.** If we wrote `const startElapsed = elapsedMs`, the effect would track `elapsedMs` as a dependency. Then every time `elapsedMs` changed inside the `tick` callback, the effect would re-run — cancelling its own RAF mid-frame. The result: the timer never advances past the first frame.

`untrack(() => elapsedMs)` reads the value without recording the read as a dependency. Result: the effect only re-runs when `running` (or other tracked deps) change, not when `elapsedMs` does. Reads inside `tick` (which runs later, asynchronously) are not tracked either — Svelte only tracks synchronous reads during the effect body's execution.

**4. The math uses `performance.now() - startPerf`, not a counter.** A naïve implementation would do `elapsedMs += 16` each tick. But RAF doesn't fire at exactly 60 fps — it pauses when the tab is backgrounded, varies on slow devices, etc. Computing elapsed as a delta from a fixed start guarantees the clock advances by exactly the wall-time elapsed, regardless of how many frames fired. **This is the canonical animation timing pattern.** Memorize it.

**5. The cleanup function — `return () => cancelAnimationFrame(raf)`.** Svelte calls this when the effect re-runs (because `running` changed to false, say) AND when the component unmounts. Without it, the user could click Pause, the effect would re-run with `running === false`, the early return would fire, but the previous `tick` would still be scheduled — and would happily mutate `elapsedMs` once more before noticing `running` is false.

Without the cleanup, you'd see the clock advance by one extra frame after pausing. With it, the clock stops cleanly.

**Try this**: comment out the `return () => cancelAnimationFrame(raf)` line. Pause the timer and watch the time keep ticking down by one second before stopping. That's the cost of forgetting cleanup. Then put it back. Now you've felt the value.

#### The second effect — capturing the start time

```ts
$effect(() => {
  if (running && !startedAt) {
    startedAt = new Date().toISOString();
  }
});
```

This effect runs whenever `running` or `startedAt` change. On the first frame after Start, it sets `startedAt` to the wall-clock ISO. On subsequent pauses-and-resumes, `startedAt` is already non-null so we skip — the resumed session keeps its original start time, which is what we want for the recorded `started_at`.

Could we have done this in `toggleRunning()`? Yes. We did it as an effect to demonstrate that effects are also for synchronizing derived state, not just for side effects to external APIs.

#### `$inspect` — the debugger

```ts
$inspect('pomodoro', { mode, running, remainingSec });
```

That single line logs `'pomodoro' { mode: 'work', running: false, remainingSec: 1500 }` to the browser console whenever any of those three values change. It's the easiest way to see, live, what's happening in your reactive graph.

In production you'd strip it (or wrap with `if (import.meta.env.DEV)`). In development it's invaluable — way better than `console.log` inside an effect, because `$inspect` runs at exactly the right time (after the reactive update settles) and never fires when nothing changed.

#### Keyboard shortcuts via `<svelte:window>`

```svelte
<svelte:window onkeydown={handleKey} />
```

```ts
function handleKey(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null;
  if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return;
  switch (e.key) {
    case ' ': e.preventDefault(); toggleRunning(); break;
    case 'r': case 'R': e.preventDefault(); resetTimer(); break;
    case '1': chooseMode('work'); break;
    case '2': chooseMode('short_break'); break;
    case '3': chooseMode('long_break'); break;
  }
}
```

`<svelte:window>` attaches an event listener to `window` and removes it when the component unmounts. The `target` check skips the handler when the focus is in a form input — otherwise pressing Space inside the label field would pause the timer instead of typing a space.

`e.preventDefault()` on Space stops the page from scrolling (the default action of Space on a focused button).

#### Optimistic UI via the hidden record form

```svelte
<form
  bind:this={recordForm}
  method="POST"
  action="?/record"
  use:enhance={() => async ({ update }) => {
    await update();
    await invalidateAll();
  }}
  hidden
>
  <input type="hidden" name="kind" value={recordPayload.kind} />
  ...
</form>
```

When the timer finishes, `finishSession()` populates `recordPayload`, then calls `recordForm?.requestSubmit()` on a microtask. The form submits to `?/record`, which calls `sessionsApi.create` server-side. `use:enhance` intercepts the submit — no page reload — and `await update()` refreshes form state. We then call `invalidateAll()` to re-run the page's `load`, which fetches the new session list + updated stats. The history list updates without the user ever seeing a loading state.

This is "optimistic" only in that the chime + UI reset happen immediately, before the server confirms. If the server returns an error, the chime already played and the user already saw the celebration. That's fine for a personal app; for a multi-tenant SaaS you'd handle the error and queue a retry. Project 13 (chat rooms) revisits this with proper queue-and-retry semantics.

### B.5 — The `aria-live="polite"` clock

```svelte
<p class="clock" aria-live="polite" aria-atomic="true">
  <span class="time" data-testid="clock">{formatClock(remainingSec)}</span>
</p>
```

Screen readers announce changes to `aria-live` regions. `polite` means "announce when the user is idle, don't interrupt". `atomic="true"` means "announce the whole region, not just the diff". Without atomic, the user might hear "two three colon five seven" → "two three colon five six" → ..., one digit per change. With atomic, they hear "23:57" → "23:56" → ... — much less noisy.

We could go further (announce only every 5 minutes, say) but for a Pomodoro the per-second granularity is mostly going to be ignored — the user will hear the chime when it matters.

### B.6 — `prefers-reduced-motion`

```css
@media (prefers-reduced-motion: reduce) {
  .ring {
    transition: none;
  }
}
```

Same pattern as project 03. The progress ring uses a `conic-gradient` that updates continuously as `--progress` changes; the `transition` smooths the visual update. When the user has opted out of motion, we drop the transition. Playwright verifies it: `getComputedStyle(ring).transitionDuration === '0s'`.

---

## C. Tests

| Layer | What's covered |
| --- | --- |
| cargo (8) | Kind whitelist, label trim/length, seconds bounds, kind lowercasing. |
| vitest (6) | API client: list, create, remove, stats, error mapping, encoded paths. |
| playwright (24 = 6 × 4) | a11y, initial state, Space toggle, mode keys, full record-via-API roundtrip, reduced-motion. |

### A note on testing the timer

The Playwright spec doesn't run the 25-minute timer in real time (too slow) or mock RAF (too fragile). Instead, the "recorded session shows in history" test calls `request.post` to seed a session directly on the backend, then loads the page and asserts the history row.

The timer mechanics are covered by `pnpm dev` — you exercise them by hand, you read the code, you see them work. Some things are just easier to verify visually than to test. **The lesson**: don't write fragile tests for things better verified manually. Write fast tests for things the human-loop is slow at (API surface, accessibility, lifecycle correctness).

If you wanted to test the timer deterministically, the canonical approach is to mock `performance.now` and `requestAnimationFrame` and drive them by hand. That's about 60 lines of test infrastructure for one test. Not worth it for this scope.

---

## D. Closing — what you can do now

- Write `$effect` that schedules a `requestAnimationFrame` loop with proper cleanup. You can explain why `untrack` is necessary when reading and writing the same `$state` in the same effect.
- Synthesize a chime tone with the Web Audio API. You can explain the suspended state, the envelope (attack/decay), and the exponential-ramp-zero gotcha.
- Wire `<svelte:window>` for global keyboard shortcuts that don't interfere with form inputs.
- Use `$inspect` to debug reactivity without `console.log`-spam.
- Build an "optimistic UI" — the user sees instant feedback, the server roundtrip happens in the background.
- Honor `aria-live` for time-varying UI without overwhelming screen-reader users.

Open `projects/05-bookmarks/COMMANDS.md` next. Bookmark Manager with Tags — many-to-many SQL, URL-as-state filters (`?tag=rust&q=axum`), debounced search, `use:clickOutside` action, link-preloading on hover. Different lessons, same spine.
