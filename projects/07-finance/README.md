# Project 07 — Personal Finance Ledger

> "I don't know where my money goes."

A double-entry personal finance ledger. Add accounts (asset / liability / income / expense / equity), log transactions as balanced postings, import bank CSV exports, see top expenses as a hand-drawn bar chart.

Seventh of 30 projects, and the **last SQLite project**. Project 08 reshapes the same patterns onto Postgres + Docker Compose.

New lessons:

- **Double-entry accounting** — every transaction has 2+ postings; sum across postings must be exactly zero. The pure validator + proptest invariants live in `src/ledger.rs`. The schema doesn't enforce balance (SQLite can't sum across rows in a CHECK), so the application + tests are the guard.
- **`rust_decimal`** — for parsing dollar amounts from user input / CSV without going through floats. The `decimal_to_minor` helper rejects 3-decimal-place input.
- **CSV import** — `csv` crate + transactional bulk insert. Errors per-row are collected and returned to the user along with success counts.
- **Canvas-drawn bar chart** — ~80 lines of TypeScript that gives you a pixel-perfect chart with DPR scaling, rounded bar tops, and label truncation. No chart-library dependency.
- **`ResizeObserver` + `$effect`** — the chart redraws whenever its container width changes.

## Stack

Same as previous, plus `rust_decimal`, `csv` (backend). No new frontend deps.

## Quality gates

- `cargo test` → **10/10** (8 unit + 2 proptest: `balanced_always_validates`, `unbalanced_never_validates`)
- `pnpm check` → 0/0
- `pnpm test:unit` → **3/3**
- `pnpm test:e2e` → **20/20** (5 specs × 4 viewports)
- svelte-autofixer clean

## What's next

Project 08 — **Recipe Book with Photos** — last SQLite project, first file uploads (multipart, mime-sniffed, EXIF stripped), first service worker, recipe JSON-LD.
