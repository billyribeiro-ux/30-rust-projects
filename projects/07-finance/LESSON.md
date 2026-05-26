# Project 07 — Lesson

Headline lessons:
1. **Double-entry validation** — every transaction's postings must sum to zero. Pure validator + proptest invariants.
2. **`rust_decimal` for safe parsing** of user-typed and CSV-imported amounts; reject inputs whose precision would force rounding.
3. **Canvas-drawn charts** — no chart library, ~80 lines of TypeScript, DPR-correct, redraws on container resize via `ResizeObserver`.

---

## A. Backend

### A.1 — The schema

```sql
CREATE TABLE accounts (id, name UNIQUE, kind CHECK in (...), color, created_at);
CREATE TABLE transactions (id, description, occurred_at, created_at);
CREATE TABLE postings (
    id, transaction_id FK, account_id FK,
    amount_minor INTEGER NOT NULL
);
```

Two key decisions:
- **`amount_minor` is signed**. Positive = debit, negative = credit. The convention is arbitrary (some systems split into separate debit/credit columns) but having one signed column means "sum to zero" is a single SQL `SUM`.
- **`accounts.kind` has a CHECK constraint** (`asset/liability/income/expense/equity`). Same defense-in-depth pattern as project 04 — the application normalizes input, the DB rejects anything that slips through.

The schema does NOT enforce "transaction postings sum to zero". SQLite can't run aggregate CHECKs across rows. The application validator is the source of truth, backed up by proptest. This is the **load-bearing invariant of the entire app** — get it wrong and your balances drift.

### A.2 — `ledger.rs` — the validator

```rust
pub fn validate_postings(postings: &[PostingInput]) -> AppResult<()> {
    if postings.len() < 2 {
        return Err(...);  // "at least two postings"
    }
    let mut seen = std::collections::HashSet::new();
    let mut sum: i128 = 0;
    for (i, p) in postings.iter().enumerate() {
        if p.amount_minor == 0 { return Err(...); }
        if !seen.insert(p.account_id.clone()) { return Err(...); }
        sum = sum.checked_add(p.amount_minor as i128).ok_or(...)?;
    }
    if sum != 0 { return Err(...); }
    Ok(())
}
```

