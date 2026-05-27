# Project 14 — Calendar & Scheduler

> "My partner and I keep double-booking. We need a real calendar with timezone awareness, recurring events, and the ability to share with each other — without paying $7/mo each for Google Workspace."

A two-person-scale calendar app. Create calendars (each with its own color
and default IANA timezone), add events one-off or recurring (full RFC 5545
RRULE), share read or edit access with another user by email, view a
month grid, subscribe via ICS to any calendar app.

## What's new in project 14

1. **RRULE recurrence** (RFC 5545) via the `rrule` crate. Recurring events
   store one master row with a single RRULE string; range queries expand
   occurrences server-side at request time inside the user's window.
2. **Timezone-aware events** with `chrono-tz`. Events store `start_at`/
   `end_at` as TIMESTAMPTZ (UTC on the wire) plus a `tz` field that
   captures the user's authoring timezone. RRULE expansion honours the
   author's timezone so DST shifts land correctly.
3. **Shared calendars with row-level permissions** — `calendar_shares`
   table with `view`/`edit` permissions. Every read/write query joins
   the permission lattice so the handler never writes
   `if owner || share.edit`.
4. **Cursor-free range queries** with a 90-day cap to bound RRULE
   expansion cost — a malicious `from=1900&to=2100` would otherwise burn
   a worker.
5. **Calendar-app subscribable ICS export** at
   `/api/export/calendar/{id}` — emits the RRULE verbatim so the client
   does the expansion locally (no server-side fan-out per occurrence).

## Stack additions over project 13

- `chrono-tz` for IANA timezone resolution
- `rrule = "0.14"` for RFC 5545 recurrence
- Dropped: WebSockets, DashMap, futures (project 13's lesson is shipped)

## Run it

```bash
cd projects/14-calendar
docker compose up -d                       # Postgres + MailHog
cp backend/.env.example backend/.env

cd backend && cargo run                    # http://localhost:3013
cd frontend && pnpm install && pnpm dev    # http://localhost:5186
```

Register two accounts. With user A, create a calendar and add a weekly
recurring event using `RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR`. Share with
user B's email at `edit` permission. User B sees the events on their
month view and can add their own.

## What you'll learn

- The "store master + expand at read time" pattern for recurrences. Why
  this beats materialising every occurrence as its own row.
- Timezone handling: why UTC on the wire AND `tz` in the row is the right
  fix, vs. picking one or the other.
- Row-level permissions as a SQL discipline, not as `if` statements in
  handlers.
- The axum 0.8 path-segment constraint ("`/{id}.ics` is invalid") and
  what to do instead.

Read [LESSON.md](./LESSON.md) for the line-by-line walkthrough.

## Status

Shipped 2026-05-26. All 40 Playwright tests pass (10 × 4 viewports).
Backend: 7 unit tests, clippy clean. Live verification:
8-occurrence weekly RRULE → 4 occurrences correctly windowed inside a
4-week range, ICS export produces a valid VCALENDAR Google Calendar can
subscribe to.
