# Project 13 — Commands

Ports: backend `3012`, frontend dev `5185`, Playwright preview `4185`. Postgres on `5432`, MailHog SMTP on `1025`, MailHog web UI on `8025`.

## 1. Local infra

```bash
cd projects/13-chat
docker compose up -d                # Postgres + MailHog
```

Postgres comes up as `chat` / `chat_dev_password` on database `chat`.

## 2. Backend bootstrap

```bash
cd backend
cp .env.example .env
cargo run                           # listens on :3012
```

```bash
curl http://localhost:3012/healthz                              # → ok
curl -i http://localhost:3012/api/auth/me                       # → 401 (no cookie)
```

## 3. Frontend bootstrap

```bash
cd frontend
pnpm install
pnpm dev                            # http://localhost:5185
```

Open <http://localhost:5185/register> in two browser profiles (or one window
plus an incognito), create two accounts. With user A, create a room. Copy
the URL. With user B, paste the URL, click the slug in the rooms list, then
"Join". You should both see each other's messages instantly.

## 4. Smoke-test the WebSocket from the shell

```bash
# Log in and save the cookie
curl -i -c /tmp/c \
  -H 'content-type: application/json' \
  http://localhost:3012/api/auth/register \
  -d '{"email":"smoke@example.com","password":"correct horse battery staple","name":"Smoke"}'

# Create a room
curl -b /tmp/c -H 'content-type: application/json' \
  http://localhost:3012/api/rooms \
  -d '{"slug":"smoke","name":"smoke test"}'

# Connect via Python (the curl WS client is awkward)
pip install websockets
python3 - <<'PY'
import asyncio, websockets, json, re
session = re.search(r'app_session\t([^\t\n]+)', open('/tmp/c').read()).group(1)
async def main():
    async with websockets.connect(
        "ws://localhost:3012/api/rooms/smoke/ws",
        additional_headers={"Cookie": f"app_session={session}"},
    ) as ws:
        print("RX joined:", await ws.recv())
        await ws.send(json.dumps({"type":"send","body":"hello via WS"}))
        print("RX msg:   ", await ws.recv())
asyncio.run(main())
PY
```

You should see two JSON lines: a `joined` event broadcast back to you,
then a `message` event with the row you just sent. The row is persisted —
`curl -b /tmp/c http://localhost:3012/api/rooms/smoke/messages` confirms.

## 5. Quality gates

```bash
# Backend
cd backend
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                              # 7 tests

# Frontend
cd frontend
pnpm check                              # svelte-check, 0 errors
pnpm build                              # production build
pnpm test:e2e                           # Playwright, 4 viewports, 36 tests
```

## 6. Reset the database

```bash
psql postgres://chat:chat_dev_password@localhost:5432/chat \
  -c "TRUNCATE messages, room_members, rooms, auth_tokens, sessions, users RESTART IDENTITY CASCADE;"
```

## 7. Tear down

```bash
docker compose down -v
```

## 8. Commit + push

```bash
cd ../..
git add projects/13-chat README.md
git commit -m "ship project 13: real-time chat rooms"
git push -u origin claude/fervent-mccarthy-2lbRU
```
