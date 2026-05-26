# Project 11 — Lesson

Headline lessons:
1. **Postgres + Docker Compose** for local infra.
2. **Argon2id + server-side sessions** as the auth foundation for projects 11–30.
3. **`hooks.server.ts`** as the single place auth is resolved on the frontend.
4. **Cookie forwarding** from SvelteKit's Node server to the cross-origin Rust backend.
5. **Route groups `(auth)` and `(app)`** with layout guards as the auth-routing model.
6. **`tsvector` full-text search** with a GENERATED column + GIN index.
7. **Per-user data scoping enforced server-side** + the **permission-matrix tests** that prove it.

---

## A. Backend

### A.1 — Docker Compose

```yaml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: contacts
      POSTGRES_PASSWORD: contacts_dev_password
      POSTGRES_DB: contacts
    ports: ["5432:5432"]
    volumes: [postgres_data:/var/lib/postgresql/data]
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U contacts -d contacts"]
  mailhog:
    image: mailhog/mailhog:latest
    ports: ["1025:1025", "8025:8025"]
volumes: { postgres_data: {} }
```

Two services, one volume. `mailhog` catches all outbound SMTP into a browser UI at http://localhost:8025 — zero config, no real delivery. The healthcheck is what `docker compose up --wait` looks at; in our world we just `docker compose up -d` and proceed.

### A.2 — Postgres schema highlights

The migration introduces several Postgres-specific patterns that recur through every later project:

```sql
CREATE TABLE users (
  id UUID PRIMARY KEY,
  email TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
  ...
);
```

- **`UUID`** native type instead of TEXT. Native UUIDs are 16 bytes vs 36 for the canonical hyphenated form — faster comparisons, smaller indexes.
- **`CHECK (email = lower(email))`** enforces the normalization invariant at the DB level. App code lowercases on input; the DB ensures nobody ever sneaks a mixed-case row in via direct SQL.

```sql
CREATE TABLE sessions (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash BYTEA NOT NULL UNIQUE,
  expires_at TIMESTAMPTZ NOT NULL,
  ...
);
```

- **`BYTEA`** for the SHA-256 hash (32 bytes), not TEXT. Comparing 32 raw bytes is cheaper than comparing 64 hex chars + skipping case rules.
- **`TIMESTAMPTZ`** stores in UTC with zone awareness — never use plain `TIMESTAMP` in a real product.
- **`ON DELETE CASCADE`** removes sessions when a user is deleted (account closure).

```sql
CREATE TABLE contacts (
  ...
  search_tsv TSVECTOR GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', coalesce(name, '')),    'A') ||
    setweight(to_tsvector('simple', coalesce(email, '')),   'B') ||
    setweight(to_tsvector('simple', coalesce(company, '')), 'B') ||
    setweight(to_tsvector('simple', coalesce(notes, '')),   'D')
  ) STORED
);
CREATE INDEX idx_contacts_search_tsv ON contacts USING GIN (search_tsv);
```

- **`GENERATED ALWAYS AS ... STORED`** computes `search_tsv` on every INSERT/UPDATE automatically. The application never writes to it; it can never go out of sync.
- **`setweight(...)`** stamps each token with a weight letter (A best, D worst). `ts_rank` later orders results by relevance using those weights.
- **`'simple'`** dictionary instead of `'english'`: 'simple' just lower-cases, no stemming. Stemming makes "running" find "run" but also makes "axum" find "ax". For names + emails, 'simple' is safer.
- **GIN index** on the tsvector makes `@@ plainto_tsquery(...)` fast.

```sql
CREATE TRIGGER trg_contacts_updated_at
  BEFORE UPDATE ON contacts
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
```

The trigger keeps `updated_at` fresh on every UPDATE without the app having to remember. Project 22 (background jobs) revisits triggers for audit logging.

### A.3 — `auth/hash.rs` — Argon2id

```rust
fn params() -> Params {
    Params::new(19_456, 2, 1, Some(32)).expect("argon2 params valid")
}
```

OWASP 2025 first-class recommendation for interactive logins:
- **`m_cost = 19_456`** KiB (~19 MiB) — RAM per hash. The dominant attack defense.
- **`t_cost = 2`** iterations.
- **`p_cost = 1`** lane (parallelism).
- **`output = 32`** bytes (256-bit hash).

A single login takes ~30ms on a 2024-class CPU. Multiply by `m_cost` MiB and you get the GPU cost: attacker with a 24GB GPU can do ~1200 parallel hashes vs millions for SHA-256. **That ratio is the whole point of Argon2id.**

The PHC-formatted output string captures salt + params, so a hash is self-describing — no parallel schema needed:

```
$argon2id$v=19$m=19456,t=2,p=1$<22-char-salt>$<43-char-hash>
```

