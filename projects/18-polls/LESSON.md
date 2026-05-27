# Project 18 — Lessons

The headline lesson is bridging Postgres `LISTEN`/`NOTIFY` to Server-Sent
Events. Everything else (QR codes, voter-key derivation, Svelte 5 springs)
is supporting cast.

---

## 1. Postgres `LISTEN`/`NOTIFY` -> SSE: the bridge

### The problem

Project 13 (Chat) used `tokio::sync::broadcast` to fan vote-counts out to
WebSocket subscribers. That works **for one backend process**. The moment
you scale to two (a load balancer round-robins), half your votes never
reach half your viewers — each process has a private broadcast channel.

Postgres `LISTEN`/`NOTIFY` flips that: Postgres itself becomes the pub/sub
bus. **All** backend processes subscribe to the same channel; **all**
viewers see every vote regardless of which process serves their SSE socket.

### The mechanics

```rust
// Vote handler — after INSERTing into `votes`:
let counts = compute_counts_json(&pool, &slug).await?;
sqlx::query("SELECT pg_notify($1, $2)")
    .bind(format!("poll_{slug}"))
    .bind(counts.to_string())
    .execute(&pool)
    .await?;
```

```rust
// SSE handler — opens a DEDICATED connection per subscriber:
let mut listener = PgListener::connect(&db_url).await?;
listener.listen(&format!("poll_{slug}")).await?;

let init = futures::stream::once(async move {
    Ok::<_, Infallible>(Event::default().event("counts").data(snapshot))
});
let live = listener.into_stream().filter_map(|n| async move {
    match n {
        Ok(n) => Some(Ok::<_, Infallible>(Event::default().event("counts").data(n.payload()))),
        Err(_) => None,
    }
});

Sse::new(init.chain(live)).keep_alive(
    KeepAlive::new().interval(Duration::from_secs(15)).text("ping"),
)
```

### Why `PgListener::connect(&db_url)` and not `connect_with(&pool)`

`LISTEN` is **per-connection state** in Postgres. If we borrowed a pooled
connection, the LISTEN would die the moment it returned to the pool — and
the next subscriber might get a connection that's listening to *someone
else's* poll. Per-subscriber dedicated connection is the only safe move.

### Why a 15-second keep-alive

Reverse proxies (nginx 60s, Cloudflare 100s, AWS ALB 60s) silently kill
"idle" TCP connections. The `KeepAlive` ticker writes an SSE comment line
(`: ping\n\n`) every 15s — invisible to the browser, but enough TCP
traffic that no proxy considers the connection idle.

### Trade-offs vs. `tokio::sync::broadcast`

| Aspect              | broadcast (project 13)            | LISTEN/NOTIFY (this project)        |
|---------------------|-----------------------------------|-------------------------------------|
| Multi-process?      | No                                | Yes                                 |
| Payload limit       | unbounded (Rust struct)           | 8 KB (Postgres NOTIFY limit)        |
| Connections         | 1 (channel in app memory)         | 1 per subscriber (LISTEN socket)    |
| Latency             | ~microseconds                     | ~1-5 ms (extra round-trip)          |
| Durable?            | No (lost on restart)              | No (NOTIFY is fire-and-forget too)  |

For audience polls — small payload, modest concurrent viewers, durability
not required — LISTEN/NOTIFY is the better choice. For chat with 10k
concurrent users on a single box, broadcast still wins.

---

## 2. `axum::response::sse::Sse<Stream>`

axum 0.8's SSE support is built on `futures::Stream`. The handler returns
`Sse<impl Stream<Item = Result<Event, _>>>`. Each `Event` you yield is one
SSE record on the wire:

```
event: counts
data: {"slug":"q4plan","total":3,"counts":[...]}

```

