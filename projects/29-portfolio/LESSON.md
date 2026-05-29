# Lesson — Project 29 (Cinematic Portfolio, GSAP #3)

Three things this project teaches that no earlier project did:

1. **`{@attach}` attachments** — Svelte 5's modern replacement for
   actions, used here to invoke `revealOnScroll(node)` per list item.
2. **GSAP + IntersectionObserver as a free ScrollTrigger** — the
   pattern most portfolios actually need, without pulling the paid
   Club plugin.
3. **View Transitions API** — one CSS line gets you cinematic route
   changes for free.

---

## A. Frontend

### A1. The `{@attach}` attachment

```svelte
<li
  {@attach (node) => {
    revealOnScroll(node);
  }}
>
```

`{@attach}` runs the supplied function with the DOM node when it
mounts and unmounts. Compared to the old `use:action` syntax:

- **No naming the action**. The function is anonymous and inline.
- **Closes over the current scope** — you can reach `data`, `state`,
  whatever, without a `params` argument.
- **Cleanup is the function's return value**. Returning `null` is
  fine when there's nothing to clean up.

We use it here to register an `IntersectionObserver` per card. When
the card scrolls into view, GSAP fades and lifts it. The observer
disconnects after the first reveal so we don't keep watching.

### A2. The motion module

```ts
export async function revealOnScroll(el: HTMLElement, opts: { y?: number; duration?: number } = {}) {
  if (prefersReducedMotion()) {
    el.style.opacity = '1';
    return;
  }
  const { gsap } = await loadGsap();
  gsap.set(el, { opacity: 0, y: opts.y ?? 36 });
  const io = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (e.isIntersecting) {
        gsap.to(el, { opacity: 1, y: 0, duration: opts.duration ?? 0.6, ease: 'power3.out' });
        io.disconnect();
      }
    }
  }, { threshold: 0.15 });
  io.observe(el);
}
```

- **`prefersReducedMotion`** short-circuits the entire path. No GSAP
  import, no observer. Users who opted out see fully-opaque content
  immediately.
- **`gsap.set(el, { opacity: 0, y: 36 })`** is the "hide it first" — we
  set the initial state synchronously so the user doesn't see a flash
  of fully-opaque content before the observer fires.
- **`threshold: 0.15`** — fire when 15% of the element is in view.
  At 0 you trigger when the element is technically visible (one
  pixel poking up). At 0.5+ the animation feels late.
- **`io.disconnect()`** — fire once, then stop. We don't unreveal
  on scroll-out; that's distracting.

### A3. View Transitions

```css
:global(html) { view-transition-name: root; }
```

Modern browsers (Chromium-based, Safari TP, soon Firefox) read this
and *automatically* cross-fade the page on navigation. Adding
`view-transition-name: hero-1` to a hero image makes navigation feel
like the image is the same physical node across pages — Apple's
product-page transitions, free.

We ship the root setup but leave per-element names for the editor to
add per post. The CSS:

```css
@media (prefers-reduced-motion: reduce) {
  :global(*) {
    animation-duration: 0.001ms !important;
    transition-duration: 0.001ms !important;
  }
}
```

…clamps all animations to nearly-zero for users who opted out. View
transitions become instant, GSAP becomes instant, browser-native
CSS transitions become instant. One rule, total compliance.

### A4. CreativeWork JSON-LD

The detail page emits:

```json
{
  "@context": "https://schema.org",
  "@type": "CreativeWork",
  "name": "...",
  "datePublished": "...",
  "dateModified": "...",
  "inLanguage": "..."
}
```

Search results will surface the title, date, and (if you add an
`image`) the hero. `inLanguage` lets Google route the right locale
variant to the right SERPs.

---

## B. Backend

### B1. Markdown on write, not on read

```rust
pub fn render_mdx(body: &str) -> String {
    let parser = Parser::new_ext(body, Options::ENABLE_STRIKETHROUGH | ... );
    let mut raw = String::new();
    html::push_html(&mut raw, parser);
    ammonia::clean(&raw)
}
```

The HTML is computed *once* in the CMS create/update path and stored
in `posts.body_html`. The public read path is a single SELECT — no
parser cost, no sanitisation cost per request.

The cost is one extra column and a slightly heavier write path. At
typical portfolio sizes (10s-100s of posts, rare updates) this is
the right trade.

### B2. The draft/published gate

```sql
CREATE INDEX idx_posts_status_published
    ON posts (status, published_at DESC NULLS LAST);
```

The public list query is `WHERE status = 'published' ORDER BY published_at DESC`
— a partial index would be cleaner here (only `status='published'` rows
in the index). We use the composite index because the CMS list also
sorts by status. Trade-off documented; not load-bearing.

### B3. The `mdsvex` upgrade path

To go from "Markdown HTML" to "MDX with embedded Svelte components":

1. Move content to `.mdx` files in the repo, not the DB.
2. Configure `mdsvex` in `svelte.config.js`.
3. Make routes prerendered (`export const prerender = true`).
4. Move the CMS to a separate admin app or remove it entirely.

That's a different project (closer to a static-site generator). This
one prioritises *the editor can ship from any device with a browser*.

---

## C. Tests

### C1. `cargo test` (3)

- `auth::hash::tests::round_trip` (Argon2)
- `cms_flow::draft_is_invisible_until_published` — drafts hidden,
  publish exposes; the body_html matches what render_mdx produces.
- `cms_flow::markdown_strips_script_tags` — sanitiser smoke test.

### C2. `pnpm test:unit` (3)

`postsApi.list`, slug URL-encoding, non-2xx → ApiCallError.

### C3. `pnpm test:e2e` (2 × 4 = 8)

Homepage axe-clean, 404 for unknown slug.

---

## D. What you can do now

1. Ship a portfolio that *feels* premium without an oversized bundle.
2. Reason about GSAP + IntersectionObserver as a "scroll-reveal" pair
   that scales to 100s of items.
3. Wire one CSS line to opt into View Transitions API and let the
   browser do cinematic route changes for free.

Project 30 — the SaaS Capstone. The integration of every primitive in
this curriculum.
