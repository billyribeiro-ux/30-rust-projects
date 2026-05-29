# Lesson — Project 25 (Course Marketplace, Stripe Connect)

Three things this project teaches that no earlier project did:

1. **Stripe Connect Express accounts** — how the platform delegates KYC
   to Stripe and what changes in the API surface vs the standalone
   Checkout flow.
2. **Destination charges with `application_fee_amount`** — the one-
   PaymentIntent shape that takes a platform cut and sends the rest
   to the instructor.
3. **`reverse_transfer + refund_application_fee`** — making both sides
   whole on a refund, atomically.

---

## A. Backend

### A1. Express account creation

```rust
let body: Vec<(&str, String)> = vec![
    ("type", "express".into()),
    ("email", email.into()),
    ("country", country.into()),
    ("capabilities[card_payments][requested]", "true".into()),
    ("capabilities[transfers][requested]", "true".into()),
];
let res = self.http
    .post(format!("{}/v1/accounts", self.base_url))
    .basic_auth(&self.secret_key, Some(""))
    .header("Idempotency-Key", Uuid::new_v4().to_string())
    .form(&body)
    .send()
    .await?;
```

- **`type=express`** — Stripe runs the KYC UI on their domain. We get
  a hosted onboarding flow without writing a single form. Tradeoff:
  less branding control.
- **Capabilities** — Stripe used to enable everything by default. Now
  we have to ask. `card_payments` lets the connected account be
  charged; `transfers` lets the platform send money to them. Without
  both, your destination charges fail.

### A2. Account-link onboarding (the bouncing URL)

```rust
("return_url", return_url.into()),     // we come back here on success
("refresh_url", refresh_url.into()),   // we come back here if the link expires
```

The URL Stripe returns is one-shot. If the instructor closes the tab
and comes back tomorrow, we mint a new one. Don't store the URL.
That's the whole pattern.

### A3. The destination-charge Checkout

```rust
("payment_intent_data[application_fee_amount]", platform_fee_cents.to_string()),
("payment_intent_data[transfer_data][destination]", destination_account_id.into()),
```

Two fields, one PaymentIntent. Stripe:
1. Authorises the card for the full price.
2. On capture, transfers `(price − fee)` to the instructor's account.
3. Keeps `fee` on the platform balance.

You don't have to do two-step transfers, you don't have to manage
balance reconciliation between platform and instructor, and the
instructor is in your books for the right amount automatically.

We compute the fee with **basis points** so floating-point is never
involved:

```rust
let fee = course.price_cents * s.platform_fee_bps / 10_000;
```

A `BIGINT` price (`i64`) multiplied by an `i64` basis-points value
stays in `i64` until the divide. No rounding surprises.

### A4. The refund with reverse-transfer

```rust
("payment_intent", pi_id.into()),
("reverse_transfer", "true".into()),
("refund_application_fee", "true".into()),
```

- **`reverse_transfer=true`** — the instructor's `(price − fee)` is
  pulled back from their connected account onto the platform.
- **`refund_application_fee=true`** — the platform's `fee` is returned
  to the customer too.

Without either flag, the refund is partial in a confusing way. With
both, the world is restored to the pre-purchase state. We test that
the refund endpoint returns 204 and that `enrollments.refunded_at` is
non-NULL.

### A5. The `account.updated` reaction

```rust
"account.updated" => on_account_updated(&s, &event.data.object).await?,
```

Stripe sends this when the instructor's KYC status changes — onboarding
complete, missing documents, etc. We mirror only what we need:
`payouts_enabled` and `details_submitted`. Tested.

The `checkout/courses/:slug` route refuses to start a session unless
`payouts_enabled = true`. Without that check, an instructor could be
charged for a course we couldn't pay them for.

### A6. The 14-day refund window

```rust
let age = chrono::Utc::now() - row.created_at;
if age > chrono::Duration::days(14) {
    return Err(AppError::Forbidden);
}
```

The window is the *server's* rule. The UI hint ("Refundable within 14
days") is a courtesy. A determined customer who edits a hidden form
field still gets a 403. Defence in depth: the rule lives in the
backend, not in `<input type="hidden">`.

---

## B. Frontend

### B1. The buy form

```svelte
<form method="POST" action="?/buy" use:enhance={...}>
  <button type="submit" class="primary" disabled={submitting}>
    {submitting ? 'Opening Stripe…' : 'Buy course'}
  </button>
</form>
```

The `+page.server.ts` action POSTs to the backend's
`/api/enroll/checkout/:slug`, gets a Stripe-hosted URL back, and
`redirect(303, out.checkout_url)`. The student leaves our site to
pay; comes back via `success_url`. The standard pattern.

### B2. `enrolled` flag from the API

The course-detail response includes `enrolled: bool` resolved from
the session cookie. The UI swaps the buy form for "You're enrolled."
when it's true. The signal is server-rendered, so no flash of buy
button.

---

## C. Tests

### C1. `cargo test` (5)

- Argon2 round-trip
- `connect_flow::account_updated_flips_payouts_enabled` — webhook → DB
- `connect_flow::checkout_completed_creates_enrollment_and_is_idempotent`
  — fulfilment + replay safety
- `connect_flow::signature_mismatch_returns_401` — webhook security

### C2. `pnpm test:unit` (4)

`coursesApi.list/create`, `enrollApi.checkout` URL-encoding, non-2xx
→ ApiCallError.

### C3. `pnpm test:e2e` (3 × 4 = 12)

Axe-clean homepage + login. Unknown slug → 404.

---

## D. What you can do now

1. Stand up a marketplace where the platform never holds the
   instructor's money on its balance sheet.
2. Pay out automatically — Stripe's payout schedule applies to the
   connected account.
3. Refund cleanly with both halves of the transaction made whole.

Project 26 — Live Coding Interview Platform. CRDT (yrs) + SAML SSO.
