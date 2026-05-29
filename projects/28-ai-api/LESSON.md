# Lesson — Project 28 (AI Inference API)

Three things this project teaches that no earlier project did:

1. **Hashing API keys at rest** — SHA-256 with a high-entropy random
   secret, why that's enough (no salt, no Argon2) and how to look them
   up in O(index).
2. **Per-key RPM gating** as one SQL count.
3. **WebAuthn / Passkeys** — the four-call ceremony, why state lives
   on the server, and how `webauthn-rs` smooths the spec.

---

## A. Backend

### A1. API key generation

```rust
let mut bytes = [0u8; 32];
OsRng.fill_bytes(&mut bytes);
let secret_body = URL_SAFE_NO_PAD.encode(bytes);
let secret = format!("sk_live_{secret_body}");
let prefix = secret_body.chars().take(8).collect::<String>();
let hash = Sha256::digest(secret.as_bytes()).to_vec();
```

- **32 bytes** of OS randomness — 256 bits of entropy, well above the
  rule-of-thumb "no brute force possible at internet timescales."
- **base64url-no-pad** — URL-safe characters only; no padding so the
  length is fixed (43 chars).
- **`sk_live_` prefix** — Stripe / OpenAI convention. The prefix is
  not secret; it just makes leaked keys easier to recognise.
- **Prefix (first 8 chars)** is stored in the clear in `api_keys.prefix`.
  The dashboard renders `abcd1234…` so users can tell their keys apart
  without ever seeing the secret.

### A2. Why SHA-256 and not Argon2

Password hashes use Argon2 because passwords are *low entropy* —
users type "Summer2026!" which a GPU can brute-force without a
slow hash. **Random 32-byte secrets are high entropy** — there is no
brute-force shortcut, so the slowness of Argon2 buys nothing and
costs every authenticated request a CPU spike.

`SHA-256(secret)` is collision-resistant and constant-time relative
to the secret length. It's also indexable: we have a UNIQUE index on
`api_keys.hash`, so authentication is one B-tree lookup, not a
table scan.

### A3. The authenticate function

```rust
pub async fn authenticate(pool: &PgPool, raw_header: &str)
    -> AppResult<(Uuid, Uuid, i32)>
{
    let secret = raw_header.strip_prefix("Bearer ").ok_or(Unauthorized)?;
    let hash = Sha256::digest(secret.as_bytes()).to_vec();
    let row = sqlx::query!(
        r#"SELECT id, user_id, rpm_limit, revoked_at
           FROM api_keys WHERE hash = $1"#,
        hash
    ).fetch_optional(pool).await?.ok_or(Unauthorized)?;
    if row.revoked_at.is_some() { return Err(Unauthorized); }
    // RPM gate.
    let count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM usage_events
           WHERE api_key_id = $1 AND ts > now() - INTERVAL '1 minute'"#,
        row.id
    ).fetch_one(pool).await?;
    if count >= row.rpm_limit as i64 {
        return Err(Forbidden);
    }
    Ok((row.id, row.user_id, row.rpm_limit))
}
```

Two queries, both indexed:

- `api_keys.hash` UNIQUE → B-tree lookup.
- `usage_events.api_key_id, ts DESC` → range scan bounded by the last
  minute.

At 1000 RPS / key the second query still runs in single-digit ms.