`verify_password` parses the PHC, re-runs Argon2 with the same params, constant-time compares. Returns:
- `Ok(true)` — password matches
- `Ok(false)` — password mismatch
- `Err(Internal)` — the stored hash itself is malformed (corruption — loud error, not silent reject)

### A.4 — `auth/session.rs` — server-side sessions

```rust
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)  // 43 chars, cookie-safe
}

pub fn hash_token(raw: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    h.finalize().to_vec()
}
```

Three properties matter:
1. **32 bytes of `OsRng` entropy** = 256 bits. Unguessable.
2. **URL-safe base64 without padding** = 43 chars, safe in cookies + URLs.
3. **SHA-256 hash stored, raw in cookie.** A DB leak alone doesn't grant impersonation — the attacker also needs the live cookie value.

Sliding expiry happens on every lookup:

```rust
let new_expires = Utc::now() + Duration::days(SESSION_TTL_DAYS);
sqlx::query!("UPDATE sessions SET last_seen_at = now(), expires_at = $1 WHERE id = $2",
             new_expires, r.session_id).execute(pool).await?;
```

A logged-in user stays logged in indefinitely as long as they visit at least once per 30 days. Project 17 (URL shortener + 2FA) adds device-list views that surface `last_seen_at` and `ip`/`user_agent`.

#### Cookie attributes

```rust
Cookie::build((SESSION_COOKIE_NAME, raw_token))
    .http_only(true)            // no document.cookie reads
    .secure(secure)             // HTTPS-only in prod
    .same_site(SameSite::Lax)   // CSRF baseline
    .path("/")
    .max_age(time::Duration::days(SESSION_TTL_DAYS))
    .build()
```

- **HttpOnly** stops XSS from stealing the cookie.
- **Secure** stops MITM on plain HTTP. We set it to `false` in dev because browsers reject `Secure` on `http://localhost:5183`.
- **SameSite=Lax** is the modern CSRF default — the cookie is sent on top-level navigations (links) but NOT on cross-site POSTs. For state-changing requests from other origins, the browser drops the cookie → the backend sees no session → 401.

#### The `AuthUser` extractor

```rust
impl<S> FromRequestParts<S> for AuthUser
where S: Send + Sync, AppState: axum::extract::FromRef<S>,
{
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state: AppState = axum::extract::FromRef::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(cookie) = jar.get(SESSION_COOKIE_NAME) else {
            return Err(AppError::Unauthorized);
        };
        match lookup(&app_state.pool, cookie.value()).await? {
            Some(u) => Ok(u),
            None => Err(AppError::Unauthorized),
        }
    }
}
```

Any handler can ask for `user: AuthUser` in its signature. Missing/expired cookie → 401 before the handler body even runs. **This means every protected route is one signature-tweak away from being auth-required**, and the auth check can never be accidentally skipped — if you don't request the extractor, you're explicitly unauth.

### A.5 — `auth/tokens.rs` — single-use tokens

Verify-email and password-reset are the same shape:

```rust
let row = sqlx::query!(
    r#"
    UPDATE auth_tokens
    SET consumed_at = now()
    WHERE token_hash = $1
      AND kind = $2
      AND expires_at > now()
      AND consumed_at IS NULL
    RETURNING user_id
    "#,
    token_hash, kind_str,
).fetch_optional(pool).await?;
row.map(|r| r.user_id).ok_or(AppError::NotFound)
```

The `WHERE consumed_at IS NULL` + `UPDATE ... RETURNING` is the atomic single-use guard. Two concurrent verify attempts on the same token will see one succeed (`RETURNING` produces the row) and one fail (no row to update); there's no read-then-write race window.

### A.6 — `routes/auth.rs` — no-enumeration discipline

Three places we never leak whether a given email is registered:

**Login** — `/api/auth/login` runs Argon2 even when the user doesn't exist:

```rust
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
let (..., valid) = match row {
    Some(u) => (u.id, ..., hash::verify_password(&password, &u.password_hash)?),
    None => {
        let _ = hash::verify_password(&password, DUMMY_HASH);  // burns same CPU time
        (Uuid::nil(), ..., false)
    }
};
```

Without the dummy-hash verification, an attacker times two responses — "no such user" returns in 10ms (DB miss); "wrong password" returns in 30ms (DB hit + Argon2 verify). The 20ms gap reveals which emails exist. With the dummy verify, both paths take ~30ms.

**Forgot** — `/api/auth/forgot` always returns 204:

```rust
if let Some(u) = row {
    let raw_token = tokens::issue(...).await?;
    s.mailer.send(...).await;
}
Ok(StatusCode::NO_CONTENT)  // always
```

**Register** — duplicate email returns 409 with a generic message:

```rust
if let Err(sqlx::Error::Database(dbe)) = &result
    && dbe.constraint() == Some("users_email_key")
{
    return Err(AppError::Conflict(
        "an account with that email already exists".into(),
    ));
}
```

The error message is generic and the path is HTTP 409 (Conflict), not a custom code that distinguishes "duplicate email" from other conflict types.

### A.7 — `routes/contacts.rs` — per-user scoping

Every contacts query starts with `WHERE c.user_id = $1` where `$1` comes from the `AuthUser` extractor. The user's id is never accepted from the client — we don't even have a path like `/api/users/:user_id/contacts`. The client says "show me MY contacts" via the cookie; the server says "your cookie maps to user X; here are X's rows."

```rust
async fn read(State(s): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<Json<Contact>> {
    let row = sqlx::query!(
        "... WHERE c.id = $1 AND c.user_id = $2 GROUP BY c.id",
        id, user.id,
    ).fetch_optional(&s.pool).await?.ok_or(AppError::NotFound)?;
    ...
}
```

When user A asks for a contact id owned by user B, the `AND c.user_id = $2` clause produces zero rows → `AppError::NotFound`. We return **404, not 403** — confirming the row exists at all leaks info. The Playwright permission-matrix test asserts both behaviors.

### A.8 — FTS query construction

```sql
SELECT ..., ts_rank(c.search_tsv, plainto_tsquery('simple', $3)) as ...
FROM contacts c
WHERE c.search_tsv @@ plainto_tsquery('simple', $3)
ORDER BY ts_rank(c.search_tsv, plainto_tsquery('simple', $3)) DESC,
         c.updated_at DESC
```

- **`plainto_tsquery('simple', $3)`** is forgiving — accepts user input verbatim, strips operators, no quoting needed. For power-user query syntax (AND/OR/NEAR) you'd use `websearch_to_tsquery`.
- **`@@`** is the match operator: "does this tsvector match this tsquery?"
- **`ts_rank(...)`** scores each match by weight (A > B > C > D) and proximity. Sort DESC for relevance-first results, then fall back to `updated_at` for ties.

---

## B. Frontend

### B.1 — `hooks.server.ts` — the auth handle

```ts
export const handle: Handle = async ({ event, resolve }) => {
  const raw = event.cookies.get(SESSION_COOKIE);
  event.locals.sessionCookie = raw ?? null;
  event.locals.user = null;

  if (raw) {
    try {
      const f: typeof fetch = (input, init) =>
        fetch(input as RequestInfo, {
          ...init,
          headers: { ...(init?.headers ?? {}), cookie: `${SESSION_COOKIE}=${raw}` }
        });
      event.locals.user = await authApi.me(f);
    } catch (err) {
      if (err instanceof ApiCallError && err.status === 401) {
        event.cookies.delete(SESSION_COOKIE, { path: '/' });
      } else {
        console.error('hooks.server.ts: /me lookup failed', err);
      }
    }
  }

  return resolve(event);
};
```

Five things going on:

1. **Every request flows through `handle`** — there's no other auth check in the codebase.
2. **The cookie is read on the server**, populating `locals.sessionCookie` (raw value) and `locals.user` (resolved). Children consume via `event.locals.user` — typed via `src/app.d.ts`.
3. **Cookie forwarding to a cross-origin backend.** `event.fetch` only forwards cookies for same-origin requests; our backend lives on `:3010`. So we wrap `fetch` to manually attach the cookie header for every call.
4. **Auto-cleanup of stale cookies.** If the backend says 401 (cookie expired or revoked), we `cookies.delete` it so the browser stops sending it on subsequent requests.
5. **Non-401 errors are logged but don't fail the request.** The page can still render in anonymous mode if the backend is briefly unreachable.

### B.2 — `app.d.ts` — typed locals

```ts
declare global {
  namespace App {
    interface Locals {
      user: import('$lib/types').User | null;
      sessionCookie: string | null;
    }
  }
}
```

This single declaration makes `event.locals.user` typed across every `load` and action in the app. TypeScript catches "I tried to use `user.email` but I forgot to check if it's null" at compile time.

### B.3 — `(auth)` and `(app)` route groups

SvelteKit's route groups (`(foo)`) are folders whose name does NOT appear in the URL but DO scope a layout. We use them as the bouncer:

**`(auth)/+layout.server.ts`** — if you're logged in, you don't belong here:
```ts
export const load: LayoutServerLoad = async ({ locals }) => {
  if (locals.user) redirect(303, '/');
  return {};
};
```

**`(app)/+layout.server.ts`** — if you're not logged in, you don't belong here:
```ts
export const load: LayoutServerLoad = async ({ locals, url }) => {
  if (!locals.user) {
    redirect(303, `/login?next=${encodeURIComponent(url.pathname + url.search)}`);
  }
  return { user: locals.user };
};
```

