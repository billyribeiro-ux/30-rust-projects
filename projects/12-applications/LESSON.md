# Project 12 — Lesson

Headline lessons:
1. **bcrypt → Argon2 dual-verify** as a zero-downtime password-hash migration path.
2. **Background tokio task** for due-soon reminders, idempotent by construction.
3. **ICS (RFC 5545) calendar export** hand-rolled — small format, no crate needed.
4. **Status pipeline + auto-emitted timeline events** as the canonical "what changed and when" pattern.
5. **Numbered status-timeline UI** in Svelte 5 with past/current/future visual states.

Assumed knowledge (taught in earlier projects): Argon2id parameter choice, opaque session cookies, `hooks.server.ts` + cookie forwarding, `(auth)/(app)` route grouping, `AppError → IntoResponse`, sqlx compile-time-checked queries, Postgres FK cascades, Phosphor icons, plain-CSS cascade layers, Playwright permission-matrix testing — all covered in [project 11](../11-contacts/LESSON.md).

---

## A. Backend

### A.1 — Schema changes that earn their keep

The migration in `backend/migrations/0001_init.sql` keeps project 11's identity tables almost verbatim, with one consequential edit and one new column on `users`:

```sql
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
  name TEXT NOT NULL DEFAULT '',
  password_hash TEXT,                 -- WAS NOT NULL in project 11.
  legacy_bcrypt_hash TEXT,            -- NEW.
  email_verified_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (password_hash IS NOT NULL OR legacy_bcrypt_hash IS NOT NULL)
);
```

**What** — `password_hash` is now `NULL`-able and `legacy_bcrypt_hash` is added, with a row-level `CHECK` that **at least one** must be set.

**Why** — When you import users from a legacy system that hashed with bcrypt, you have their bcrypt hash but no plaintext. You can't reverse-engineer the bcrypt hash into an Argon2 one. The migration has to leave `password_hash` empty for those rows and stash the bcrypt material elsewhere. The `CHECK` constraint keeps the database honest: a fresh signup writes only `password_hash`; an import writes only `legacy_bcrypt_hash`; nothing in the system ever produces a row where both are NULL.

**What would break otherwise** — If you kept `password_hash NOT NULL` and "imported" by stuffing the bcrypt hash into it, login would call `argon2::PasswordHash::new()` on a bcrypt string and fail with a parse error. The user would be locked out forever, with no path back.

Two more constraints worth noting:

```sql
CREATE TABLE applications (
  ...
  status TEXT NOT NULL DEFAULT 'wishlist'
    CHECK (status IN (
      'wishlist','applied','screening','interview',
      'offer','accepted','rejected','withdrawn'
    )),
  ...
);
```

The status is `TEXT` with a `CHECK` whitelist instead of a Postgres ENUM. **Why** — ENUM `ALTER TYPE ... ADD VALUE` is fine forward but you can't drop a value without rewriting the column. A `TEXT + CHECK` constraint is just as fast to validate (Postgres uses a CHECK plan) and trivially editable as the product evolves. We pay one byte per row; we earn back hours every quarter.

```sql
CREATE TABLE next_steps (
  ...
  due_at TIMESTAMPTZ NOT NULL,
  completed_at TIMESTAMPTZ,
  reminded_at TIMESTAMPTZ,           -- NEW: idempotency key for the bg task
  ...
);
```

`reminded_at` is the focal point of the whole background-task design — see §A.4.

### A.2 — The dual-verify login handler

`backend/src/routes/auth.rs` is mostly project 11's login flow. The body of the handler now branches into the migration path:

```rust
let row = sqlx::query!(
    r#"SELECT id, name, password_hash, legacy_bcrypt_hash, email_verified_at
       FROM users WHERE email = $1"#,
    email,
).fetch_optional(&s.pool).await?;

const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

let (user_id, name, email_verified, valid, was_legacy) = match row {
    Some(u) => {
        let argon_ok = match u.password_hash.as_deref() {
            Some(h) => hash::verify_password(&input.password, h)?,
            None => {
                let _ = hash::verify_password(&input.password, DUMMY_HASH);
                false
            }
        };
        if argon_ok {
            (u.id, u.name, u.email_verified_at.is_some(), true, false)
        } else if let Some(legacy) = u.legacy_bcrypt_hash.as_deref() {
            let bcrypt_ok = hash::verify_bcrypt(&input.password, legacy)?;
            (u.id, u.name, u.email_verified_at.is_some(),
             bcrypt_ok, bcrypt_ok)
        } else {
            (u.id, u.name, u.email_verified_at.is_some(), false, false)
        }
    }
    None => {
        let _ = hash::verify_password(&input.password, DUMMY_HASH);
        (Uuid::nil(), String::new(), false, false, false)
    }
};

if !valid { return Err(AppError::Unauthorized); }
```

There are **three** identity-leak paths here, all of which the dummy-hash trick closes:

| Path | What attacker sees if NOT defended | Our defense |
|---|---|---|
| No such email | Response in 1ms (no hashing) | Verify against `DUMMY_HASH` |
| Email exists, only legacy hash, wrong password | Bcrypt-only timing (~10ms) different from Argon2 (~80ms) | Verify against `DUMMY_HASH` *first* |
| Email exists, only modern hash, wrong password | Argon2 timing (~80ms) | The natural path |

The attacker can't distinguish "no such user" from "wrong password" from "legacy-only user" by stopwatch. That's user-enumeration prevention done at the millisecond level.

Then, on a successful bcrypt verify, the upgrade:

```rust
if was_legacy {
    match hash::hash_password(&input.password) {
        Ok(new_hash) => {
            if let Err(e) = sqlx::query!(
                "UPDATE users SET password_hash = $1, legacy_bcrypt_hash = NULL WHERE id = $2",
                new_hash, user_id,
            ).execute(&s.pool).await {
                tracing::warn!(err = ?e, user_id = %user_id,
                    "legacy-to-argon2 upgrade failed; will retry next login");
            } else {
                tracing::info!(user_id = %user_id,
                    "legacy bcrypt → argon2 upgrade complete");
            }
        }
        Err(e) => tracing::warn!(err = ?e, "argon2 re-hash failed during legacy upgrade"),
    }
}
```

**What** — On a successful bcrypt verify, re-hash the plaintext (which we still have in scope from the request body) with Argon2 and `UPDATE` the row to set `password_hash` and `NULL` out `legacy_bcrypt_hash`.

**Why** — This makes the migration **opportunistic**: each successful login peels one user off the legacy hash. After a release week most active users are migrated; after a quarter you can drop the column. No batch job, no maintenance window, no "everyone reset your password" email.

**What would break otherwise** — A naive approach is to do this re-hash inside a transaction with the session creation, and abort the login if the upgrade fails. That couples two unrelated operations: the user is authenticated, but they can't get in because a *non-essential* write to the same row failed. We log and proceed. The CHECK constraint guarantees the row stays valid (Argon2 is set before bcrypt is cleared, atomically, in the same UPDATE statement).

**Try it:** Look at `backend/examples/genbcrypt.rs`. It's a 5-line CLI that hashes an arg with bcrypt cost 12. Use it to seed a legacy user, log in via curl, and watch the upgrade log line appear. See COMMANDS.md §5.

### A.3 — bcrypt helper in `auth/hash.rs`

The hash module gains:

```rust
pub fn verify_bcrypt(password: &str, bcrypt_hash: &str) -> AppResult<bool> {
    bcrypt::verify(password, bcrypt_hash)
        .map_err(|e| AppError::Internal(format!("bcrypt verify failed: {e}")))
}

#[cfg(test)]
pub fn hash_bcrypt(password: &str) -> AppResult<String> {
    bcrypt::hash(password, 12)
        .map_err(|e| AppError::Internal(format!("bcrypt hash failed: {e}")))
}
```

