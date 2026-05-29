# Lesson — Project 21 (Newsletter, Stripe Subscriptions)

Four things this project teaches that no earlier project did:

1. **Mirror Stripe state locally**, and read from the mirror. Network
   calls to Stripe are eventually consistent and rate-limited; your
   "is this person Pro?" check has to be a single Postgres lookup.
2. **The dunning state machine**: active → past_due → grace → downgrade.
3. **Two cookie-scoped session systems** in one app. The author and
   the subscriber are *different security principals* with different
   blast radii — they don't share a session table.
4. **Article JSON-LD with `isAccessibleForFree: false`** so Google
   surfaces the paywall correctly and doesn't ban your site for
   cloaking when it sees a full body to Googlebot and a teaser to
   users.

---

## A. Backend

### A1. Why the local mirror

Pseudocode for "render this post":

```rust
let is_pro = sqlx::query_scalar!(
    "SELECT plan = 'pro' FROM subscriptions WHERE subscriber_id = $1",
    sub.id,
).fetch_optional(pool).await?.flatten().unwrap_or(false);
```

If we instead called Stripe's `GET /v1/subscriptions/:id` here:

- Every gated-post read costs a 200ms+ round trip.
- Stripe rate-limits at ~25 req/s/account; one popular post pegs it.
- During Stripe partial outages, your *paywall* breaks instead of just
  your *billing*.

The mirror is the right design. The webhook handler is the only writer;
it logs every state change. Reconciliation (a periodic pull from Stripe
to fix drift) is one cron task we don't ship here but should in any
real product — call it out and move on.

### A2. The dunning state machine

```
                    ┌──────────────────┐
                    │   incomplete     │  fresh checkout, no payment yet
                    └────────┬─────────┘
              checkout.session.completed
                             ▼
                    ┌──────────────────┐
              ┌───▶ │     active       │ ◀─── invoice.paid
              │     └────────┬─────────┘
              │              │ invoice.payment_failed
              │              ▼
              │     ┌──────────────────┐
              │     │   past_due       │  past_due_since = now()
              │     └────────┬─────────┘
              │              │ 14 days elapse
              │              ▼
              │     ┌──────────────────┐
              │     │ canceled (free)  │  reaper task downgrades
              │     └──────────────────┘
              │
              └─── invoice.paid (within grace window)
```

The grace-period query is one SQL:

```sql
SELECT subscriber_id FROM subscriptions
WHERE status = 'past_due'
  AND past_due_since IS NOT NULL
  AND past_due_since < now() - INTERVAL '14 days'
```

Tested explicitly in `webhook_flow.rs::dunning_downgrade_after_grace_period`.
A `cron` (or simply a tokio task at startup) loops through these and
calls `downgrade_to_free(subscriber_id)`. The reaper isn't running in
the binary by default — production deployments would add a `--worker`
flag; we wire it in project 22 properly.

### A3. Webhook security (again, but with rotation)

```rust
pub fn verify(raw_body: &[u8], sig_header: &str, secret: &str, now_unix: i64)
    -> Result<(), VerifyError>
```

Same shape as project 16, plus:

- **Accepts multiple `v1=`** values in the header. When Stripe rotates
  a webhook secret you get a window where both old and new arrive
  signed; if you reject "more than one v1=", you drop legitimate events.
  Tested by `accepts_multiple_v1_for_rotation`.
- **Constant-time compare across all candidates** (we don't early-return
  on first mismatch). This keeps timing flat regardless of which secret
  matches.

### A4. Two sessions, two cookies

```
SESSION_COOKIE_NAME    = "app_session"   ← author/admin, full CRUD
SUBSCRIBER_COOKIE_NAME = "sub_session"   ← subscriber, magic link only
```

The author session table is `sessions`, the subscriber session table is
`subscriber_sessions`. Sharing one table means: any bug that lets a
subscriber session be looked up by the author extractor (or vice versa)
becomes a privilege escalation. Two tables, two cookies, two
extractors. The blast radius is correct.

