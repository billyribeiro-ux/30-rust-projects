# Project 06 — Lesson

> Read alongside the code. Headline lessons: **money as integer cents**, **the pure splitter function with proptest**, **`$bindable` for reusable form inputs**, **field-level errors** mapped from server to specific inputs.

---

## A. Backend

### A.1 — Money on the wire

The DB stores `amount_cents INTEGER NOT NULL CHECK (amount_cents > 0)`. JSON sends `amount_cents: 1234` (number, an integer). Rust uses `i64`. Frontend uses `number` (which, since it's an integer < 2^53, is exact). At no point does a float dollar amount touch state.

Why? **`0.1 + 0.2 === 0.30000000000000004`**. IEEE-754 doubles can't represent some decimal fractions exactly. Sum a few hundred such "amounts" and your "$100.00 total" is "$99.97 with a long tail of decimals". With integer cents, every arithmetic operation is exact, and the round-trip wire format is unambiguous.

Database `CHECK (amount_cents > 0)` is defense in depth. The route's `normalize` validator also rejects ≤ 0. Both layers exist so a future code bug can't insert nonsense.

### A.2 — `splitter.rs` — the pure function

The three split kinds, all distilled to one signature:

```rust
pub fn split(amount_cents: i64, kind: SplitKind, shares: &[ShareInput]) -> AppResult<Vec<ShareOutput>>
```

`ShareInput.value` is overloaded: cents for `Exact`, basis points (0-10_000 = 0-100%) for `Percent`, ignored for `Equal`. The function returns `share_cents` per member.

**Invariants every kind guarantees:**
- Sum of returned `share_cents` == `amount_cents` (exactly, never off by a cent)
- Each share is non-negative
- The input list has no duplicate member ids
- The amount is > 0

The first three are the **load-bearing** invariants. Property tests verify them on thousands of random inputs (`amount` from 1¢ to $1M × 1-12 members) for `Equal` and `Percent`. `Exact` skips the property test because the user provides the answer — the property would be tautological.

#### Equal split — the remainder distribution

```rust
let base = amount_cents / n;
let remainder = amount_cents - base * n;  // in [0, n)
shares.iter().enumerate().map(|(i, s)| ShareOutput {
    member_id: s.member_id.clone(),
    share_cents: base + if (i as i64) < remainder { 1 } else { 0 },
})
```

For $1.00 across 3 members: base = 33, remainder = 1. First member gets 34, others get 33. Sum = 100. Always.

The "first members get the extra cent" rule is convention. Some apps rotate it across expenses to be fair over time; we don't. (For an n=3 group, the first listed member would "overpay" by 1¢ every meal, which over a year is $20. We accept this for simplicity — see the proptest `equal_max_difference_is_one_cent`.)

#### Percent split — largest remainder method

```rust
let mut out: Vec<(usize, i64, i64)> = shares.iter().enumerate().map(|(i, s)| {
    let exact = amount_cents.saturating_mul(s.value);
    let cents = exact / 10_000;
    let remainder = exact - cents * 10_000;
    (i, cents, remainder)
}).collect();

let assigned: i64 = out.iter().map(|(_, c, _)| *c).sum();
let mut leftover = amount_cents - assigned;

let mut order: Vec<usize> = (0..out.len()).collect();
order.sort_by(|&a, &b| out[b].2.cmp(&out[a].2).then(out[a].0.cmp(&out[b].0)));
let mut k = 0;
while leftover > 0 {
    let pos = order[k % order.len()];
    out[pos].1 += 1;
    leftover -= 1;
    k += 1;
}
```

For $1.00 split 33.33% / 33.33% / 33.34%:
- exact: 100 × 3333 = 333300, 100 × 3333 = 333300, 100 × 3334 = 333400
- cents: 33, 33, 33; remainders: 3300, 3300, 3400
- assigned: 99; leftover: 1
- order by remainder desc: [2, 0, 1] (the 33.34% goes first, ties broken by index)
- distribute 1 leftover cent → member 2 gets +1 → final: 33, 33, 34. Sum = 100.

This is the **Hamilton / largest remainder method**, used in apportionment math (allocating legislative seats by population) and in our case allocating cents by percentage. The math guarantees the sum equals the total, even with edge cases like 100% to one member or 50/50 splits.

#### Why proptest matters here

The property `percent_always_sums_to_amount` runs ~1024 random scenarios per build. If you mutate the algorithm — say, change `Math.floor` to `Math.round` or break the leftover-distribution loop — proptest produces a minimal counterexample (smallest amount + member count that fails) within seconds. Try it: remove the `while leftover > 0` loop and re-run `cargo test`.

### A.3 — `error.rs` — field-level errors

```rust
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    Validation(String),
    Fields(Vec<FieldError>),
    NotFound,
    Database(#[from] sqlx::Error),
}
```

Response JSON:

```json
{ "error": { "code": "validation_failed", "message": "...", "fields": [
    { "field": "amount_cents", "message": "must be greater than 0" },
    { "field": "shares", "message": "duplicate member: m1" }
]}}
```

The frontend's `ApiCallError` parses `error.fields` into a typed array. The form can render each error next to its corresponding input — much better UX than a single "validation failed" toast.

The `members.rs` route uses this pattern to surface DB constraint failures as field errors:

```rust
.map_err(|err| {
    if let sqlx::Error::Database(dbe) = &err
        && dbe.message().contains("UNIQUE constraint failed")
    {
        return AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "a member with that name already exists".into(),
        }]);
    }
    AppError::Database(err)
})?;
```

The DB enforces uniqueness on `members.name`. If the application logic ever fails to check, the DB still rejects — but we intercept the raw error and reshape it into a friendly field error. Rust 2024 `let-chains` (the `if let ... && ...` pattern) makes this read cleanly.

### A.4 — `balances.rs` — the greedy settlement

```rust
fn compute_settlements(balances: &[Balance]) -> Vec<Settlement> {
    let mut creditors = balances.iter()
        .filter(|b| b.net_cents > 0)
        .map(|b| (b.member_id.clone(), b.net_cents))
        .collect::<Vec<_>>();
    let mut debtors = balances.iter()
        .filter(|b| b.net_cents < 0)
        .map(|b| (b.member_id.clone(), -b.net_cents))
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    while let (Some(c), Some(d)) = (creditors.last_mut(), debtors.last_mut()) {
        let amount = c.1.min(d.1);
        if amount == 0 { break; }
        out.push(Settlement { from: d.0.clone(), to: c.0.clone(), cents: amount });
        c.1 -= amount; d.1 -= amount;
        if c.1 == 0 { creditors.pop(); }
        if d.1 == 0 { debtors.pop(); }
    }
    out
}
```

The greedy algorithm pairs the largest creditor with the largest debtor, has the debtor pay the lesser of the two amounts, removes the now-zero side, repeats. Produces **at most N-1 transactions** for N members.

This is **not** always the minimum number of transactions — for that you'd need to solve the NP-hard subset-sum problem to detect "X owes A exactly what A owes Y" chains. For a household of 4-8 people, the greedy result is good enough and runs in O(N).

---

## B. Frontend

### B.1 — `lib/money.ts` — the centralization

```ts
export function parseMoney(input: string): number | null {
  const cleaned = input.replace(/[\s,$]/g, '').replace(/[^\d.\-]/g, '');
  if (cleaned === '' || cleaned === '-' || cleaned === '.') return null;
  // ... handles "12", "12.34", "12.5", "-12.34", "$1,234.56"
  // rejects "12.345" (would lose precision)
}

export function formatCents(cents: number): string { ... }
export function formatMoney(cents: number, currency = 'USD'): string {
  return new Intl.NumberFormat(undefined, { style: 'currency', currency, ... }).format(cents / 100);
}
```

**One place** that parses user strings to cents. **One place** that formats cents to display strings. If the parsing logic ever changes (new currency, different separator conventions), there is exactly one file to update. If you accidentally use raw `Number(amountString) * 100` somewhere, you've created an island of float math.

The test for `parseMoney("12.345")` returning `null` is the key: three-decimal-place input would force a rounding decision (down? half-even?), and any decision is an opportunity for cents to be lost. We reject the input instead — the user fixes their typo.

### B.2 — `MoneyInput.svelte` — `$bindable`

```svelte
<script lang="ts">
  type Props = {
    cents?: number | null;
    label: string;
    name?: string;
    /* ... */
  };

  let {
    cents = $bindable(null),
    label,
    /* ... */
  }: Props = $props();

  let text = $state(cents === null || cents === undefined ? '' : formatCents(cents));

  function onInput(e: Event) {
    text = (e.target as HTMLInputElement).value;
    cents = parseMoney(text);  // writing to a $bindable prop notifies the parent
  }
</script>
```

The parent uses it like this:

```svelte
let amount = $state<number | null>(null);
<MoneyInput bind:cents={amount} label="Amount" name="amount" required />
```

`$bindable()` is the modern Svelte 5 way to declare a two-way bindable prop. The parent's `bind:cents={amount}` creates the binding:
- Parent writes → child sees the new value
- Child writes (via the `cents = parseMoney(text)` line) → parent's `amount` updates

The `text` state is a local typing buffer separate from `cents`. This is intentional: when the user types "12.", `parseMoney` returns `null` (incomplete), but we don't want to clobber their typing. So we keep `text` as-is and only `cents = null` until they finish typing.

On blur, we pretty-print: "12" → "12.00", "12.5" → "12.50". This is the standard "format on commit, not on input" UX. Trying to format on every keystroke makes the cursor jump and breaks the user's flow.

### B.3 — The "payload" serialization pattern

The expense form has lots of state: payer, amount, description, split kind, selected members, and per-member exact/percent values. A traditional `<input name="payer" />`, `<input name="amount" />` ... form would need a dozen named inputs and would lose the structured shape (the shares are an array of objects, not flat fields).

Instead:

```svelte
const payloadJson = $derived(() => {
  const shares = selectedIds.map((id) => ({
    member_id: id,
    value: splitKind === 'exact' ? (exactCents[id] ?? 0)
         : splitKind === 'percent' ? (percentBp[id] ?? 0)
         : 0
  }));
  return JSON.stringify({ payer_id, amount_cents: amount ?? 0, description, split_kind: splitKind, shares });
});

<input type="hidden" name="payload" value={payloadJson()} />
```

The whole form payload is serialized into one hidden field, posted as a single JSON string, parsed server-side. The server's form action validates and forwards to the API.

This is a SvelteKit-idiomatic way to mix "rich client-side form state" with "SvelteKit form actions for the network roundtrip + progressive enhancement". You get the best of both:
- The form **works without JavaScript** (the hidden `payload` field is in the DOM as a string).
- With JS, `use:enhance` intercepts and gives you optimistic UI + error display.

### B.4 — The live preview

```svelte
const preview = $derived(computePreview());
```

`computePreview` runs the same splitter math in TypeScript as the backend's Rust function. Show the user, **before** they submit, exactly what each member would owe. The "Shares total $XX.XX — looks good" message updates in real time.

The button is disabled when `!preview.ok || !payerId`. The user can't even submit invalid data; the server's validation is the second line of defense.

You might object: "we've now implemented the splitter twice". Yes. The cost is a duplicated ~30 lines of arithmetic. The benefit is **instant feedback** without a server round-trip per keystroke. For a personal-finance app, the duplication is the right tradeoff. (For a tax-calculation engine where the math MUST not drift, you'd factor it into a shared WASM module or a single source of truth. Project 28 — AI Inference API metered billing — uses this approach for usage estimation.)

### B.5 — `$derived` for aggregate state

```svelte
const addExpenseError = $derived(
  form && (form as { kind?: string }).kind === 'addExpense' && 'error' in form
    ? (form as { error?: string }).error
    : ''
);
```

The server's form-action return value has a discriminator (`kind: 'addExpense' | 'addMember' | ...`) so the page can distinguish which action returned an error. Without that, an error on the member form would also light up the expense form's error display. The `$derived` filters to the right one.

---

## C. Tests

| Layer | Coverage |
| --- | --- |
| cargo (10 = 7 unit + 3 proptest) | Equal/exact/percent edge cases (sum mismatch, leftover distribution, duplicates, zero amount). Proptests: `equal_always_sums_to_amount`, `equal_max_difference_is_one_cent`, `percent_always_sums_to_amount`. |
| vitest (14 = 7 money + 4 api + 3 etc.) | Money parser/formatter exhaustively, including rejection of 3-decimal-place input. ApiCallError surfaces `.fields` on validation errors. |
| playwright (16 = 4 × 4) | a11y, full lifecycle (seed via API → see balances + settlements), submit-disabled-when-zero, equal-split-preview correctness. |

---

## D. What you can do now

- Encode money correctly in any web app: integer cents from edge to edge, parse/format in one place, never mix with floats.
- Write splitter / proration / apportionment math that sums exactly via the largest-remainder method.
- Verify "this distribution sums correctly for any input" with proptest properties.
- Build reusable form-control components with `$bindable` props.
- Surface field-level validation errors all the way from a SQL constraint to a specific input under the user's cursor.

Open `projects/07-finance/COMMANDS.md` next — personal finance ledger. Double-entry accounting (debits + credits sum to zero per transaction), `rust_decimal`, CSV import, charts drawn directly on `<canvas>`. The last SQLite project before we move to Postgres in project 11.