Two `+layout.server.ts` files. No auth checks anywhere else. New pages added to `(app)/...` get the gate for free.

### B.4 — `lib/server/api.ts` — `serverFetch` helper

The hooks pattern of "wrap fetch to add the cookie header" recurs in every server-side `load` that hits the backend. We extract it:

```ts
export function serverFetch(cookie: string | null): typeof fetch {
  if (!cookie) return fetch;
  return ((input, init) =>
    fetch(input as RequestInfo, {
      ...init,
      headers: { ...(init?.headers ?? {}), cookie: `${SESSION_COOKIE}=${cookie}` }
    })) as typeof fetch;
}
```

Used everywhere:

```ts
export const load: PageServerLoad = async ({ locals }) => {
  const data = await dashboardApi.get(serverFetch(locals.sessionCookie));
  return { dashboard: data };
};
```

### B.5 — Forwarding the backend's Set-Cookie to the browser

When the user posts the login form, the SvelteKit action fetches the backend's `/api/auth/login`. The backend returns 200 + `Set-Cookie: contacts_session=...`. We need to land that cookie in the **browser's** cookie store, not the Node server's. SvelteKit's `cookies.set` writes to the response that goes back to the browser:

```ts
const header = res.headers.get('set-cookie');
const match = header?.match(/contacts_session=([^;]+)/);
if (match) {
  cookies.set(SESSION_COOKIE, match[1]!, {
    path: '/',
    httpOnly: true,
    sameSite: 'lax',
    secure: process.env.NODE_ENV === 'production',
    maxAge: 60 * 60 * 24 * 30
  });
}
```

Notice we re-apply our own security flags rather than verbatim-forwarding the backend's Set-Cookie header. That lets us swap `secure: false` in dev for `secure: true` in prod without the backend caring about the deployment target.

---

## C. Tests

### C.1 — Backend unit tests

`cargo test`: Argon2 round-trip; password length validation; malformed-PHC raises internal error; email normalize lowercases + trims + rejects malformed.

### C.2 — Playwright: the permission matrix

The single most important test in this project:

```ts
test("PERMISSION MATRIX: user A cannot read user B's contact", async ({ request }) => {
  const a = await apiRegister(request);
  const b = await apiRegister(request);
  const bContactId = await apiCreateContact(request, b.cookie, "Bob's Contact");

  // A tries to GET B's contact → 404 (NOT 403 — never confirms row exists)
  const res = await request.get(`${BACKEND}/api/contacts/${bContactId}`, {
    headers: { cookie: `contacts_session=${a.cookie}` }
  });
  expect(res.status()).toBe(404);

  // A tries to PATCH B's contact → 404
  // A tries to DELETE B's contact → 404
  // B can still see + delete their own contact
});
```

The shape of this test is what the curriculum's principal-engineer note flagged: *"if you can't list permissions you don't have an auth system."* For every `(role, action, resource)` triple in your app you should have an entry in this matrix. We currently have 1 role (owner). Project 23 (multi-tenant help desk) adds a real matrix with admin/agent/customer roles and parameterizes the tests.

### C.3 — Other Playwright coverage

- **No-auth 401 fan-out** — every protected endpoint asserts 401 without a cookie.
- **axe-core a11y** on `/login` and `/` (dashboard).
- **Wrong password** returns a generic 401 message (no enumeration).
- **Logout** clears the cookie + bounces back to `/login`.
- **CRUD round-trip via the UI** — register → add contact → search → see in list.

---

## D. Closing — what you can do now

- Stand up Postgres + a local mail catcher (MailHog) with one Docker Compose file.
- Build an auth system you'd ship: Argon2id, server-side sessions with hashed tokens, single-use email tokens, sliding expiry, HttpOnly+SameSite+Secure cookies.
- Apply no-enumeration discipline: timing-resistant login, generic responses on register/forgot/login.
- Use SvelteKit route groups + `hooks.server.ts` + `+layout.server.ts` guards to enforce auth in TWO files for the entire app.
- Forward cookies from your Node server to a cross-origin backend without leaking them anywhere they shouldn't go.
- Search text in Postgres at scale with `tsvector` + GIN + `plainto_tsquery` + `ts_rank`.
- Write permission-matrix tests that prove server-side scoping holds for every (role, action, resource) — the principal-engineer test pattern.

Project 12 — **Job Application Tracker + bcrypt legacy-migration lesson**. The auth foundation from this project carries over unchanged; we add a "we acquired a company and inherited their bcrypt hashes" scenario and the **dual-verify + re-hash-on-login pattern** every real-world auth system eventually needs.
