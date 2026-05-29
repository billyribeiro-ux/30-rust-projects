# Lesson — Project 24 (Hybrid Search KB)

Three things this project teaches that no earlier project did:

1. **Weighted `tsvector` GIN** with a trigger — so search ranking
   knows the title matters more than the body, and the app code can
   never forget to refresh the index column.
2. **`pg_trgm` similarity** as the typo-tolerance layer that gets you
   80% of Algolia for one CREATE EXTENSION.
3. **Reciprocal Rank Fusion** — how to combine two rankers (FTS and
   Meili) without per-query tuning.

---

## A. Backend

### A1. The weighted index

```sql
CREATE OR REPLACE FUNCTION articles_tsv_update() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
  NEW.tsv :=
      setweight(to_tsvector('english', coalesce(NEW.title, '')),   'A') ||
      setweight(to_tsvector('english', coalesce(NEW.summary, '')), 'B') ||
      setweight(to_tsvector('english', coalesce(NEW.body_md, '')), 'C');
  RETURN NEW;
END
$$;
CREATE TRIGGER trg_articles_tsv
BEFORE INSERT OR UPDATE OF title, summary, body_md ON articles
FOR EACH ROW EXECUTE FUNCTION articles_tsv_update();
```

Why every piece matters:

- **`setweight(... , 'A')`** through `'D'` — Postgres FTS bucks lexemes
  into four weight classes. `ts_rank_cd` uses default weights
  `{0.1, 0.2, 0.4, 1.0}` for `{D, C, B, A}`. A title match contributes
  10× a body match per term. **Tested explicitly** in
  `fts_ranks_title_match_above_body_match`.
- **`coalesce(..., '')`** — `to_tsvector(NULL)` is NULL. NULL `||`
  anything is NULL. One missing summary and your tsv is empty.
- **Trigger on `OF title, summary, body_md`** — fires only when those
  cols change. Saves rewriting tsv on `updated_at`-only updates.
- **`BEFORE INSERT OR UPDATE`** — the row that hits the table already
  has the tsv set. No second statement.

### A2. The hot path

```sql
SELECT
  id, slug, title,
  ts_headline('english', COALESCE(summary, body_md),
              plainto_tsquery('english', $1),
              'MaxFragments=2, MaxWords=18, MinWords=6,
               StartSel=<mark>, StopSel=</mark>') AS snippet,
  ts_rank_cd(tsv, plainto_tsquery('english', $1))::float8 AS rank_score
FROM articles
WHERE published_at IS NOT NULL
  AND tsv @@ plainto_tsquery('english', $1)
ORDER BY ts_rank_cd(tsv, plainto_tsquery('english', $1)) DESC
LIMIT 50;
```

- **`plainto_tsquery`** is forgiving — handles spaces, ignores quotes,
  ANDs terms. Good for end-user input. Use `websearch_to_tsquery` if
  you want `+ - "phrase"` syntax surfaced.
- **`ts_headline`** does the snippet selection + `<mark>` wrapping for
  us. The `MaxWords/MinWords` keep snippets in the 6–18 word band
  that fits in a search result card.
- **`::float8` cast** is load-bearing. `ts_rank_cd` returns `real`
  (float4); sqlx's `f64` decode panics on the 4-byte read otherwise.
  We learned this the hard way (commit log).

### A3. pg_trgm fallback

```sql
SELECT id, slug, title, summary,
       similarity(title, $1)::float8 AS sim_score
FROM articles
WHERE published_at IS NOT NULL
  AND similarity(title, $1) > 0.2
ORDER BY similarity(title, $1) DESC
LIMIT 20;
```

We use `similarity > 0.2` rather than the `%` operator's default
threshold (0.3) because real typos like "postgrss" land in the
[0.2, 0.3] band. The `gin_trgm_ops` index on `title` accelerates
both `%` and `similarity()` predicates.

