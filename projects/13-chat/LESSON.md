# Project 13 — Lesson

Headline lessons:

1. **WebSockets via `axum::extract::ws`** — authenticate BEFORE upgrade, split
   into two cooperating tasks, tear down deterministically.
2. **`tokio::sync::broadcast` as a per-room fan-out hub** — one channel per
   room, every member subscribes; sending writes once and the channel fans
   out.
3. **Presence tracking** on join/disconnect — broadcast `Joined`/`Left`
   events on the same channel so clients update their roster live.
4. **Persistence + scrollback** — DB write before broadcast; an HTTP cursor
   endpoint for "give me messages older than this timestamp."
5. **Mobile-first chat UI** with browser-side reconnect and a `100dvh` grid.

Assumed knowledge (taught earlier): Argon2id + opaque session cookies (11),
hooks.server.ts cookie forwarding (11), `(auth)/(app)` route groups (11),
`AppError → IntoResponse` (1, refined 11), per-user 404-not-403 scoping (11),
form actions vs remote functions (8, 11), Postgres FK cascades (11).

---

## A. Backend

### A.1 — Schema is small on purpose

`backend/migrations/0001_init.sql` keeps the project-11 auth tables (we strip
the project-12 legacy_bcrypt_hash column; project 12's lesson is shipped). The
domain is three tables:

```sql
CREATE TABLE rooms (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9][a-z0-9-]{0,63}$'),
    name        TEXT NOT NULL,
    created_by  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE room_members (
    room_id   UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (room_id, user_id)
);

CREATE TABLE messages (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id    UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body       TEXT NOT NULL CHECK (length(body) BETWEEN 1 AND 2000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_messages_room_time ON messages (room_id, created_at DESC);
```

**What** — `rooms` has a slug regex enforced by a CHECK constraint. `room_members` is the join table (composite primary key prevents duplicate memberships without an extra unique index). `messages` has a length CHECK on the body (1..=2000 chars) and a compound `(room_id, created_at DESC)` index that perfectly matches the scrollback query.

**Why the slug regex in SQL** — Application code validates first (cleaner 400 messages), but the DB owns the *invariant*. If a future migration or import bypasses our handler, the constraint still holds. Same reasoning as the `CHECK (email = lower(email))` in users from project 11.

**Why no `messages.body TEXT NOT NULL DEFAULT ''`** — A zero-length body is meaningless for a chat message. The CHECK constraint forbids it, and the handler trims+rejects empty/whitespace input. Validation at the boundary (Rust) is the user-facing contract; validation at the storage layer (Postgres) is the safety net.

### A.2 — The `ChatHub` shared state

`backend/src/state.rs` introduces the process-wide chat hub:

```rust
pub struct ChatHub {
    pub channels: DashMap<Uuid, broadcast::Sender<RoomEvent>>,
    pub presence: DashMap<Uuid, DashSet<Uuid>>, // room_id → set<user_id>
}

impl ChatHub {
    pub fn sender(&self, room_id: Uuid) -> broadcast::Sender<RoomEvent> {
        self.channels
            .entry(room_id)
            .or_insert_with(|| broadcast::channel(256).0)
            .clone()
    }
}
```

`AppState` holds an `Arc<ChatHub>`. Two facts deserve attention:

**What is `tokio::sync::broadcast`** — A multi-producer, multi-consumer channel where every receiver sees every message. Compare:

| Channel              | producers | consumers | semantics                          |
|----------------------|-----------|-----------|------------------------------------|
| `mpsc`               | many      | one       | each msg goes to exactly one consumer |
| `broadcast`          | many      | many      | every receiver sees every msg      |
| `watch`              | many      | many      | each receiver sees the *latest* msg only |

For chat, every connected member should see every message. That's `broadcast` exactly. The integer `256` is the channel's buffer — see §A.4 (lag) for what happens when a slow consumer falls behind.

**Why `DashMap` instead of `Arc<RwLock<HashMap<...>>>`** — DashMap is a sharded concurrent map. Each shard has its own lock; readers contend only with writers on the same shard. For a hub where 10 rooms get traffic and 1000 idle ones don't, this is much better than one giant `RwLock`. The reads in our hot path (subscribe on connect, broadcast on every message) never block each other.

### A.3 — The WebSocket route, step by step

`backend/src/routes/ws.rs` defines `GET /api/rooms/{slug}/ws`. The handler signature is the centerpiece:

```rust
async fn ws_handler(
    State(s): State<AppState>,
    user: AuthUser,            // extractor runs BEFORE the upgrade
    Path(slug): Path<String>,
    ws: WebSocketUpgrade,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query!(
        r#"SELECT r.id, r.slug, u.name AS author
           FROM rooms r
           JOIN room_members m ON m.room_id = r.id
           JOIN users u ON u.id = $2
           WHERE r.slug = $1 AND m.user_id = $2"#,
        slug, user.id,
    )
    .fetch_optional(&s.pool).await?.ok_or(AppError::NotFound)?;

    Ok(ws.on_upgrade(move |socket| handle(socket, s, row.id, user.id, row.author)))
}
```

**Why auth happens before the upgrade** — The `AuthUser` extractor runs as part of the HTTP request, BEFORE axum tries to do the WebSocket handshake. If authentication fails, the response is a normal HTTP 401 — the client never gets to the protocol switch. If we did it after the upgrade (inside `handle`), we'd accept the socket, then close it, which is wasteful and confusing.

**Why the membership check uses a JOIN** — Two SQL queries (one for the room, one for the membership) would be both slower and racier. A single SELECT with a JOIN is one round trip and atomic-ish (the rows are read at one snapshot). The `ok_or(NotFound)` collapses "no such room" and "not a member" into the same 404 — we don't leak existence.

Inside `handle()`, the two-tasks shape:

```rust
let (mut ws_tx, mut ws_rx) = socket.split();
let mut rx = state.hub.sender(room_id).subscribe();

let send_task = tokio::spawn(async move {
    while let Ok(ev) = rx.recv().await {
        let json = serde_json::to_string(&ev).unwrap();
        if ws_tx.send(WsMessage::Text(json.into())).await.is_err() { break; }
    }
});

let recv_task = tokio::spawn(async move {
    while let Some(Ok(msg)) = ws_rx.next().await {
        // parse ClientMsg::Send, INSERT into messages, broadcast a RoomEvent
    }
});

tokio::select! {
    _ = send_task => {}
    _ = recv_task => {}
}
```

**Why two tasks** — `WebSocket` implements both `Stream` (for receiving) and `Sink` (for sending). We split it so each direction can run independently in its own tokio task. The browser can fire 50 messages while we're mid-send of a backlog of broadcasts, and neither side starves the other.

**Why `tokio::select!`** — Either side closing means the connection is gone. The `select!` returns as soon as one task finishes, and the other is automatically aborted when the runtime drops the task handle. Without it, you'd have to either poll both halves (wasteful) or pick one to wait on (deadlocky if the *other* one closed).

**Why `serde_json::to_string` inside the loop and not pre-serialized in the broadcast event** — Two reasons. (1) Most rooms are small; the cost of one `to_string` per connected client is microseconds. (2) Pre-serializing means the broadcast item is `String`/`Bytes`, and every receiver gets the same buffer — which is a small memory win that we trade for the flexibility of `RoomEvent` being a typed Rust enum the test code can match on. The trade is local; revisit if a room exceeds ~100 active sockets.

### A.4 — Lag handling (the slow-consumer problem)

`broadcast::Sender::subscribe()` returns a `broadcast::Receiver`. If a receiver doesn't call `recv()` fast enough, the channel's ring buffer (256 slots in our case) wraps, and the next `recv()` returns `Err(RecvError::Lagged(skipped))`.

We chose to **drop the connection** on lag:

```rust
while let Ok(ev) = rx.recv().await {
    // forward to socket
}
// when rx.recv() returns Err (Lagged or Closed), the loop exits and the task ends
```

**Why drop rather than skip-and-continue** — A lagged client is, by definition, behind. Forwarding "current" messages without the lost ones means the user sees a gap they don't know about. Dropping the WS forces the client to reconnect and call `GET /messages?before=...` to fill the gap from the DB. Persistence is the source of truth; the broadcast is only the *push channel*.

**The capacity number (256)** — On a chat room with 1000 messages/sec (which would be wild), 256 buffers ~250ms of lag tolerance. For our use cases — a 4-person team room — 256 is silly-large. Default-tune it once you measure.

### A.5 — Why we write to the DB BEFORE broadcasting

```rust
let rec = sqlx::query!(
    r#"INSERT INTO messages (room_id, user_id, body)
       VALUES ($1, $2, $3)
       RETURNING id, created_at"#,
    room_id, user_id, body,
).fetch_one(&recv_pool).await;

if let Ok(rec) = rec {
    let _ = recv_tx.send(RoomEvent::Message { id: rec.id, ..., created_at: rec.created_at });
}
```

**What** — The INSERT is awaited, returning the new `id` and server-assigned `created_at`, BEFORE we put the event on the broadcast channel.

**Why** — Three benefits:
1. The broadcast carries the **real** id and timestamp, not client-supplied or made-up values. Every subscriber gets the same source-of-truth row.
2. If the INSERT fails (CHECK constraint violation, FK race), no broadcast happens — peers don't see a phantom message they can't `GET` later.
3. A subscriber who missed the broadcast can fetch the row from `GET /messages` because it's already committed.

**What would break otherwise** — Broadcast-first-write-later means peers see a message that may never persist (worst: phantom). Or write-and-broadcast in parallel: subscribers might see the message before the row commits, leading to scrollback that "doesn't have" the message they just saw.

### A.6 — The scrollback HTTP endpoint

`backend/src/routes/messages.rs` is a cursor-paginated GET, not a WS frame. The lesson is that the WebSocket is **only** for live events; bulk historical reads are HTTP.

```sql
SELECT m.id, m.room_id, m.user_id, u.name AS author, m.body, m.created_at
FROM messages m
JOIN users u ON u.id = m.user_id
WHERE m.room_id = $1 AND m.created_at < $2
ORDER BY m.created_at DESC
LIMIT $3
```

Cursor is `before=<ISO timestamp>`. Default `before = now() + 1 day` so a fresh page gets the most recent 50. To load older, the browser passes the `created_at` of its currently-oldest rendered message and gets the next 50 older than that. **What** — keyset pagination over an indexed column. **Why** — OFFSET-based pagination is O(N) on the offset; keyset is O(log N) regardless of depth.

### A.7 — Wiring it all in `main.rs`

```rust
let state = AppState {
    pool,
    mailer: Arc::new(mailer),
    public_url,
    secure_cookies,
    hub: Arc::new(ChatHub::new()),
};

let app = Router::new()
    .nest("/api/auth", routes::auth::router())
    .nest("/api/rooms", routes::rooms::router())
    .nest("/api/rooms", routes::messages::router())
    .nest("/api/rooms", routes::ws::router())
    .route("/healthz", get(health))
    .with_state(state)
    ...
```

Three `nest("/api/rooms", ...)` calls compose: each adds its own routes
under that prefix. `rooms::router()` defines `/`, `/{slug}`,
`/{slug}/join`, `/{slug}/leave`; `messages::router()` adds
`/{slug}/messages`; `ws::router()` adds `/{slug}/ws`. Axum merges them
without conflict because each sub-router contributes a different leaf.

---

## B. Frontend

### B.1 — Types mirror the wire enum

```ts
export type RoomEvent =
  | (Message & { type: 'message' })
  | { type: 'joined'; user_id: string; author: string }
  | { type: 'left'; user_id: string; author: string };

export type ClientMsg = { type: 'send'; body: string };
```

The Rust backend serializes the enum as `{"type": "message", ...}` /
`{"type": "joined", ...}` via `#[serde(tag = "type", rename_all = "snake_case")]`.
The TypeScript discriminated union mirrors that exactly. The `switch (ev.type)`
in the page narrows the union at each branch — TypeScript flags any missed
case at build time.

### B.2 — The chat page lifecycle

`frontend/src/routes/(app)/rooms/[slug]/+page.svelte` is where every new
runtime concept comes together: a WebSocket opened in `$effect`, message
state in `$state`, presence as a `Set`, an auto-scroller, and a composer.

```svelte
let messages = $state<Message[]>([]);
let presence = $state<Set<string>>(new Set());
let connected = $state(false);
let composer = $state('');

$effect(() => {
  messages = [...data.initialMessages]; // reset when slug changes
  tick().then(scrollToBottom);
});

$effect(() => {
  connect();
  return () => ws?.close();
});
```

**Two `$effect`s, deliberately** — One reacts to `data` (re-seed the
scrollback when navigating between rooms), one runs on mount and tears
down on unmount (open + close the WS). Combining them into a single
effect would couple "data changed" to "reconnect," which we don't want —
the slug *is* in the WS URL, so reconnection happens via the cleanup +
re-run cycle automatically when `wsUrl(data.room.slug)` returns a
different string.

**Why `$state<Set<string>>` instead of `$state.raw`** — Svelte 5
auto-proxies `$state` for arrays/objects, but `Set` and `Map` aren't
deeply tracked by the proxy. We re-assign with `new Set([...])` on every
add/remove — that's what triggers reactivity. The `svelte/reactivity`
package has a `SvelteSet` we could use instead; that's introduced in
project 14 (Kanban) where set membership churns more.

### B.3 — Reconnect with exponential backoff

```ts
let backoff = 500;
function connect() {
  ws = new WebSocket(wsUrl(data.room.slug));
  ws.addEventListener('open', () => { connected = true; backoff = 500; });
  ws.addEventListener('close', () => {
    connected = false;
    ws = null;
    reconnectTimer = setTimeout(connect, backoff);
    backoff = Math.min(backoff * 2, 10_000);
  });
}
```

**What** — On close, schedule a reconnect at the current backoff, then
double the backoff (capped at 10s). On successful open, reset backoff to
500ms.

**Why exponential rather than linear** — Linear retries (every 1s
forever) hammer a struggling backend exactly when it's least able to
respond. Exponential with a cap gives a slow server breathing room
without making the user wait minutes on a transient blip.

**What would break otherwise** — No reconnect at all: a single
WS-server restart kills every chat session. Always-instant reconnect:
a network glitch turns into a 1000-req/s burst per client.

### B.4 — Browser-side composer, with Enter-to-send

```svelte
<textarea
  bind:value={composer}
  onkeydown={onKey}
  rows="1"
  placeholder="Message #{data.room.slug}…"
  maxlength="2000"
  disabled={!connected}
></textarea>

function onKey(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    send();
  }
}
```

**The `Enter` vs `Shift+Enter` convention** — Plain Enter sends; Shift+Enter
inserts a newline. This is the Slack/Discord muscle memory. Anything else
will annoy the user the moment they want a multi-line message.

**The `disabled={!connected}` discipline** — A user typing into a textarea
that the page can't send creates the worst kind of dataloss: the user thinks
they sent it. Disabling the input until the WS confirms it's open is the
cheapest form of "respect the user's time."

### B.5 — Mobile-first layout in 100dvh

```css
.room {
  display: grid;
  grid-template-rows: auto 1fr auto;
  height: calc(100dvh - var(--topnav-height, 56px));
}
```

**Why `dvh` not `vh`** — On mobile, `100vh` includes the browser chrome
that *might* hide on scroll. The result is that fixed-bottom composers
either get hidden behind the tab bar or jump when the chrome animates.
`100dvh` is the *dynamic* viewport height: it accounts for chrome
animations, and the layout doesn't jump.

**Why `grid-template-rows: auto 1fr auto`** — Three rows: head (auto-sized
to its content), scroller (takes all remaining space — `1fr`), composer
(auto-sized). The scroller is the only thing that gets `overflow-y: auto`,
which means the composer stays planted at the bottom and the head stays at
the top regardless of how many messages there are.

The `@media (max-width: 480px)` block hides the room "name" subtitle on
narrow screens to keep the header readable. Real mobile-first work
isn't "make it stack at small widths" — it's "decide what's essential
and what's surface."

### B.6 — Live announce via `aria-live`

```svelte
<div class="scroller" bind:this={scroller} role="log" aria-live="polite">
  ...
</div>
<div class="status" aria-live="polite">
  <span class="dot"></span>
  <span>{connected ? 'Live' : 'Reconnecting…'}</span>
</div>
```

**What** — `role="log"` plus `aria-live="polite"` tells screen readers
"this region updates over time; announce changes when the user pauses."
The connection status pill is also `aria-live` so a user with vision
loss hears "Reconnecting" exactly when a sighted user sees the dot pulse.

**Why two `polite` regions don't conflict** — Multiple `polite` regions
coexist fine. `assertive` (interrupts) is the one to avoid stacking;
`polite` queues.

### B.7 — Playwright E2E end-to-end including WebSocket

The `WebSocket round-trip` test is the headline:

```ts
test('WebSocket round-trip: send a message and see it echo', async ({ page }) => {
  // ... login, create room, navigate to /rooms/{slug}
  await expect(page.getByText(/^live$/i)).toBeVisible({ timeout: 10_000 });
  await page.getByPlaceholder(`Message #${slug}…`).fill('hello over the wire');
  await page.getByRole('button', { name: /^send$/i }).click();
  await expect(page.getByText('hello over the wire')).toBeVisible({ timeout: 5_000 });
});
```

**What this proves** — The status pill flipping to "Live" means the WS
opened. The visible message text after Send means the round trip
completed: browser → WS → DB → broadcast → WS → DOM. If any of those
steps breaks (the auth check, the upgrade, the INSERT, the channel
send/recv, the JSON parse, the reactive append) the test fails.

**Why a 10s timeout for "Live"** — The Playwright preview server boots
the app in `production` mode, which can take a beat. The 10s is generous;
in practice it lands in well under a second.

---

## C. Patterns to carry forward

- **Auth before upgrade** is the universal shape for any WebSocket/SSE
  endpoint. Run the extractor first; only call `on_upgrade` if it
  succeeds.
- **`broadcast` + `DashMap`** is the simplest in-process pub-sub. Project
  21 (live polls) reuses this pattern, scaled to thousands of subscribers
  per channel.
- **Write-then-broadcast** is the ordering for any "this needs to be
  durable AND live." Project 23 (job queue) writes a job row, then
  notifies. Project 28 (analytics dashboard) writes a metric, then
  pushes.
- **Cursor pagination over indexed `created_at DESC`** is the workhorse
  for any timeline. Project 21, 24, 28 all reuse this exact shape.
- **`100dvh` grid layout + sticky composer** is the right answer for any
  product that has a chat-like interaction. Project 24 (help desk
  conversations) and project 27 (live coding) both inherit it.

### Try it (exercises)

1. Add a **typing indicator**: extend `ClientMsg` with a `Typing { is_typing: bool }`
   variant and broadcast a `RoomEvent::Typing` so other clients can render
   "Alice is typing…". The body never persists.
2. Implement **scroll-up-to-load-older**: detect when the scroller hits
   the top, call `messagesApi.list(f, slug, { before: oldestRendered })`,
   and prepend the results without scroll-jump (capture
   `scroller.scrollHeight` before, restore after).
3. **Per-room presence list**: render the names of currently-connected
   users by tracking the join/left events. Hint: a `Map<user_id, name>`
   updated on each event, derived to a sorted name list.
4. **WebSocket auth without cookies**: switch to a one-shot ticket
   exchange — POST `/api/rooms/{slug}/ws-ticket` returns a short-lived
   token, the browser passes it as `?ticket=` to the WS URL, the server
   exchanges it for a session. (Useful when the WS endpoint lives on a
   different origin and the cookie won't go.)

---

Project 13 is shipped when:

- [x] `cargo fmt --check` clean
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `cargo test` — 7/7 passing
- [x] `pnpm check` — 0 errors / 0 warnings
- [x] `pnpm build` — clean
- [x] `pnpm test:e2e` — 36/36 (9 tests × 4 viewports), including the
  full browser WebSocket round trip
- [x] Live smoke: open two browser windows, see each other's messages
  appear in real time
