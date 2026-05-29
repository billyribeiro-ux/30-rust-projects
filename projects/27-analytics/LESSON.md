# Lesson — Project 27 (Realtime Analytics, GSAP #2)

Three things this project teaches that no earlier project did:

1. **Background broadcaster pattern** — one tokio task computes,
   `broadcast::Sender` fans out, every SSE subscriber gets the same
   payload. No N×M queries.
2. **GSAP number-roll with lazy GSAP** — cinematic motion that doesn't
   ship to the SSR.
3. **Three-tier ingest → aggregate → push** as orthogonal components,
   each replaceable.

---

## A. Backend

### A1. The aggregation

```sql
SELECT
  (SELECT COUNT(*)::bigint FROM events WHERE ts > now() - INTERVAL '1 minute') AS events_last_minute,
  (SELECT COUNT(*)::bigint FROM events WHERE ts > now() - INTERVAL '1 hour')   AS events_last_hour,
  (SELECT COUNT(DISTINCT session_id)::bigint FROM events
        WHERE ts > now() - INTERVAL '1 hour' AND session_id IS NOT NULL)         AS unique_sessions_last_hour
```

Three scalar subqueries in one round-trip. Each hits
`idx_events_ts` directly. At ~1M events / hour the query runs in
single-digit milliseconds — fine for a 1-second tick.

When the table outgrows the index (>10M events), the upgrade path is:

1. **Partition events by day** (`PARTITION BY RANGE (ts)`). The `WHERE
   ts > now() - INTERVAL '1 hour'` prunes to one or two partitions.
2. **Materialise the aggregate** every minute via a separate
   `kpi_buckets` table.
3. **Move to DuckDB / ClickHouse** for ad-hoc OLAP. Postgres remains
   OLTP for the ingest path.

LESSON COMMANDS.md §7 covers (3).

### A2. The broadcaster loop

```rust
pub fn spawn_kpi_loop(state: AppState, interval: Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            match compute_snapshot(&state.pool).await {
                Ok(snap) => {
                    if let Ok(json) = serde_json::to_string(&snap) {
                        let _ = state.kpis_tx.send(json);
                    }
                }
                Err(e) => tracing::warn!(?e, "kpi snapshot failed"),
            }
        }
    });
}
```

- **One task computes** instead of N (where N is the number of SSE
  subscribers). The dashboard query cost is independent of viewer
  count.
- **`broadcast::send` returns `Err` when there are zero receivers** —
  we ignore it. The loop keeps ticking; new viewers get the next snap.
- **Errors don't abort the loop.** A transient query failure logs a
  warning and we try again next tick.

### A3. The SSE handler

```rust
async fn stream(State(s): State<AppState>, _u: AuthUser)
    -> Sse<impl Stream<Item = Result<Event, Infallible>>>
{
    let rx = s.kpis_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|r| match r {
        Ok(json) => Some(Ok(Event::default().event("kpis").data(json))),
        Err(_) => Some(Ok(Event::default().event("lag").data("0"))),
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
```

- **`BroadcastStream`** adapts the broadcast receiver into a `Stream`.
- **`event("kpis")`** lets the client listen with
  `es.addEventListener('kpis', …)` so we can multiplex event types
  on the same connection.
- **`KeepAlive::default()`** sends periodic comments through the
  connection so corporate proxies don't time it out at 60s.
- **`Err` from the receiver = lag.** We emit `event: lag` so the
  client can `refetch()` if it cares about strict consistency.

---

## B. Frontend

### B1. The number-roll

```ts
export async function rollNumber(el: HTMLElement, from: number, to: number, ms = 600) {
  if (prefersReducedMotion() || ms <= 0) {
    el.textContent = formatInteger(to);
    return;
  }
  const { gsap } = await loadGsap();
  const obj = { v: from };
  gsap.killTweensOf(obj);
  await new Promise<void>((resolve) =>
    gsap.to(obj, {
      v: to, duration: ms / 1000, ease: 'power3.out',
      onUpdate() { el.textContent = formatInteger(obj.v); },
      onComplete() { el.textContent = formatInteger(to); resolve(); },
    })
  );
}
```

What's load-bearing:

- **`prefersReducedMotion()`** sets the textContent instantly. No
  GSAP. No `import('gsap')`. Users who said "no motion" never pay the
  cost.
- **`gsap.killTweensOf(obj)`** before starting prevents stacking
  animations when KPIs change faster than `ms`.
- **`onComplete` forces the final value.** GSAP's final tick is at
  `ms - epsilon`, so the displayed value lands one frame short.
- **`obj.v` is a plain number**, not the DOM node. GSAP doesn't have
  to touch CSS until `onUpdate`. Cheaper than tweening `textContent`
  directly.

### B2. The element-ref pattern

```svelte
<p class="value" bind:this={elMin}>{kpis.events_last_minute.toLocaleString()}</p>
```

Then in the effect:

```ts
if (elMin) rollNumber(elMin, before.events_last_minute, next.events_last_minute);
```

We bind the DOM node and let GSAP write `textContent` directly. The
Svelte reactive cycle is bypassed for the animation frames — that's
the right call because the user only sees the *final* value as
"state", the intermediates are pure motion.

### B3. The `state_referenced_locally` silences

The page seeds `prev` from `data.kpis.*`. Svelte warns. We silence
with comments — the pattern is intentional ("seed once, then update
from SSE").

---

## C. Tests

### C1. `cargo test` (3)

- `auth::hash::tests::round_trip` (Argon2)
- `kpi_flow::ingest_then_snapshot_reflects_counts` — POST 7 events,
  verify aggregate is right.
- `kpi_flow::ingest_rejects_empty_kind` — validation works.

### C2. `pnpm test:unit` (4)

`kpisApi.snapshot/streamUrl`, `ingestApi.send` body shape, non-2xx →
ApiCallError.

### C3. `pnpm test:e2e` (3 × 4 = 12)

Unauthenticated redirect, login axe-clean, ingest endpoint returns 204.

---

## D. What you can do now

1. Stand up a live KPI dashboard with one query, one broadcaster, one
   SSE route.
2. Animate numbers with GSAP without inflating the SSR bundle.
3. Reason about the upgrade path to a real OLAP store.

Project 28 — AI Inference API. WebAuthn passkeys + Stripe metered billing.