The **constant-time compare** is not necessary here because we're
hashing the input before comparing to a stored hash — even if SHA-256
takes variable time on different inputs (it doesn't, but pretend), an
attacker can't probe-and-time their way to the correct hash without
already knowing the input.

### A4. WebAuthn flow

```
Browser                                       Backend
   │  POST /register/start {email}             │
   ├──────────────────────────────────────────▶│
   │                                          │ Build PasskeyRegistration
   │                                          │ Insert webauthn_states row
   │      {state_id, challenge: CCR}          │
   │◀──────────────────────────────────────────┤
   │  navigator.credentials.create(CCR)        │
   │  (user touches fingerprint sensor)        │
   │  POST /register/finish {state_id, cred}   │
   ├──────────────────────────────────────────▶│
   │                                          │ DELETE + RETURNING from webauthn_states
   │                                          │ webauthn.finish_passkey_registration
   │                                          │ INSERT into webauthn_credentials
   │                                          │ Create session
   │      201 + set-cookie                     │
   │◀──────────────────────────────────────────┤
```

Why state lives **server-side**:

- `PasskeyRegistration` contains the challenge bytes and policy state.
  If we sent it to the browser and asked it back, an attacker who
  intercepts the round-trip could replay an old challenge against a
  new credential.
- Server-side state is **single-use** (`DELETE … RETURNING`). Once
  finish-step consumes it, it's gone. A replay sees 401.

`webauthn-rs` handles the cryptographic details (attestation
verification, signature validation, counter checks). We hand it the
output of `navigator.credentials.create()` (a JSON blob) and the
`PasskeyRegistration` state; it returns a `Passkey` we persist.

### A5. The `webauthn_states` cleanup

A future cron task should DELETE rows where `expires_at < now()`.
The TTL (5–10 min) is enforced by the `expires_at > now()` check in
finish-step queries, so an expired row is functionally dead even
without cleanup. Memory pressure motivates the cleanup, not security.

### A6. Stripe Meter Events (the unshipped piece)

The `usage_events` schema is *complete*:

```sql
CREATE TABLE usage_events (
    id BIGSERIAL PRIMARY KEY,
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    ts TIMESTAMPTZ NOT NULL DEFAULT now(),
    units INT NOT NULL DEFAULT 1,
    reported_to_stripe_at TIMESTAMPTZ
);
CREATE INDEX idx_usage_unreported ON usage_events (reported_to_stripe_at)
    WHERE reported_to_stripe_at IS NULL;
```

The **partial index on `reported_to_stripe_at IS NULL`** means the
reconciler's query —

```sql
SELECT id, api_key_id, units, ts
FROM usage_events
WHERE reported_to_stripe_at IS NULL
ORDER BY ts LIMIT 1000;
```

— is an O(index) scan against pending rows only, regardless of how
big the table grows in absolute terms.

The reconciler implementation is a tokio task identical in shape to
project 22's worker. COMMANDS.md §7 has the skeleton. Stripe's
endpoint is `POST /v1/billing/meter_events`; idempotency key is
`usage-<row.id>`.

---

## B. Frontend

### B1. One-time secret reveal

```svelte
{#if createdSecret}
  <aside class="reveal" role="status">
    <p><strong>One-time secret:</strong></p>
    <code class="secret">{createdSecret.secret}</code>
    <p class="note">Copy this now — we won't show it again.</p>
  </aside>
{/if}
```

`createdSecret` is derived from `form` (the SvelteKit action return)
— it's only present on the request that created the key. Navigate
away and back, and it's gone. The pattern matches Stripe, OpenAI,
GitHub's tokens, etc.

### B2. The `prefix` is what we show after that

The list view renders `abcd1234…` in a monospaced font next to the
key name. Users with three keys can tell them apart; an attacker
who screenshots the page learns the prefix (useful for nothing —
secrets are 43 chars).

---

## C. Tests

### C1. `cargo test` (3)

- `auth::hash::tests::round_trip` (Argon2)
- `inference_flow::create_key_and_call_inference_records_usage` —
  the full happy path + revoke → 401.
- `inference_flow::rpm_limit_returns_403_after_threshold` — RPM gate.

### C2. `pnpm test:unit` (5)

`keysApi.list/create/revoke`, `usageApi.summary`, non-2xx →
ApiCallError.

### C3. `pnpm test:e2e` (3 × 4 = 12)

Unauthenticated redirect, login axe-clean, inference 401 without key.

---

## D. What you can do now

1. Issue API keys that can't be reconstructed from the database.
2. Rate-limit at the API-key level without a separate rate-limit service.
3. Wire passkeys into your dashboard without writing a SAML library.

Project 29 — Cinematic Portfolio + GSAP #3.
