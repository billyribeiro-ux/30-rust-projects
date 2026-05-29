# Lesson — Project 20 (Geo-aware Finder + OAuth)

Three things this project teaches that no earlier project did:

1. **OAuth 2.0 Authorization Code + PKCE**, end to end. State, verifier,
   challenge, nonce. Hand-rolled because a library would hide every
   field that the spec was built around.
2. **PostGIS**: GEOGRAPHY columns, GiST indexes, ST_DWithin /
   ST_Distance — the mechanics behind every "near me" search.
3. **A `TEST_ONLY_TOKEN`-gated session-injection route**: the only sane
   way to write e2e for an OAuth-or-magic-link product without driving
   a real provider in CI.

---

## A. Backend

### A1. The OAuth flow, on the wire

```
┌───────────┐  /start                        ┌──────────────┐
│  Browser  │ ────────────────────────────▶ │  finder API   │
└───────────┘                                └──────────────┘
       ▲ 303 Redirect ?response_type=code           │
       │ &client_id=… &state=S                      │  sets cookies:
       │ &code_challenge=C &nonce=N                 │    oauth_state=S
       │                                            │    oauth_verifier=V (V → C = SHA256+base64url)
       │                                            │    oauth_nonce=N
       │                                            │    oauth_provider=google
       │
┌───────────┐  /authorize                  ┌──────────────────┐
│  Browser  │ ──────────────────────────▶ │  Google IdP      │
└───────────┘                              └──────────────────┘
       ▲ 303 Redirect to /callback?code=AUTHCODE&state=S      │
       │                                                       │
┌───────────┐  /callback?code=…&state=…                ┌──────────────┐
│  Browser  │ ────────────────────────────────────▶ │  finder API   │
└───────────┘                                            └──────────────┘
                                                             │  1. state == cookie? (constant-time)
                                                             │  2. POST /token with grant_type=authorization_code,
                                                             │     code, code_verifier=V (PKCE)
                                                             │  3. id_token nonce == oauth_nonce? (Google)
                                                             │  4. GET /userinfo with access_token
                                                             │  5. upsert (user + oauth_account); create session
                                                             ▼  303 to OAUTH_SUCCESS_REDIRECT (set-cookie: app_session=…)
```

Every defence is doing real work:

- **`state`** — CSRF defence. Without it, an attacker could trick a
  signed-in user into completing a callback they didn't initiate. We
  generate 32 bytes of random, stash them in an HttpOnly cookie scoped
  to `/api/auth/oauth`, and constant-time-compare on callback.
- **PKCE `code_verifier` / `code_challenge`** — Replaces the client
  secret in public clients (SPAs / mobile). Even though we *have* a
  client secret, PKCE adds defence: someone who intercepts the
  authorization code can't redeem it without the verifier, which lives
  only in our cookie. We use S256 (`base64url(sha256(verifier))`); the
  spec also allows `plain`, which is broken.
- **`nonce`** (Google only) — Embedded in the `id_token` the IdP returns.
  We verify it on callback so a stolen `id_token` from someone else's
  session can't be replayed against us.
- **Cookies are `SameSite=Lax`** (not `Strict`) — they need to be sent
  on the IdP's 302 back to our callback. Strict would block them.

### A2. PostGIS — the queries that matter

```sql
SELECT
  p.id, p.name, p.cuisine, p.address,
  ST_Y(p.geom::geometry) AS lat,
  ST_X(p.geom::geometry) AS lng,
  ST_Distance(p.geom, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) AS distance_m,
  …
FROM places p
WHERE ST_DWithin(
        p.geom,
        ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
        $3
      )
ORDER BY p.geom <-> ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
LIMIT 100;
```

- **`GEOGRAPHY(Point, 4326)`** is the spheroidal type. Use this, not
  `GEOMETRY`, when distance must be in real metres on a real Earth.
  GEOMETRY is faster, but distances are in degrees on a 2D plane —
  fine for tiles, wrong for "within 2 km of me".
- **`ST_MakePoint(lng, lat)`** — lng is X, lat is Y. Swap them once and
  every result is in the wrong hemisphere. Note the order.
- **`ST_DWithin(a, b, m)`** filters; **`<->` operator** orders by
  distance using the GiST index. Without `<->` you'd compute distance
  to every matching row and sort in memory.
- **`ST_SetSRID(...)` + `::geography`** — `ST_MakePoint` produces a
  generic geometry without SRID; we set 4326 (WGS-84) and cast to
  geography so the units match the column.

### A3. The `test_only` route

```rust
async fn login_as(
    State(s): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(input): Json<LoginAsInput>,
) -> AppResult<impl IntoResponse> {
    let Ok(expected) = std::env::var("TEST_ONLY_TOKEN") else {
        return Ok((StatusCode::NOT_FOUND, jar, "").into_response());
    };
    if headers.get("x-test-token").map(|h| h.as_bytes()).unwrap_or_default() != expected.as_bytes() {
        return Ok((StatusCode::UNAUTHORIZED, jar, "").into_response());
    }
    // … look up or auto-create user by email, create a session, set cookie.
}
```