Note the `#[cfg(test)]` on `hash_bcrypt`. The production code path **never** creates a bcrypt hash — bcrypt is one-way for us, an artifact of imports. We only need to *verify* against existing hashes. Limiting the hash function to test builds means a future contributor can't accidentally reach for bcrypt when adding a feature.

### A.4 — The background reminder task

`backend/src/reminders.rs` is the entire feature in ~70 lines. The headline contract:

> 1. Spawn via `tokio::spawn`. Same runtime as the HTTP server, no extra process.
> 2. Own a clone of `AppState` (Arc-cheap).
> 3. Loop forever on `tokio::time::interval(tick_interval)`.
> 4. **Per-tick errors are LOGGED, not propagated.** A `?` here would let one transient DB failure kill the task forever.
> 5. The `reminded_at` column is the idempotency key.

Wired in `main.rs`:

```rust
let reminder_state = state.clone();
let reminder_interval = std::env::var("REMINDER_TICK_SECS")
    .ok()
    .and_then(|v| v.parse::<u64>().ok())
    .unwrap_or(60); // 60s in dev, override to 3600 in prod
tokio::spawn(async move {
    reminders::run(
        reminder_state,
        std::time::Duration::from_secs(reminder_interval),
    ).await;
});
```

The tick body:

```rust
async fn tick(state: &AppState) -> Result<(), sqlx::Error> {
    let horizon = chrono::Utc::now() + Duration::hours(24);
    let rows = sqlx::query!(
        r#"
        SELECT ns.id, ns.body, ns.due_at, u.email, u.name, a.company, a.role
        FROM next_steps ns
        JOIN users u ON u.id = ns.user_id
        JOIN applications a ON a.id = ns.application_id
        WHERE ns.completed_at IS NULL
          AND ns.reminded_at IS NULL
          AND ns.due_at <= $1
        LIMIT 100
        "#,
        horizon,
    ).fetch_all(&state.pool).await?;

    for r in rows {
        state.mailer.send(&r.email, &subj, body).await;
        sqlx::query!(
            "UPDATE next_steps SET reminded_at = now() WHERE id = $1",
            r.id,
        ).execute(&state.pool).await?;
    }
    Ok(())
}
```

**What** — Each tick, query for rows that are due within 24 hours, not completed, and not yet reminded. Send the email. Stamp `reminded_at`.

**Why this is idempotent** — If the task crashes between "send email" and "stamp reminded_at" for a given row, that row will be re-selected on the next tick — and the user will get a *duplicate* email. We accept that. The alternative — stamping *before* sending — means a row that has actually never been emailed gets marked as if it were. We chose "at-least-once with rare duplicates" over "at-most-once with rare misses." For reminders, duplicate is annoying; miss is the bug.

**What would break otherwise** — A common temptation is to have a separate `reminders_sent` table (one row per send). That introduces a transactional question: is the send committed when the row is written, or after the SMTP round trip? If you commit before send, you've reintroduced the "stamped but never sent" failure mode. If after, you've doubled your round trips. The `reminded_at` column on `next_steps` collapses the problem to one UPDATE per send.

**Other production gotchas the lesson glosses over** —
- `LIMIT 100` prevents one tick from doing unbounded work on a backlog.
- A future scale move is `SELECT ... FOR UPDATE SKIP LOCKED` so multiple workers can share the queue. Project 23 (Background Jobs Dashboard) does that properly.

### A.5 — Auto-emitted status_change events

`backend/src/routes/applications.rs` `update()` checks whether the incoming patch changes `status` and, if so, emits an `application_events` row in the same transaction:

```rust
let row = sqlx::query_as!(/* SELECT current values */).fetch_one(&s.pool).await?;
let new_status = patch.status.as_ref().unwrap_or(&row.status);

if new_status != &row.status {
    sqlx::query!(
        "INSERT INTO application_events
           (id, application_id, user_id, kind, body, new_status, occurred_at)
         VALUES (gen_random_uuid(), $1, $2, 'status_change', '', $3, now())",
        row.id, user.id, new_status,
    ).execute(&s.pool).await?;
}
// ... then UPDATE applications
```

