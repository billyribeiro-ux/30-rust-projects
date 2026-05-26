# Project 06 — Expense Splitter

> "Roommates can't track who owes who."

A tiny household expense splitter. Add members, log who paid what, split equally / exactly / by percentages. The Balances panel computes net positions and proposes minimum-transaction settlements.

Sixth of 30 projects. New lessons:

- **Money as `i64` cents** (never floats) — both wire format and storage. Frontend parses string→cents in one place, formats cents→string in one place. The `0.1 + 0.2 !== 0.3` problem cannot occur.
- **Pure splitter** with proptest invariants — equal-divides-remainder, percent-largest-remainder method. Properties verify "shares sum to total" for thousands of random inputs.
- **Field-level errors** — `AppError::Fields(Vec<FieldError>)` returns a JSON array under `error.fields` that the client can map back to specific form inputs.
- **`$bindable` MoneyInput** — a reusable component the parent binds to with `bind:cents={...}`.
- **`$state.snapshot`-style payload serialization** — the expense form serializes its derived state into a single hidden JSON payload field for the form action.
- **Greedy settlement algorithm** — pair the largest creditor with the largest debtor, repeat. At most N-1 transactions for N members.

## Stack

Same as previous projects, plus `proptest` (backend, dev-dep). Wire format: integer cents in JSON.

## Quality gates

- `cargo test` → **10/10** (7 unit + 3 proptest, each generating 1024 random inputs)
- `pnpm check` → 0/0
- `pnpm test:unit` → **14/14** (7 money + 4 api + 3 etc.)
- `pnpm test:e2e` → **16/16** (4 specs × 4 viewports)
- svelte-autofixer clean

## What to read next

`LESSON.md` walks through:
- Why never floats for money + the parseMoney/formatCents centralization pattern
- The three split kinds and why "largest remainder method" makes percent-splits sum exactly
- The `$bindable` rune and the parent's `bind:cents={...}` syntax
- The "field-level error" pattern (server FieldError → client `.fields` on ApiCallError)
- The greedy settlement algorithm and why it's optimal-enough for personal use