Why this isn't a backdoor:

- **Default-closed**. With `TEST_ONLY_TOKEN` unset (the production
  default), every request returns **404**, indistinguishable from a
  typo on the URL. There's no version of "I forgot a flag in prod" that
  unlocks this.
- **Header-gated**. Even with the env var set, the request must carry
  `x-test-token: <secret>`. We don't ship that header in real client code.
- **Logged via tracing** when used (in a real deployment you'd also
  alarm on it). Tests run hot, prod runs cold.

Without something like this, the e2e test "signed-in user sees their
name" has to drive a real OAuth provider — flaky, requires a real
account, fights with rate limits. With it, the test is one POST.

---

## B. Frontend

### B1. SSR-first JSON-LD

The homepage is server-rendered with full `Restaurant` + `ItemList`
JSON-LD:

```ts
const jsonLd = $derived(JSON.stringify({
  '@context': 'https://schema.org',
  '@type': 'ItemList',
  itemListElement: data.places.slice(0, 20).map((p, i) => ({
    '@type': 'ListItem',
    position: i + 1,
    item: {
      '@type': 'Restaurant',
      name: p.name,
      address: p.address,
      servesCuisine: p.cuisine,
      geo: { '@type': 'GeoCoordinates', latitude: p.lat, longitude: p.lng },
      aggregateRating: p.avg_rating
        ? { '@type': 'AggregateRating', ratingValue: p.avg_rating, reviewCount: p.review_count }
        : undefined
    }
  }))
}));
```

…rendered into `<svelte:head>` with `{@html '<script type="application/ld+json">…</script>'}`.
Crawlers see the structured data before any JS runs.

The e2e suite asserts on it (`script[type="application/ld+json"]`),
because "we added SEO" is the kind of claim that quietly regresses if
no test holds the line.

### B2. Three documented `state_referenced_locally` silences

- `let q = $state(data.q)` — seed once; the form submits via GET, so a
  new page load brings fresh data.
- `let cuisine = $state(data.cuisine)` — same reason.
- `const place = data.place` on the detail page — pure read-through.

In every case the warning is correct in the general case ("you might be
expecting reactivity that won't happen") and wrong for this specific
intention (we *want* a single snapshot). The comment documents intent;
without it the next reader would either ignore the warning blindly or
"fix" it into a behavioural change.

### B3. Geolocation with a polite degradation

```ts
function askLocation() {
  geoStatus = 'asking';
  navigator.geolocation.getCurrentPosition(
    (pos) => { … reload with lat/lng … },
    () => { geoStatus = 'denied'; },
    { timeout: 8000 }
  );
}
```

When the user denies (or the browser is on a non-secure origin and
geolocation is blocked), we surface a friendly hint with canned cities
(`?lat=…&lng=…`). The product doesn't break; the search just doesn't
have "near me" as a constraint.

---

## C. Tests

### C1. Backend (`cargo test`, 9 tests)

- Argon2 round-trip (round_trip_verifies, rejects_short_password)
- email lowercase / trim / reject malformed
- PKCE: `google_authorize_url_includes_pkce_state_nonce`
- PKCE: `github_authorize_url_includes_pkce_state_no_nonce`
  (GitHub doesn't support OIDC, no nonce field)

Future expansions (wiremock'd /token + /userinfo for full callback)
are left as a hand-rolled follow-up; the autofixer + clippy gate kept
the surface clean as we shipped.

### C2. Frontend (`pnpm test:unit`, 6 tests)

- `placesApi.list` serializes lat/lng/radius/q correctly + omits absent
- `placesApi.get` URL-encodes the id
- `reviewsApi.upsert` POST body shape
- `authApi.oauthStart` returns absolute URL
- non-2xx maps to `ApiCallError`

### C3. E2E (`pnpm test:e2e`, 5 × 4 = 20 runs)

- Homepage axe-clean
- JSON-LD ItemList is present
- Search query reflected in URL (`waitForURL` to avoid networkidle race)
- Login page exposes OAuth buttons + axe-clean
- Signed-in user (via `TEST_ONLY_TOKEN`) sees their name + can log out

---

## D. What you can do now

1. Write OAuth login that you understand at the protocol level.
2. Reason about PKCE, state, and nonce as distinct defences.
3. Ship a geo-aware product without writing a service to do what
   PostGIS does in one operator.
4. Make e2e for OAuth-only products tractable via a default-closed
   test-injection route.

Project 21 — **Stripe Subscriptions**, the second of four Stripe
ladders. Mirroring Stripe state locally, webhook idempotency, dunning,
and proration on upgrade.