**What** — Whenever the status changes, write an audit row.

**Why in the handler, not a trigger?** — Project 24 introduces audit triggers (the *real* "you can't trust application code to remember to log" pattern, with `pg_audit`-style coverage). For this single-table case, a trigger would tie the schema to the audit logic in a way that's harder to test from `cargo test`. Doing it in the handler is fine for now; project 24 graduates to triggers when it actually matters (multi-tenant compliance).

**What would break otherwise** — If you emit the event *after* the UPDATE returns, and the UPDATE succeeds but the INSERT fails, you've lost the audit row. Doing it before (as we do) means the worst case is "audit row exists for a status change that didn't actually happen" — and that's caught by `application_events.application_id REFERENCES applications(id) ON DELETE CASCADE` blocking the orphan.

### A.6 — ICS export hand-rolled

`backend/src/routes/export.rs` builds a VCALENDAR string and returns it with the right content type. The lesson is two-fold: the format is small enough that you don't need a crate, and the axum response API has type-inference foot-guns worth knowing about.

```rust
async fn ics(State(s): State<AppState>, user: AuthUser) -> AppResult<Response<Body>> {
    let rows = /* ... SELECT next_steps for user ... */;

    let mut buf = String::new();
    write_line(&mut buf, "BEGIN:VCALENDAR");
    write_line(&mut buf, "VERSION:2.0");
    write_line(&mut buf, "PRODID:-//30-rust-projects//Project 12 Job Tracker//EN");
    write_line(&mut buf, "CALSCALE:GREGORIAN");
    write_line(&mut buf, "METHOD:PUBLISH");
    write_line(&mut buf, "X-WR-CALNAME:Job applications — next steps");

    for r in rows {
        write_line(&mut buf, "BEGIN:VEVENT");
        write_line(&mut buf, &format!("UID:{}-12-applications@30-rust-projects.local", r.id));
        write_line(&mut buf, &format!("DTSTAMP:{}", ics_dt(&r.created_at)));
        write_line(&mut buf, &format!("DTSTART:{}", ics_dt(&r.due_at)));
        write_line(&mut buf, "DURATION:PT30M");
        write_line(&mut buf, &format!("SUMMARY:{}",
            ics_escape(&format!("{} — {} ({})", r.body, r.company, r.role))));
        write_line(&mut buf, if r.completed_at.is_some() { "STATUS:COMPLETED" } else { "STATUS:CONFIRMED" });
        write_line(&mut buf, "BEGIN:VALARM");
        write_line(&mut buf, "ACTION:DISPLAY");
        write_line(&mut buf, &format!("DESCRIPTION:{}", ics_escape(&r.body)));
        write_line(&mut buf, "TRIGGER:-PT1H");
        write_line(&mut buf, "END:VALARM");
        write_line(&mut buf, "END:VEVENT");
    }
    write_line(&mut buf, "END:VCALENDAR");

    let resp = Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"next-steps.ics\"")
        .body(Body::from(buf))
        .expect("response builds");
    Ok(resp)
}
```

The three RFC 5545 things you can't get wrong:

1. **Line endings are CRLF.** Section 3.1: "*Lines of text SHOULD NOT be longer than 75 octets ... Lines SHALL be terminated by a CRLF sequence.*" The `write_line` helper appends `\r\n`. If you use plain `\n`, half the calendar clients silently refuse the file.
2. **UTC timestamps are `20260524T143000Z` — no separators.** Section 3.3.5. The `ics_dt` helper formats with `%Y%m%dT%H%M%SZ`. Don't accidentally use `%Y-%m-%dT%H:%M:%SZ` (that's ISO 8601 *extended*; ICS wants *basic*).
3. **Escape backslash, comma, semicolon, newline** in text values. Section 3.3.11. `ics_escape` handles all four. The order matters: replace `\` first, then the rest — otherwise the new `\,` you just wrote gets re-escaped to `\\,`.

The `VALARM` block with `TRIGGER:-PT1H` tells the client to fire a reminder 1 hour before the event. The leading `-` is the spec's way of saying "before" — `PT1H` alone would mean "1 hour *after* start", which is useless for due-date reminders.

**What** — return type is `Response<Body>` not `impl IntoResponse`.

**Why** — `axum::response::Response::builder().body(buf)` is ambiguous: `buf` is `String`, and `Response<T>` is generic over `T`, and there isn't a single `Into<Body>` impl that wins. Returning the explicitly-typed `Response<Body>` and constructing with `Body::from(buf)` pins down the body type. Going through `IntoResponse` works but obscures the path; for binary/streaming responses (later projects: file vault, image gallery, HLS chunks) you'll write the same shape.

### A.7 — Routes wired in main

```rust
let app = Router::new()
    .nest("/api/auth", routes::auth::router())
    .nest("/api/applications", routes::applications::router())
    .nest("/api/applications/{application_id}/events", routes::events::router())
    .nest("/api/applications/{application_id}/next-steps", routes::next_steps::router())
    .nest("/api/dashboard", routes::dashboard::router())
    .nest("/api/export", routes::export::router())
    .route("/healthz", axum::routing::get(health))
    .with_state(state)
    .layer(TraceLayer::new_for_http())
    .layer(cors);
```

Two nested routers under the same parent path use the `{application_id}` axum 0.8 path-parameter syntax (the `{...}` form replaced `:name` between axum 0.7 and 0.8 — bear that in mind when porting old examples). Inside `events::router()` and `next_steps::router()` the handlers extract `Path((application_id, id))` to enforce that the child resource belongs to the application in the URL — never just `WHERE id = $1`, always `WHERE id = $1 AND application_id = $2 AND user_id = $3`.

---

## B. Frontend

### B.1 — Types match the wire exactly

`frontend/src/lib/types.ts`:

```ts
export type AppStatus =
  | 'wishlist' | 'applied' | 'screening' | 'interview'
  | 'offer'    | 'accepted' | 'rejected' | 'withdrawn';

export const STATUS_LABEL: Record<AppStatus, string> = {
  wishlist: 'Wishlist', applied: 'Applied', screening: 'Screening',
  interview: 'Interview', offer: 'Offer', accepted: 'Accepted',
  rejected: 'Rejected', withdrawn: 'Withdrawn',
};
```

The 8 values mirror the backend CHECK constraint exactly. The `STATUS_LABEL` map is the single source of human-readable strings; templates use `STATUS_LABEL[s.status]` instead of capitalizing-and-hoping. **If you add a status, TypeScript will fail the build until you add the label** — exhaustiveness via the `Record<AppStatus, string>` type.

### B.2 — Status timeline UI with past/current/future states

`frontend/src/routes/(app)/applications/[id]/+page.svelte` builds the centerpiece UI: a horizontal numbered timeline that shows where the application is in the pipeline:

```svelte
{@const pipeline = ['wishlist','applied','screening','interview','offer','accepted'] as const}
{@const currentIdx = pipeline.indexOf(a.status as typeof pipeline[number])}