Four rules:
1. At least two postings (a single one can't balance).
2. No zero postings (a zero is a no-op + signals a bug).
3. No duplicate account ids in the same transaction (would represent two events conflated).
4. Sum of `amount_minor` is **exactly zero**.

We accumulate the sum in `i128` to make overflow impossible for any practical input. `checked_add` is belt-and-suspenders — if your portfolio is large enough to overflow `i128`, you have bigger problems.

#### Proptest invariants

```rust
proptest! {
    #[test]
    fn balanced_always_validates(...) {
        // Sample N-1 random amounts, append the negative of their sum.
        // The result must validate.
    }

    #[test]
    fn unbalanced_never_validates(amounts in prop::collection::vec(1i64..=1_000_000, 2..=10)) {
        // All positive amounts → sum > 0 → must NOT validate.
    }
}
```

These two properties cover both directions of the invariant: the validator must accept any balanced input AND reject any unbalanced input. Together with the 6 unit tests, the validator is exhaustively verified.

**Try this**: remove the `if sum != 0` line. `unbalanced_never_validates` finds a counterexample in milliseconds.

### A.3 — `rust_decimal` for safe number parsing

```rust
pub fn decimal_to_minor(d: Decimal) -> Result<i64, String> {
    if d.scale() > 2 {
        return Err(format!("amount has more than 2 decimal places: {}", d));
    }
    let minor = d * Decimal::new(100, 0);
    if minor.fract() != Decimal::ZERO {
        return Err("amount must be whole cents".into());
    }
    i64::try_from(minor.trunc().mantissa()).map_err(|_| "amount overflows i64".into())
}
```

We use `rust_decimal::Decimal` to parse strings like `"12.34"`, multiply by 100 (exactly, no floats), check the result is a whole number, convert to `i64`. Reject anything with more than 2 decimal places — those would force a rounding decision, which we'd rather force the user to disambiguate.

This is the canonical safe-money parsing in Rust. Don't use `f64`. Don't manually split on `.`. Use `Decimal`.

The serde feature `serde-with-str` makes `Decimal` serialize/deserialize as a string in JSON (`"12.34"` not `12.34`). That preserves precision across the wire — JSON numbers are floats in many parsers.

### A.4 — The CSV importer

```rust
let mut rdr = csv::ReaderBuilder::new().has_headers(true).trim(csv::Trim::All).from_reader(req.csv.as_bytes());
let mut tx = pool.begin().await?;
for (i, rec) in rdr.records().enumerate() {
    // Parse row → date, description, amount, counterparty name.
    // Lookup counterparty by name; skip if missing.
    // Insert balanced transaction (asset side + counterparty side).
}
tx.commit().await?;
```

Per-row failure handling: append to `errors: Vec<String>`, increment `skipped`, continue. The successful rows are committed atomically at the end. If anything panics mid-loop, the transaction rolls back and the user can fix the CSV and retry.

The frontend gets `{ imported: N, skipped: M, errors: [...] }` back — the user sees both what succeeded AND what failed with specific line numbers. Better UX than a generic "import failed" toast.

### A.5 — The `accounts` list query

```sql
SELECT a.id, a.name, a.kind, a.color, a.created_at,
       COALESCE((SELECT SUM(amount_minor) FROM postings WHERE account_id = a.id), 0) AS "balance!: i64"
FROM accounts a
ORDER BY a.created_at ASC
```

Each account's running balance is computed by a correlated subquery summing all its postings. For 100 accounts × 10000 postings, this is 100 sub-SUMs — SQLite handles it in milliseconds. For a real production app you'd cache the balance on the account row and update it on every posting insert. For an MVP, the subquery is fine and the code is simpler.

---

## B. Frontend

### B.1 — `lib/money.ts`

Same shape as project 06: `parseMoney`, `formatMoney`, `formatCents`. Centralizing the parse/format functions in one file means there's one place to audit for float bugs.

### B.2 — `BarChart.svelte` — canvas from scratch

```svelte
let canvas: HTMLCanvasElement | undefined = $state();
let width = $state(0);

// ResizeObserver feeds width into reactive state
$effect(() => {
  if (!canvas) return;
  const ro = new ResizeObserver((entries) => {
    const w = entries[0]?.contentRect.width ?? 0;
    if (w > 0) width = Math.floor(w);
  });
  ro.observe(canvas);
  return () => ro.disconnect();
});

// Redraw whenever data or width changes
$effect(() => {
  if (!canvas) return;
  if (width === 0) return;
  const items = data;  // make the effect track data

  const dpr = window.devicePixelRatio || 1;
  canvas.width = width * dpr;
  canvas.height = height * dpr;
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;
  const ctx = canvas.getContext('2d');
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  // ... draw bars ...
});
```

Four ideas:

**Two effects, two responsibilities.** The first observes container size and pushes to `width`. The second redraws whenever `data` OR `width` changes. Splitting them keeps each effect simple.

**Device pixel ratio (DPR) is essential.** A 1× canvas on a Retina display looks like blurry mush. We size the canvas backing store to `width * dpr` pixels and set the CSS dimensions to `width` px. Then `setTransform(dpr, 0, 0, dpr, 0, 0)` makes every drawing command scale automatically. Forgetting DPR is the #1 mistake people make with `<canvas>`.

**The `items = data` line is a touch.** Without it, the effect wouldn't track `data` (since we only use it via the closure). Reading the value once at the top of the effect makes it a tracked dependency.

**`ResizeObserver` + `$effect` cleanup.** The observer is disconnected in the cleanup function, so we don't leak observers across mounts.

#### Why not use a chart library?

For *this* chart, the library would add 30-50 KB to the bundle and most of its features go unused. Hand-rolling 80 lines gives you:
- Full control over visual style (matches the design tokens automatically)
- Zero abstraction overhead — what you read is what runs
- DPR support, ResizeObserver — both 5-line additions
- No upgrade path / breaking changes from someone else's roadmap

For a *complex* chart (interactive zoom, tooltips with formatters, time-series with smart axis ticks), a library like Chart.js or Apache ECharts pays off. Project 27 (realtime analytics dashboard) introduces one. For a single bar chart, hand-rolling is the right call.

### B.3 — A11y for canvas

```svelte
<div class="wrap" role="img" aria-label={ariaLabel}>
  <canvas bind:this={canvas}></canvas>
</div>
```

`<canvas>` cannot have `role="img"` directly (axe/Svelte's a11y rules reject it because canvas is an interactive element). The workaround: wrap in a `<div role="img">` with the aria-label. The screen reader sees "Top expense balances" or whatever; sighted users see the chart.

For a fully accessible chart you'd also provide a hidden `<table>` with the data — `aria-describedby` pointing to it — so a screen reader can read the underlying numbers. We don't yet; project 27 will.

### B.4 — The transaction form: derived payload

The transaction form computes a balanced two-posting payload from the user's "from + to + amount" inputs:

```ts
const payloadJson = $derived(() => {
  if (!amount || !fromId || !toId || fromId === toId) {
    return JSON.stringify({ description, occurred_at: '', postings: [] });
  }
  return JSON.stringify({
    description,
    occurred_at: new Date().toISOString(),
    postings: [
      { account_id: fromId, amount_minor: -amount },
      { account_id: toId, amount_minor: amount }
    ]
  });
});
```

The "from" account gets a negative posting (money leaving). The "to" account gets a positive posting (money arriving). Sum = zero. Always.

If the user picks the same account for both, or leaves an input empty, the payload is `postings: []` and the submit button is disabled (`txReady` derived). The server's validator is the safety net — if our derivation has a bug, the server says 422 and the form shows the field error.

The frontend form mirrors what real accounting software (Beancount, Ledger, hledger) does: a two-leg simple entry. The backend's API accepts arbitrary multi-leg transactions, so you can extend the UI later (split rent across roommates, depreciate an asset over time) without changing the API.

---

## C. Tests

| Layer | Coverage |
| --- | --- |
| cargo (10 = 8 unit + 2 proptest) | Balanced/unbalanced/zero/duplicate posting rejection; decimal-to-minor round-trip; rejection of 3-decimal-place input. Two proptest properties covering both directions of the sum-zero invariant. |
| vitest (3) | API client list/create; field-level errors surface on ApiCallError.fields. |
| playwright (20 = 5 × 4) | a11y, seed accounts + transaction via API, unbalanced transaction returns 422 with field error, submit-button-disabled logic, canvas chart renders with proper accessible name. |

---

## D. Closing — what you can do now

- Design and enforce double-entry accounting integrity. You understand why the validator + proptest are the load-bearing safety net.
- Parse and store money safely from any source — user input, CSV, JSON — with `rust_decimal` + integer minor units.
- Write a CSV importer that handles per-row failures gracefully and reports them.
- Draw bar charts (and by extension line charts, sparklines, donuts, heatmaps) directly on a `<canvas>`. You know about DPR scaling and `ResizeObserver`, the two things people forget.

Project 08 — **Recipe Book with Photos** — last SQLite project. First file upload (multipart, mime-sniffed, EXIF stripped). Recipe JSON-LD for Google rich results. First service worker. Then we move to Postgres 16 + Docker Compose at project 11.
