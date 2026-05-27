# Lesson — Project 16

The lesson here isn't "use the Stripe SDK". It's **what the Stripe SDK
does on your behalf, and what you must do yourself even if you use it**.

Three pieces of payment infrastructure that look easy but bite:

1. **Webhook signature verification** — what HMAC does, why constant-time
   compare matters, why timestamps protect against replay.
2. **Idempotency** — Stripe will deliver a webhook more than once, often.
   Your handler must be a no-op the second time.
3. **Signed delivery URLs** — how to give a customer a download link by
   email that expires, is revokable, and can't be forged.

## A. Backend

### A.1 The webhook signature module (`src/stripe/webhook.rs`)

```rust
pub fn verify(
    secret: &[u8],
    header_value: &str,
    body: &[u8],
    now_unix: i64,
) -> Result<(), SigError> {
    let (ts, sigs) = parse_header(header_value).ok_or(SigError::MalformedHeader)?;
    if sigs.is_empty() { return Err(SigError::MalformedHeader); }
    if (now_unix - ts).abs() > REPLAY_WINDOW_SECS {
        return Err(SigError::TimestampSkewed);
    }
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(ts.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    for sig_hex in sigs {
        if let Ok(decoded) = hex::decode(sig_hex)
            && bool::from(expected.ct_eq(&decoded)) { return Ok(()); }
    }
    Err(SigError::SignatureMismatch)
}
```

Line by line:

- **`now_unix: i64` is injectable**. The handler passes `webhook::now_unix()`;
  the tests pass any timestamp they like. This is the only honest way to
  test "is a 6-minute-old event rejected?" without `tokio::time::pause`
  shenanigans.
- **`(now_unix - ts).abs() > REPLAY_WINDOW_SECS`**. We use `abs()`
  because a future timestamp is just as suspicious as a stale one — both
  mean the clock you can't see has been tampered with. 5 minutes is what
  Stripe documents; we mirror it.
- **`mac.update(ts.to_string().as_bytes())` then `b"."` then `body`**.
  The literal scheme Stripe defines: `HMAC(secret, "{t}.{raw_body}")`.
  The `.to_string()` is the deliberate, painless way to avoid any custom
  itoa.
- **`ct_eq`**. This is the whole reason we pulled `subtle`. `==` on byte
  slices short-circuits as soon as the first byte differs; an attacker
  with a few thousand requests can use the timing of failure to recover
  the prefix of the HMAC. `ct_eq` always touches every byte.
- **The `for sig_hex in sigs` loop**. Stripe sends multiple `v1=` headers
  during secret rotation — you have to accept *any* one of them. Reject
  if none matches.

### A.2 The route (`src/routes/stripe.rs`)

The Axum extractor stack is the first interesting bit:

```rust
async fn handle_webhook(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: Bytes,           // <-- raw body extractor
) -> AppResult<impl IntoResponse> { ... }
```

`Bytes` is what `axum::body::Bytes` gives us — the raw bytes of the
request body **before any decoding**. If we used `Json<WebhookEvent>` we
would have lost the original bytes by the time we tried to verify the
signature: `serde_json` doesn't re-serialize to the exact same string,
so the HMAC wouldn't match. **The order matters: verify first, parse
second.**

The idempotency dance:

```rust
let inserted = sqlx::query!(
    r#"INSERT INTO stripe_events (event_id, type, payload)
       VALUES ($1, $2, $3) ON CONFLICT (event_id) DO NOTHING
       RETURNING event_id"#, ...
).fetch_optional(&s.pool).await?;
if inserted.is_none() {
    tracing::info!(event_id = %event.id, "duplicate webhook delivery — skipping");
    return Ok(StatusCode::OK);
}
```

`ON CONFLICT DO NOTHING ... RETURNING event_id` is the trick. The first
deliverer of the event wins the insert and gets a `Some(_)`. The second,
third, fourth deliverers (Stripe retries on 5xx, but it also occasionally
double-delivers on success) get `None` and bail. The unique constraint
on `event_id` is what guarantees this; the `RETURNING` is what tells us
who won.

Why `return Ok(StatusCode::OK)` and not 409? Stripe interprets non-2xx
as "retry me later" — exactly what we don't want. The duplicate IS our
desired outcome; 200 says "thanks, I have this".

### A.3 Signed download URLs (`src/signing.rs`)

The token format is `<nonce>.<hmac_b64>`. The nonce is 32 random bytes
base64url'd; the HMAC is over the nonce alone (we don't sign the expiry
because we trust the DB to enforce that — see below).

