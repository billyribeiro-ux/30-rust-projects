# Lesson — Project 22 (Background Jobs Dashboard)

Three things this project teaches that no earlier project did:

1. **`FOR UPDATE SKIP LOCKED`** — why this single SQL clause is what
   lets you skip Redis/SQS for most workloads.
2. **Lease + retry + backoff + dead-letter** as one cohesive state
   machine, not four separate fixes.
3. **SSE + invalidateAll()** as the simplest "live dashboard" pattern
   in SvelteKit. No WebSockets, no polling.

---

## A. Backend

### A1. The hot path

```sql
WITH claimed AS (
    SELECT id
    FROM jobs
    WHERE queue = $1
      AND status = 'pending'
      AND run_at <= now()
    ORDER BY run_at
    LIMIT $2
    FOR UPDATE SKIP LOCKED
)
UPDATE jobs
SET status = 'running',
    locked_until = $3,
    locked_by = $4,
    attempts = attempts + 1,
    updated_at = now()
FROM claimed
WHERE jobs.id = claimed.id
RETURNING jobs.*;
```

What's load-bearing:

- **`FOR UPDATE SKIP LOCKED`** — the row-level lock acquired by another
  in-flight worker is *skipped*, not waited on. Without this, two
  workers contending for the queue serialise — throughput is 1, not N.
- **CTE + `FROM claimed`** — we SELECT-then-UPDATE in one statement.
  Doing it in two means the row could be locked, claimed, marked
  succeeded, and deleted between our SELECT and our UPDATE.
- **`attempts = attempts + 1`** — we increment *on claim*, not on
  failure. If a worker crashes mid-execution we don't lose the
  attempt count.
- **`ORDER BY run_at`** is the FIFO of the queue. Combined with the
  partial index `WHERE status = 'pending'` it's an index scan, not a
  heap scan.

### A2. The index

```sql
CREATE INDEX idx_jobs_pending_run_at
    ON jobs (status, run_at)
    WHERE status = 'pending';
```

A partial index — only `pending` rows are in it. As jobs flow through
`pending → running → succeeded`, they leave the index. So the queue
size in the index is bounded by *pending depth*, not historical depth.
At a million-row history with 50 pending, the queue's planning cost is
the cost of 50 rows, not 1M.

### A3. The retry / dead-letter dispatcher

```rust
pub async fn mark_failed_or_retry(
    pool: &PgPool,
    id: Uuid,
    err: &str,
    next_run_at: DateTime<Utc>,
) -> AppResult<bool> {
    let row = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = CASE
                       WHEN attempts >= max_attempts THEN 'dead'
                       ELSE 'pending'
                     END,
            run_at = CASE
                       WHEN attempts >= max_attempts THEN run_at
                       ELSE $2
                     END,
            locked_until = NULL, locked_by = NULL,
            last_error = $3,
            updated_at = now()
        WHERE id = $1
        RETURNING status
        "#,
        id, next_run_at, err
    ).fetch_one(pool).await?;
    record_result(pool, id, false, err).await?;
    Ok(row.status == "dead")
}
```

The `CASE` expressions encode the dead-letter rule in SQL, atomically
with the lease release. A version that branches in app code

```rust
if attempts >= max_attempts {
    /* mark dead */
} else {
    /* schedule retry */
}
```

…has a race where another claim happens in between. Doing it in one
statement makes the rule a property of the database, not the worker.

### A4. The lease — and what we *didn't* ship

`locked_until` is set when we claim. If the worker dies, the next
claim (anywhere) sees `now() > locked_until`… well, would, if our
`claim()` filtered on it. It doesn't.

That's a known omission, documented here, listed in the README as
future work. The full pattern:

```sql
WHERE status = 'running'
  AND locked_until < now()
```

…and a separate "expired-leases reaper" job. Project 30 (the capstone)
ships this.

### A5. Backoff math (proptest-style)

```rust
pub fn delay_secs(&self, attempt: u32) -> f64 {
    let base = self.base_secs * self.factor.powi((attempt.max(1) - 1) as i32);
    let capped = base.min(self.max_secs);
    let jitter_factor = 1.0 + rng.gen_range(-0.25..=0.25);
    (capped * jitter_factor).max(1.0)
}
```