---

## B. Frontend

### B1. The paywall + JSON-LD

`Article` JSON-LD with `isAccessibleForFree: false` plus a
`hasPart.cssSelector` tells Google: "this article exists and here's
the bit you should not crawl as if it were public." Without it,
Google sees a 600-char teaser and ranks you as a thin-content site.
With it, you keep your SEO weight *and* the paywall.

```jsonc
{
  "@type": "Article",
  "headline": "...",
  "isAccessibleForFree": false,
  "hasPart": {
    "@type": "WebPageElement",
    "isAccessibleForFree": false,
    "cssSelector": ".paywalled-body"
  }
}
```

We emit this on every post page (gated or not) — the
`isAccessibleForFree` field is the load-bearing signal.

### B2. Why HTTP 402 from `/api/posts/:slug`

The backend returns the teaser body with status **402 Payment Required**
when the visitor isn't Pro. The frontend treats 402 specially:

```ts
if (res.status === 402 && body && typeof body === 'object' && 'paywalled' in body) {
  return body as T;  // not an error — the teaser is the response
}
```

This is the right HTTP code for "you got something, but it's a teaser
because you didn't pay." 401/403 would be wrong (you're not
*forbidden*, you're *not entitled*) and 200 would hide the state from
caching layers.

### B3. The subscribe form returns one of two things

The action is a `+page.server.ts` form action:

```ts
const out = await subscribeApi.start(fetch, email, plan);
if (out.kind === 'pro') {
  redirect(303, out.checkout_url);   // off to Stripe Checkout
}
// free plan — also send a magic link so they can read.
await subscribeApi.magicStart(fetch, email).catch(() => null);
return { message: 'Subscribed. Check your inbox for a sign-in link.' };
```

Note the `catch(() => null)`: the magic-link send is best-effort. If
SMTP is down we still want the subscription to be considered created.
The user can request another magic link from `/login` once SMTP recovers.

---

## C. Tests

### C1. Backend (`cargo test`, 14 tests)

`tests/webhook_flow.rs`:

- `checkout_completed_creates_pro_subscription_and_is_idempotent`
  Proves the headline flow + replay → no double-insert.
- `payment_failed_marks_past_due_and_invoice_paid_recovers`
  Full state-machine round trip without the reaper.
- `dunning_downgrade_after_grace_period`
  Inserts a 15-day-old `past_due`, runs `find_past_due_to_downgrade`,
  asserts the id is returned, then downgrades — proves the SQL math.
- `webhook_signature_mismatch_is_401`
  Boundary test: a wrong-secret signature gets a clean 401, no crash.

`src/stripe/webhook.rs` unit tests:
- round-trip, replay-rejected, tampered-body, wrong-secret, key-rotation
  (5 tests).

`src/auth/hash.rs`: round-trip + min-length (2 tests).
`src/routes/auth.rs`: email lowercase/reject (2 tests).
`src/routes/posts.rs`: ammonia strips `<script>` (1 test).

### C2. Frontend (`pnpm test:unit`, 5 tests)

- List parses, slug URL-encoded, 402 returns body (not throws),
  subscribe body shape, non-2xx → `ApiCallError`.

### C3. E2E (`pnpm test:e2e`, 4 × 4 = 16 runs)

- Homepage axe-clean
- RSS `<link rel="alternate">` present
- Free-plan subscribe returns "check your inbox"
- Unknown slug returns 404

---

## D. What you can do now

1. Charge for content with Stripe Subscriptions — the boring correct
   way, with a local mirror, idempotent webhook, and dunning.
2. Reason about your auth in terms of *principals* (author vs
   subscriber), not just sessions.
3. Pair a paywall with structured data so SEO doesn't punish you for it.

Project 22 — **Background Jobs Dashboard**. The reaper we mentioned
becomes a first-class worker tier with `SKIP LOCKED`, retries, dead
letters, and OTLP tracing.
