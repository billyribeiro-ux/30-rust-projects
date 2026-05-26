# Project 13 — Real-time Chat Rooms

> "Slack is overkill for a 3-person side project. I just want a room I can paste a link to, where the four of us can talk without anyone signing into another SaaS."

A small, authenticated, mobile-first chat. Create a room with a slug, join it
with the link, talk in real time. Scrollback is persisted. Presence is shown.
The connection self-heals after a brief network drop.

This is the first **real-time** project in the curriculum. Until now every
mutation was an HTTP request that returned a response and a `load()` re-ran;
here the server **pushes** to the browser, multiple clients receive the same
event simultaneously, and the UI updates without any HTTP round trip after
the initial page load.

## What's new in project 13

1. **WebSockets via `axum::extract::ws`** — a single endpoint
   (`/api/rooms/{slug}/ws`) upgrades from HTTP, authenticates from the
   session cookie *before* the upgrade, and runs two cooperating tokio tasks
   per connection: one reads frames from the browser, one writes broadcast
   events back.
2. **`tokio::sync::broadcast` as a per-room fan-out hub** — one MPMC channel
   per room, owned by a process-wide `ChatHub`. Every connected client
   subscribes to its room's receiver; sending writes once and the channel
   fans out to every subscriber.
3. **Presence tracking** via a `DashMap<RoomId, DashSet<UserId>>` updated on
   join/disconnect with `Joined`/`Left` events broadcast on the same channel
   as messages.
4. **Message persistence + cursor-paginated scrollback** — the WS path
   writes each message to `messages` *before* broadcasting (so a peer who
   missed it can fetch it from `GET /messages?before=...&limit=50`).
5. **Mobile-first chat layout** — sticky header, scrolling body, sticky
   composer, all in a `100dvh` grid. Tested at 390/768/1024/1440.
6. **Browser-side reconnect with exponential backoff** — `$effect` opens
   the WS on mount, listens to `close`, reconnects with 500ms / 1s / 2s /
   4s / 8s / 10s caps. `prefers-reduced-motion` honoured.

## Stack additions over project 12

- `axum` features = `["macros", "ws"]`
- `futures` for `SinkExt`/`StreamExt` on the WebSocket split
- `tokio` features += `["sync", "time"]`
- `dashmap` for the presence map + channel registry

Removed: bcrypt + the legacy migration column (project 12's lesson is shipped).

## Run it

```bash
cd projects/13-chat
docker compose up -d                       # Postgres + MailHog
cp backend/.env.example backend/.env

cd backend && cargo run                    # http://localhost:3012
cd frontend && pnpm install && pnpm dev    # http://localhost:5185
```

Register two users in two browser windows, create a room with the first,
share the URL with the second, click Join, watch messages flow in real time
in both windows.

## What you'll learn

[LESSON.md](./LESSON.md) walks the WS handler line by line — the
upgrade-with-auth shape, the two-tasks-with-`tokio::select` pattern, why
the broadcast channel has bounded capacity, and what
`RecvError::Lagged` means for your slowest client. [COMMANDS.md](./COMMANDS.md)
is the copy-paste script from scratch.

## Status

Shipped 2026-05-26. All 36 Playwright tests pass (9 × 4 viewports), including
a full end-to-end WebSocket round trip via the browser. Backend: 7 unit
tests, clippy clean.