Two unit tests:

- **`schedule_is_monotonic_non_decreasing_in_expectation`** — 1000
  samples per attempt, median is at least the expected floor. Jitter
  shouldn't move the median below 0.75× the no-jitter value.
- **`cap_holds`** — at attempt=10 with factor=10, no-jitter value is
  10⁹ seconds. With cap=50, all samples should be ≤ 50 × 1.26.

These are proptest-style statistical assertions without bringing in
the proptest crate — for a 2-input function it's overkill.

### A6. `tracing::instrument` on the runner

```rust
#[tracing::instrument(level = "info", skip_all, fields(job.id, job.kind, job.attempt))]
async fn run_one(...)
```

When `OTEL_EXPORTER_OTLP_ENDPOINT` is wired to Tempo/Jaeger via
`opentelemetry-otlp` + `tracing-opentelemetry`, every job run produces
a span with these fields as searchable attributes. We don't ship the
exporter binding by default (it adds two heavy deps); documented in
COMMANDS.md as the production checklist.

---

## B. Frontend

### B1. The SSE loop

```ts
$effect(() => {
  const es = new EventSource(streamUrl(), { withCredentials: true });
  es.addEventListener('job', (ev) => {
    const data = JSON.parse((ev as MessageEvent).data) as JobEvent;
    liveEvents = [data, ...liveEvents].slice(0, 50);
    invalidateAll();
  });
  es.addEventListener('lag', () => invalidateAll());
  return () => es.close();
});
```

Three things doing real work:

- **`withCredentials: true`** — `EventSource` doesn't send cookies by
  default, so the auth extractor would 401. With `withCredentials`
  the browser includes the `app_session` cookie on the EventSource
  connection.
- **`invalidateAll()`** — instead of patching local state from the
  event, we ask SvelteKit to re-run the `load()` and replace `data`.
  This means the queues table + the jobs list are always exactly
  what the database says, not a guess from the event stream.
- **`lag` event** — when the backend's broadcast channel overflows
  (a slow client), the receiver gets a Lagged error. The handler
  emits an `event: lag` line; the client interprets it as "you missed
  events, refetch."

### B2. The networkidle trap (why an e2e flaked)

`page.waitForLoadState('networkidle')` waits for ~500ms of network
silence. With an open SSE connection, there's always a TCP-level
heartbeat — so networkidle never fires and the test times out.

Fix:

```ts
await Promise.all([
  page.waitForResponse((r) => r.url().includes('?/enqueue') && r.status() < 400),
  page.getByRole('button', { name: /enqueue/i }).click()
]);
```

Wait on the *specific* response you care about. Documented because
the next person to write an e2e against a live-streaming page will
hit this and the fix is non-obvious.

---

## C. Tests

### C1. `cargo test` (6 tests)

- `backoff::tests::schedule_is_monotonic_non_decreasing_in_expectation`
- `backoff::tests::cap_holds`
- `hash::tests::round_trip_verifies`
- `queue_flow::skip_locked_means_exactly_one_worker_wins` — two parallel
  claims for one pending job: exactly one wins.
- `queue_flow::failed_job_returns_to_pending_with_future_run_at_then_dead_letters`
- `queue_flow::succeeded_job_is_terminal_and_records_history`

### C2. `pnpm test:unit` (5 tests)

- queue list parses, job list serializes filters, enqueue body shape,
  retry URL encoding, non-2xx → ApiCallError.

### C3. `pnpm test:e2e` (3 × 4 = 12 runs)

- redirect to /login when unauthenticated
- /login axe-clean
- signed-in: enqueue a send_email, queues table reflects it.

---

## D. What you can do now

1. Build a worker tier without a separate service.
2. Reason about retries and dead-letters as data, not logic.
3. Wire a live dashboard with three new patterns: SSE + EventSource +
   invalidateAll().

Project 23 — Multi-tenant help desk with Postgres RLS and audit-log
triggers.