The hash stored in `download_links.token_hash` is `SHA-256(full_token)`,
NOT `SHA-256(nonce)`. So even if an attacker dumps the DB they can't
manufacture a token that hashes to a stored value: they'd need to know
the full `<nonce>.<sig>` string, which means they'd need to compute the
HMAC, which means they'd need the signing secret. **Defence in depth:
sign it AND store the hash, not one or the other.**

Why don't we sign the expiry into the token itself? Because expiry/
revocation is a server-side concern. If we ever need to invalidate a
link early (e.g. after a refund), we update `download_links.revoked_at`
in the DB. With a self-describing JWT-style token you'd be stuck with
the expiry baked in.

### A.4 Refund + revocation (`src/routes/admin.rs`)

```rust
let resp = s.stripe.refund_payment_intent(&pi, &idem).await?;
// ...
sqlx::query!("UPDATE orders SET status = 'refunded', refunded_at = now() WHERE id = $1", order.id)
    .execute(&mut *tx).await?;
sqlx::query!("UPDATE download_links SET revoked_at = now() WHERE order_id = $1 AND revoked_at IS NULL", order.id)
    .execute(&mut *tx).await?;
```

Two events should also flip status: the `charge.refunded` webhook *and*
the immediate UPDATE here. Both are fine because the UPDATE is
idempotent ("already refunded" stays refunded). The webhook is what
makes the system correct if an admin issues the refund from Stripe's
dashboard instead of ours.

The `Idempotency-Key` we pass to `POST /v1/refunds` is deterministic per
order (`rf_<order_id>`), so a double-click refunds once.

## B. Frontend

The lesson is "two audiences in one app". The storefront's `+page.svelte`
is a single email field plus a list of products with Buy buttons; no
account, no session. The admin's `+page.svelte` is gated behind
`onMount(() => authApi.me().catch(() => goto('/admin/login')))` — the
classic SPA-side guard that's fine here because the backend will
401 any admin call without a valid cookie anyway.

The price formatter is `Intl.NumberFormat` — never roll your own.
`try { } catch { fallback }` because some browsers will throw on
unrecognised currency codes.

## C. Tests

Three categories:

1. **Pure logic, no I/O** (in `src/`): `signing::tests`,
   `stripe::webhook::tests`, `auth::hash::tests`,
   `routes::download::tests::sanitise_strips_traversal`. These are the
   fastest tests in the project and run on every `cargo test`.

2. **Wiremock'd Stripe** (`tests/stripe_client.rs`): we never call the
   real API. The mock asserts the exact form bytes we send, including
   the URL-encoded nested arrays (`line_items%5B0%5D...`). This is the
   only way to catch "I changed the form encoding to JSON and forgot to
   tell anyone".

3. **DB-backed integration** (`tests/webhook_flow.rs`): we use a live
   Postgres at `DATABASE_URL`. We exercise the actual
   `stripe_events ON CONFLICT DO NOTHING` constraint and confirm that
   a duplicate insert returns `None`. We also walk a
   `products → orders → download_links` triangle to prove the lookup
   query the route uses actually returns a row.

## D. What you can do now

You can accept money on the internet, the boring correct way. That
means:

- A buyer never sees a card form you built. Stripe Checkout (or
  equivalent) is where card data lives.
- A webhook handler that won't double-charge, won't double-deliver,
  won't accept a tampered body, won't accept a replay, won't accept a
  spoofed secret.
- A delivery flow that emits a link only after Stripe confirms payment;
  that link is revokable; that link expires.
- A refund that revokes the download link in the same transaction.

What you can't do yet:

- **Subscriptions** — these have their own lifecycle (created, updated,
  past_due, canceled). That's project 21.
- **Production file delivery** — local disk under `./data/products/` is
  fine for the curriculum and for a small SaaS. For real volume you'd
  upload to S3/MinIO (project 15) and have the download endpoint emit a
  302 to a pre-signed S3 URL. The signed-URL pattern translates 1:1.
- **PCI compliance paperwork** — by never seeing card numbers you sit in
  PCI SAQ-A, the lightest tier. Don't change that.

## Deviations / accepted notes

- The integration tests assume `DATABASE_URL` points at a Postgres they
  can write to. If it isn't set, the DB-backed tests print a warning and
  return early (they don't fail). CI sets the variable explicitly.
- The CI environment used at ship time didn't have Docker — we ran
  against a system Postgres on `localhost:5432` instead of the
  docker-compose'd 5437. The schema and code are identical; only the
  connection URL changed.
- Customer email arrives at MailHog in dev. The `Mailer::send` swallows
  SMTP failures intentionally (logs only): losing a single delivery
  email shouldn't 500 the webhook, which would then make Stripe retry
  it, which would then re-run fulfilment.