<ol class="timeline">
  {#each pipeline as step, i (step)}
    <li class:past={i < currentIdx} class:current={i === currentIdx} class:future={i > currentIdx}>
      <form method="POST" action="?/status" use:enhance>
        <input type="hidden" name="status" value={step} />
        <button type="submit" class="step">
          <span class="num">{i + 1}</span>
          <span class="label">{STATUS_LABEL[step]}</span>
        </button>
      </form>
    </li>
  {/each}
</ol>
```

**What** — 6 ordered steps (the "pipeline" excludes `rejected` and `withdrawn`, which are off-ramps, not stages). Each step is a form button that POSTs the corresponding status. CSS gives `.past` a filled circle, `.current` a glowing one, `.future` a hollow one.

**Why a form, not a remote function** — Either works. Form actions degrade to a plain HTML form when JS is disabled — you can advance the status with no JavaScript at all. Remote functions (introduced project 8) need JS to fire. For a public-facing tool people might use on flaky transit Wi-Fi, the form action is the more honest default.

**What would break otherwise** — A single `<button onclick={() => advance(step)}>` with no `<form>` wrapper is the obvious-but-wrong path. It's invisible to assistive tech as anything but a button, can't be CSRF-protected by SvelteKit's same-origin check, and can't progressive-enhance.

### B.3 — The `use:enhance` pattern for status changes

The form action runs server-side and returns the updated application. `use:enhance` (no callback) keeps the URL the same and updates `$page.data` in place — no full reload, no flash. The status timeline re-renders with `.current` shifted by one step.

**Try it:** Add a `console.log(form)` in the page's `$effect` to watch the form-action response flow through. It's one of the cleaner SvelteKit data-flow stories: server mutation → server-side `load()` re-run → reactive `$state` swap → DOM diff.

### B.4 — The dashboard's "Due this week" + ICS subscribe hint

`frontend/src/routes/(app)/+page.svelte`:

```svelte
<section class="panel" aria-label="Due this week">
  <h2>Due this week</h2>
  {#if d.due_this_week.length === 0}
    <p class="empty">Nothing due.</p>
  {:else}
    <ul>
      {#each d.due_this_week as s (s.id)}
        <li>
          <a href="/applications/{s.application_id}">{s.company}</a> — {s.body}
          <time datetime={s.due_at}>{new Date(s.due_at).toLocaleDateString()}</time>
        </li>
      {/each}
    </ul>
  {/if}
  <p class="hint">
    <a href="/api/export/next-steps.ics">Subscribe via ICS</a>
    — add this URL to Apple Calendar / Google Calendar for due-date reminders.
  </p>
</section>
```

The hint link points at `/api/export/next-steps.ics` (the backend, proxied or same-origin in prod). It's a `text/calendar` response with `Content-Disposition: attachment` so clicking it downloads the file *and* lets calendar apps recognise the URL when pasted as a subscription source.

### B.5 — The `<time datetime>` discipline

Every date in the UI is wrapped in a `<time datetime="...">` element with the *machine-readable* ISO-8601 string in the attribute and the *human-readable* localized string as text. **What this earns**: screen readers announce dates predictably; `<time>` is semantic HTML; Google's date-parser for AI Overviews picks up the attribute; and CSS can target `time[datetime]` for restyling.

```svelte
<time datetime={s.due_at}>{new Date(s.due_at).toLocaleDateString()}</time>
```

### B.6 — Type-safe API client surface

`frontend/src/lib/api.ts` exposes namespaced clients per resource:

```ts
export const applicationsApi = {
  list:   (f, params?) => http(f, `/api/applications${qs(params)}`),
  read:   (f, id)      => http(f, `/api/applications/${id}`),
  create: (f, body)    => http(f, '/api/applications', { method: 'POST', json: body }),
  update: (f, id, p)   => http(f, `/api/applications/${id}`, { method: 'PATCH', json: p }),
  remove: (f, id)      => http(f, `/api/applications/${id}`, { method: 'DELETE' }),
};
```

Same pattern as project 11. The `f` parameter is the fetch function — either `event.fetch` (in `+page.server.ts`, browser context) or `serverFetch(locals.sessionCookie)` (cross-origin to the backend with cookie forwarding). The hooks pattern is unchanged from project 11.

### B.7 — Playwright permission matrix, applied to applications

`frontend/e2e/auth-matrix.spec.ts` is the headline test. The "user A cannot see user B's application" case:

```ts
test("PERMISSION MATRIX: user A cannot read user B's application", async ({ request }) => {
  const a = await apiRegister(request);
  const b = await apiRegister(request);
  const bAppId = await apiCreateApplication(request, b.cookie, 'B-Owned Corp');

  const res = await request.get(`${BACKEND}/api/applications/${bAppId}`,
    { headers: { cookie: `${COOKIE}=${a.cookie}` } });
  expect(res.status()).toBe(404);     // not 403 — never confirm existence
  // ... patch + delete also 404
  const bRead = await request.get(`${BACKEND}/api/applications/${bAppId}`,
    { headers: { cookie: `${COOKIE}=${b.cookie}` } });
  expect(bRead.status()).toBe(200);
});
```

**Why 404 and not 403** — A 403 ("forbidden") tells the attacker the resource *exists*; only the access control denied them. A 404 ("not found") is indistinguishable from "this id was never used." For per-user scoping, you want the latter: the existence of a row in another tenant's space should be invisible. Practically, the `WHERE user_id = $1` clause in our SELECT means we don't even know the row exists — there's no `if (row.user_id != me)` branch where someone could swap the comparison for `==` by mistake.

The "ICS endpoint returns text/calendar" test:

```ts
test('ICS calendar export is text/calendar and contains VCALENDAR', async ({ request }) => {
  const { cookie } = await apiRegister(request);
  const res = await request.get(`${BACKEND}/api/export/next-steps.ics`,
    { headers: { cookie: `${COOKIE}=${cookie}` } });
  expect(res.status()).toBe(200);
  expect(res.headers()['content-type']).toMatch(/text\/calendar/);
  const body = await res.text();
  expect(body).toContain('BEGIN:VCALENDAR');
  expect(body).toContain('END:VCALENDAR');
});
```

Cheap, but it guards three regressions at once: the route exists, the content type is right, and the body shape is at least a valid VCALENDAR envelope. Combined with the `dt_round_trip` and `escape_handles_specials` unit tests in `export.rs`, the format-level correctness is locked down.

---

## C. Patterns to carry forward

- **Dual-verify for hash migrations** generalises beyond bcrypt. The same shape — "try modern, fall back to legacy, upgrade on success" — works for migrating between Argon2 parameter sets (project 24 bumps `m_cost` from 19456 → 47104 and re-hashes on each login).
- **Idempotency via a single column on the same table** as the entity (`reminded_at`, `processed_at`, `notified_at`) is almost always simpler than a sidecar table. Use a sidecar only when you need *multiple* idempotent operations per entity.
- **Status pipelines as `TEXT + CHECK + Record<Union, Label>`** trade a tiny storage cost for free-extensibility. Project 24 (help desk) and project 30 (capstone PM tool) both use this.
- **Hand-rolled formats with tight specs** (ICS, sitemap.xml, robots.txt, RSS, OpenSearch description) are 50–150 LOC each and deserve to be in your repo, not as a dependency. The spec is the contract; the code is the implementation; both are visible in code review.

### Try it (exercises)

1. Add a **`withdrawn_at`** column to `applications`, set by the action that flips status to `withdrawn`, and surface it on the timeline. (Hint: same shape as `applied_at` already in the schema.)
2. Make the reminder email **include the ICS subscribe URL** in its body, so a user who hasn't subscribed yet gets a one-click prompt.
3. Add an **iCal `RRULE`** to the VEVENT for "monthly check-in" next steps — see RFC 5545 §3.8.5.3. Calendar apps will fan out the single VEVENT into recurring entries. (You'll need a new `recurrence` column on `next_steps`.)
4. Run the dual-verify migration **in reverse** as a thought experiment: what would it take to support an exported-to-bcrypt mode? (Answer: you can't, without forcing every user to log in once to capture plaintext. That asymmetry — Argon2 → bcrypt is one-way harder — is why we lock in the modern format on first contact.)

---

Project 12 is shipped when:

- [x] `cargo fmt --check` clean
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `cargo test` — 11/11 passing
- [x] `pnpm check` — 0 errors, 0 warnings
- [x] `pnpm build` — production build succeeds
- [x] `pnpm test:e2e` — 36/36 (9 tests × 4 viewports)
- [x] Live dual-verify drill — legacy bcrypt user logs in, row migrates to Argon2 in DB
