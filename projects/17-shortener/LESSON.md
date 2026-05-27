# Project 17 — Lessons

Five things this project teaches that the previous sixteen didn't:

## 1. The hot redirect path

`GET /{slug}` is the only route in this app that anonymous traffic will pound
on. Everything else is gated on a session cookie. We treat it like a hot
path:

```rust
// src/routes/redirect.rs — abbreviated
pub async fn redirect_to_target(State(s): State<AppState>, Path(slug): Path<String>, ...) {
    let row = sqlx::query!(...).fetch_optional(&s.pool).await?;

    if !bot {
        // Cheap, single-roundtrip increment. The cache, not the DB, is the
        // hot counter.
        if let Some(pool) = s.redis.clone() {
            tokio::spawn(async move { /* INCR clicks:<slug>:total */ });
        }
    }

    // Click row insert runs AFTER the response goes out. We trade a tiny
    // durability window (process death between response and insert) for a
    // measurably faster TTFB on the redirect.
    tokio::spawn(async move {
        let _ = sqlx::query!("INSERT INTO clicks ...").execute(&pool).await;
    });

    Redirect::temporary(&row.target_url).into_response()
}
```

The pattern is: **respond first, persist analytics after**. The user is
already on their way to `target_url`; we don't need them waiting for the
click row's COMMIT before we hand them the 307.

If the process dies between the response and the spawn, you lose at most
one click row — but the Redis counter has already been INCR'd, and the DB
counter is "the durable source of truth at next aggregation". You can
periodically reconcile the two; we don't, because it's a learning project.

## 2. Bot filtering at the boundary

Every short-link service eventually drowns in scraper noise. We hand-rolled
a deliberately small regex:

```rust
Regex::new(r"^(Googlebot|Bingbot|YandexBot|DuckDuckBot|curl|wget|Python-urllib|HTTPie)")
```

It's not exhaustive — production should use `ua-parser` with the upstream
regex DB. But the lesson is *where* to filter. Two specific calls:

1. Filter the analytics, not the redirect. Bots still get a 307 because
   breaking them outright would break legitimate previewers (Slack /
   Twitter unfurl, etc.).
2. The DB still gets a click row, but with `is_bot = TRUE`. That way you can
   audit later — "how much of last month was bots?" — and the aggregate
   queries (`stats_today`, etc.) filter `WHERE is_bot = FALSE`.

## 3. Redis as a counter cache

Postgres can count rows. So why Redis?

```rust
// INCR is one redis roundtrip — typically <1ms.
deadpool_redis::redis::cmd("INCR").arg(key).query_async(&mut conn).await;
```

A row INSERT, even with `tokio::spawn`, is heavier: a connection from the
pool, the actual INSERT, an index update on `idx_clicks_link_time`, and a
COMMIT. The Redis counter gives the user the *immediate* "your link just
ticked to 412" feedback without hitting the DB at all.

**Graceful degradation matters here.** `REDIS_URL` is optional. At startup,
we try to ping; on failure (env unset, connection refused, anything) we set
`state.redis = None` and the rest of the code falls back. The redirect path
just skips the INCR; the stats endpoint shows `redis_counter: null`.

```rust
let redis = match std::env::var("REDIS_URL").ok() {
    Some(url) if !url.is_empty() => match build_redis_pool(&url).await {
        Ok(pool) => Some(pool),
        Err(e) => { tracing::warn!(error = %e, "redis unavailable"); None }
    },
    _ => None,
};
```

The lesson: an optional cache should be an `Option<Pool>` end-to-end. Don't
let a missing cache wedge your app — but also don't paper over it with a
fake counter; let the UI honestly say "no cache available".

## 4. TOTP step-up flow

The hard part of 2FA isn't generating codes — `totp-rs` handles that. The
hard part is the **login state machine**:

```text
POST /api/auth/login {email, password}
  ├── 2FA off:   issues session cookie, returns UserPublic
  └── 2FA on:    returns {requires_2fa: true, intermediate: <token>}
                 ↓
        POST /api/auth/2fa/verify {intermediate, code}
          └── on success: issues session cookie, returns UserPublic
```

The intermediate token is a regular `auth_tokens` row with
`kind = 'totp_intermediate'` and a 5-minute TTL. It's single-use (consumed
on the verify call) and lives outside the session cookie so an attacker
who steals it can't ride it without the second factor.

**Backup codes are Argon2-hashed at issuance.** 10 hex codes (12 chars each,
shown grouped as `xxx-xxx`) are generated and only the hash hits the DB. On
verify, we iterate the user's unused codes (N ≤ 10, fine for a linear scan)
and `verify_password` each. The matching row's `used_at` flips. Single-use.

```rust
for c in codes {
    if hash::verify_password(&normalized, &c.code_hash)? {
        sqlx::query!("UPDATE backup_codes SET used_at = now() WHERE ...").execute(&s.pool).await?;
        return Ok(true);
    }
}
```

## 5. Per-link scoping returns 404, not 403

Look at the stats handler:

```rust
let link = sqlx::query!(
    r#"SELECT id, slug, target_url, created_at
       FROM links
       WHERE slug = $1 AND user_id = $2"#, slug, user.id,
)
.fetch_optional(&s.pool).await?
.ok_or(AppError::NotFound)?;
```

Two things going on:

1. The `WHERE user_id = $2` clause is what enforces the scope.
2. If the slug exists for *some other user*, the optional is `None` and we
   return 404 — the same response you'd get for a slug that doesn't exist
   at all.

Returning 403 ("forbidden") here would leak that the slug exists. Some
attacker iterating dictionary-likely slugs (`/login`, `/admin`, `/promo`)
would learn which ones are someone else's. 404 makes that probe useless.

## Wiring detail worth calling out

Top-level `/{slug}` MUST be the LAST route registered in axum, otherwise
`/api/auth/login` would match the slug pattern. We nest `/api/*` first,
register `/healthz`, then `.route("/{slug}", ...)`. Order matters.