We yield the **current snapshot** first (so a newly-connected client
doesn't have to wait for the next vote), then chain the live notifications.

```rust
let combined = init_stream.chain(notif_stream);
Sse::new(combined).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
```

---

## 3. One-vote-per-voter without accounts

We can't make people sign up to vote — the whole point is "scan a QR,
tap a button". So how do we stop ballot stuffing?

```rust
voter_key = base64( sha256( slug + "|" + ip + "|" + cookie ) )
```

- `slug` — so the same voter has a **different** key for each poll
  (otherwise the unique index would block their second poll vote).
- `ip` — first line of defense; the IP also hashes so the DB never sees
  raw IPs (some EU jurisdictions consider IPs personal data).
- `cookie` — minted on first visit to a poll. Persists across reloads.

Then a **single unique index** `(option_id, voter_key)` is enough. To
catch "voted A, now voting B in the same poll", we do an extra `EXISTS`
check that joins `options.poll_id`. The index alone wouldn't catch it
because option_ids differ.

### Trade-offs

A vandal with N IPs (a botnet, a corporate NAT minus the cookie) gets N
votes. **This is fine for low-stakes audience polls** — the goal is
"good enough that a casual user can't double-click their way to victory",
not Sybil-resistance. For real audience polls with prize money or
elections, use Mentimeter/Slido (account-gated) or a TOTP per attendee.

---

## 4. Svelte 5 `Spring` class for bar animation

The new Svelte 5 motion API uses **class instances** rather than the old
`spring()` store factory:

```svelte
<script lang="ts">
  import { Spring } from 'svelte/motion';

  const springs = new Map<string, Spring<number>>();
  function widthOf(opt) {
    const target = total > 0 ? (opt.count / total) * 100 : 0;
    let s = springs.get(opt.id);
    if (!s) {
      s = new Spring(target, { stiffness: 0.08, damping: 0.4 });
      springs.set(opt.id, s);
    } else {
      s.target = target;  // setting `.target` triggers the animation
    }
    return s.current;     // reading `.current` subscribes; updates re-render
  }
</script>

{#each counts as opt (opt.id)}
  {@const w = widthOf(opt)}
  <div class="track"><div class="fill" style:width={`${w}%`}></div></div>
{/each}
```

`Spring.current` is reactive — reading it in the template re-renders when
the spring frame ticks. We keep **one spring per option id** so that
when the counts re-order, animations don't reset.

We animate **percentages** (0..100), not absolute counts. That way the
bar doesn't flicker as `total` rises — only the relative balance changes.

---

## 5. QR codes from the `qrcode` crate

```rust
let code = QrCode::new(join_url.as_bytes())?;
let svg = code
    .render::<svg::Color<'_>>()
    .min_dimensions(256, 256)
    .dark_color(svg::Color("#000000"))
    .light_color(svg::Color("#ffffff"))
    .build();
```

We render SVG, not PNG. Why:
- **No image-decoding deps** (PNG path requires the `image` crate +
  encoders — ~2 MB extra binary size).
- **Crisp at any size** — projected QR codes are scaled aggressively;
  vector graphics stay sharp.
- **Smaller payload** — a 256×256 QR SVG is ~5 KB; the equivalent PNG is
  ~2 KB but with the image-crate dep, the binary cost dominates.

We add `Cache-Control: public, max-age=300` so a viewer reloading the
stage page doesn't re-render the QR every time.

---

## What I'd build next

- **Persisting voter cookies cross-poll** via a signed token, so a user
  who closes the tab between two polls in a session doesn't get a fresh
  cookie (and another vote on poll #1).
- **Open/close timer** — auto-close after N minutes from the create form.
- **Reverse-proxy aware**: prefer `Forwarded` header (RFC 7239) over
  `X-Forwarded-For` so we can opt into the more secure form.
- **Multi-select polls** — `votes.option_id` is already per-vote, so
  removing the `(option_id, voter_key)` uniqueness and adding
  `(poll_id, voter_key, option_id)` would let voters pick more than one.