We only consult trgm when FTS returns **nothing**. Mixing the two
on every query gives noisy results: trgm's fuzzy match for "kafka"
might surface "Kabuki" because of trigram overlap. FTS-first,
trgm-fallback is the cheap right answer.

### A4. Reciprocal Rank Fusion

```rust
const RRF_K: f64 = 60.0;

for (i, h) in fts.iter().enumerate() {
    *score.entry(h.id).or_default() += 1.0 / (RRF_K + (i as f64 + 1.0));
}
for (i, h) in meili.iter().enumerate() {
    *score.entry(h.id).or_default() += 1.0 / (RRF_K + (i as f64 + 1.0));
}
```

RRF's claim to fame: **you don't need to know the absolute scores of
either ranker** — only the rank positions. FTS scores live on
[0, ∞), Meili scores live on [0, 1]. Normalising them to a common
scale is per-query, error-prone, and slow. RRF says: sum of
`1 / (k + rank)` across rankers. Done.

`k = 60` is the value Cormack/Clarke recommended in the original
paper. We don't expose it as a knob — empirically it's stable
across corpora, and "tweak k until it works" is a tuning treadmill
nobody wins.

pg_trgm gets a 0.5× factor (`0.5 / (k + rank)`) because its
rankings are *salvage*, not *primary*. We promote them when FTS is
empty, but if FTS gave us 50 hits, we don't want a fuzzy match to
outrank the third FTS hit.

### A5. Meili: best-effort

```rust
let res = match res {
    Ok(r) if r.status().is_success() => r,
    _ => return Ok(vec![]),
};
```

Meili down? Meili slow? Meili returned 500? We return empty and the
FTS results show alone. The search page never blocks on the optional
ranker. **You will thank yourself the first time Meili upgrades
break the index schema.**

---

## B. Frontend

### B1. The AI-Overview "quick answer"

```html
<aside class="answer-block" role="region" aria-label="Quick answer">
  <h2>Quick answer</h2>
  <p>{article.summary}</p>
</aside>
```

Google's "AI Overview" and "Featured Snippet" features look for a
short, complete answer near the top of the page. We surface
`article.summary` as exactly that — same content as the meta
description, which is the canonical answer crawlers expect.

### B2. TechArticle + FAQPage JSON-LD

Two separate `<script type="application/ld+json">` blocks. Google
allows multiple, parses each independently. We emit FAQPage only
when there are faqs rows — emitting an empty FAQPage is a structured-
data warning that hurts ranking.

### B3. `{@html hit.snippet}` is safe here

The snippet HTML is generated **server-side by `ts_headline`** from
the article body, which we already sanitised through `ammonia` on
write. The only HTML in the snippet is `<mark>...</mark>` tags
inserted by Postgres. Risk: zero. Documented because the next reader
will ask.

---

## C. Tests

### C1. `cargo test` (4 tests)

- `auth::hash::tests::round_trip` (Argon2 sanity)
- `search_flow::fts_ranks_title_match_above_body_match` — proves the
  `setweight A > C` rule end-to-end via the HTTP API.
- `search_flow::pg_trgm_fallback_handles_typos` — types "postgrss",
  expects "postgres-tuning" first hit.
- `search_flow::empty_query_returns_empty` — the only valid behaviour
  when the user hits "Search" with nothing typed.

### C2. `pnpm test:unit` (4 tests)

`articlesApi.list`, slug URL-encoding, `searchApi.query` URL-encoding,
non-2xx → `ApiCallError`.

### C3. `pnpm test:e2e` (4 × 4 = 16 runs)

- Homepage axe-clean.
- Search form reflects q in URL.
- Unknown slug returns 404.
- Sitemap.xml is well-formed.

---

## D. What you can do now

1. Build search for any internal docs corpus without procuring an
   index-as-a-service.
2. Reason about ranking as "weight by location" (FTS) + "fall back to
   shape" (trgm) + "blend without scoring" (RRF).
3. Layer optional services (Meili) so a service outage degrades
   gracefully instead of breaking the page.

Project 25 — Course Marketplace with Stripe Connect.
