# Project 12 — Commands

Ports: backend `3011`, frontend dev `5184`, Playwright preview `4184`. Postgres on `5432`, MailHog SMTP on `1025`, MailHog web UI on `8025`.

## 1. Local infra

```bash
cd projects/12-applications
docker compose up -d                # Postgres + MailHog
```

Postgres comes up as `applications` / `applications_dev_password` on database `applications`. MailHog catches every email at <http://localhost:8025>.

```bash
# Verify the services
docker compose ps
psql postgres://applications:applications_dev_password@localhost:5432/applications -c '\dt'
```

## 2. Backend bootstrap

```bash
cd backend
cp .env.example .env
cargo run                           # listens on :3011
```

Migrations run automatically on boot (`sqlx::migrate!` in `db.rs`). The reminders task spawns immediately and ticks every `REMINDER_TICK_SECS` seconds (default 60s in dev, override to 3600 in prod via `.env`).

```bash
# Verify the API
curl http://localhost:3011/healthz                              # → ok
curl -i http://localhost:3011/api/auth/me                       # → 401 (no cookie)
```

## 3. Frontend bootstrap

```bash
cd frontend
cp .env.example .env
pnpm install
pnpm dev                            # http://localhost:5184
```

Open <http://localhost:5184/register>, create an account. Check MailHog at <http://localhost:8025> for the verification email; click the link to verify. You'll land on the dashboard.

## 4. Seed a working day

```bash
# Add an application via the UI: /applications/new
# Then open the detail page, click "Add next step", and set a due date
# within the next 24 hours. Within REMINDER_TICK_SECS the backend log will
# print "reminder sent" and MailHog will receive the email.

# Pull the ICS feed (use a real session cookie):
curl -i http://localhost:3011/api/export/next-steps.ics \
  -H "cookie: app_session=YOUR_SESSION_TOKEN" | head -20
```

Add the URL `http://localhost:3011/api/export/next-steps.ics` to Apple Calendar (`File → New Calendar Subscription`) or Google Calendar (`Other calendars → From URL`) to see your next steps alongside the rest of your life.

## 5. The legacy bcrypt → Argon2 migration drill

To prove the dual-verify path works, seed a user whose only password hash is bcrypt (no Argon2 hash):

```bash
cd backend

# Generate a real bcrypt hash for the test password
BCRYPT_HASH=$(cargo run --quiet --example genbcrypt -- 'correct horse battery staple')

# Insert a legacy user directly (note: password_hash is NULL)
psql postgres://applications:applications_dev_password@localhost:5432/applications -v hash="$BCRYPT_HASH" <<'SQL'
INSERT INTO users (id, email, name, password_hash, legacy_bcrypt_hash, email_verified_at)
VALUES (gen_random_uuid(), 'legacy@example.com', 'Legacy User', NULL, :'hash', now());
SQL

# Log in — this exercises the bcrypt fallback, then upgrades to Argon2
curl -i -c /tmp/cookies.txt http://localhost:3011/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"legacy@example.com","password":"correct horse battery staple"}'

# Verify the migration happened: password_hash is now set, legacy_bcrypt_hash is NULL
psql postgres://applications:applications_dev_password@localhost:5432/applications \
  -c "SELECT email, password_hash IS NOT NULL AS has_argon2, legacy_bcrypt_hash IS NULL AS bcrypt_cleared FROM users WHERE email='legacy@example.com';"
```

Watch the backend log for the line `legacy bcrypt → argon2 upgrade complete`.

## 6. Quality gates

```bash
# Backend
cd backend
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                              # 11 tests

# Frontend
cd frontend
pnpm check                              # svelte-check, 0 errors
pnpm build                              # production build
pnpm test:e2e                           # Playwright, 4 viewports, 36 tests
```

## 7. Reset the database

```bash
psql postgres://applications:applications_dev_password@localhost:5432/applications \
  -c "TRUNCATE users, sessions, auth_tokens, applications, application_events, next_steps RESTART IDENTITY CASCADE;"
```

The schema stays — only the rows go. Migrations don't re-run.

## 8. Tear down

```bash
docker compose down -v              # stop + delete volumes
```

The `-v` flag drops the Postgres volume so the next `up` starts from a clean slate.

## 9. Commit + push

```bash
cd ../..                            # back to repo root
git add projects/12-applications
git commit -m "ship project 12: job application tracker"
git push -u origin claude/fervent-mccarthy-2lbRU
```
