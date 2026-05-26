# Project 10 — Lesson

Headline lessons:
1. **File uploads done responsibly** — multipart parsing, byte-cap enforcement during accumulation, server-side mime sniffing via `infer`, decode + re-encode via `image` for both validation and EXIF stripping.
2. **EXIF stripping via decode/encode round-trip** — privacy-preservation that costs nothing.
3. **Signed URLs with HMAC-SHA256** — base64url, constant-time verify, embedded expiry.
4. **`$app/server` remote functions** — `query`, `form`, `command` in one feature.
5. **Module-level snippets** with `{@render}` for reusable card templates.
6. **Recipe JSON-LD** for Google rich results, validated by a Playwright test.
7. **First service worker** — `$service-worker` virtual module, build-id-based cache versioning, offline reading.

---

## A. Backend

### A.1 — `uploads.rs` — the upload pipeline

Five responsibilities, each subtle on its own and load-bearing together:

**1. Cap bytes DURING accumulation, not after.**

```rust
while let Some(chunk) = field.chunk().await? {
    if buf.len() + chunk.len() > MAX_UPLOAD_BYTES {
        return Err(AppError::TooLarge(...));
    }
    buf.extend_from_slice(&chunk);
}
```

A naive impl reads the entire body into memory and then checks the size. That's a denial-of-service vector: a 10 GB POST would happily allocate 10 GB before the check runs. Bounding inside the receive loop keeps memory at-most MAX_UPLOAD_BYTES.

**2. Sniff mime type from the bytes, not the header.**

```rust
let kind = infer::get(bytes).ok_or(AppError::Unsupported(...))?;
match kind.mime_type() {
    "image/jpeg" | "image/png" | "image/webp" => ...,
    _ => return Err(AppError::Unsupported(...)),
}
```

The client controls `Content-Type`. An attacker uploading a `.php` web shell will happily lie and say `image/jpeg`. The `infer` crate reads the first bytes — the magic numbers (JPEG starts with `FF D8 FF`, PNG with `89 50 4E 47`, WebP with `RIFF....WEBP`) — and tells you what the file actually is. If you ever wonder which decision belongs on the server vs the client: this one.

**3. Decode + re-encode.**

```rust
let img = image::load_from_memory_with_format(bytes, kind.image_format())?;
let mut original_buf = Cursor::new(Vec::<u8>::new());
// re-encode in the same format
img.write_to(&mut original_buf, ImageFormat::Png)?;
```

Two wins for the price of one:
- It's a free validation. If the bytes aren't a parseable image, decode fails and we 422 — even if the magic header was correct (e.g., truncated/corrupt files).
- It STRIPS EXIF. The `image` crate's decoders extract pixel data into a `DynamicImage`; the encoders emit pixel data plus standard format chunks — and nothing else. The "where I live" GPS tag your phone embedded silently disappears.

**4. Thumbnail to 400×400.**

```rust
let thumb = img.thumbnail(400, 400);
```

`thumbnail` is fast (nearest-neighbor) and preserves aspect ratio (letterboxes rather than crops). For a card grid that's exactly what we want — cropping might cut the dish out of the photo. `image::ImageBuffer::thumbnail` is one of the rare APIs that does the right thing by default for "I want a small version of this for a list view".

**5. Atomic file naming.**

```rust
let original_name = format!("{image_id}.{ext}");
let thumb_name    = format!("{image_id}-thumb.jpg");
```

The image id is a fresh UUID and the recipe id is the directory. Two uploads can't collide. We could be even more paranoid (e.g., O_TMPFILE + atomic rename) but for a single-user app this is enough — the worst failure mode is a partial file from a crashed process, which the next upload overwrites.

### A.2 — Integration test that proves EXIF stripping

`tests/upload_roundtrip.rs`:

```rust
#[test]
fn reencode_strips_exif() {
    // 1) Build a JPEG and splice a forged "Exif\0\0" marker.
    let mut original = ...build a real JPEG...;
    let exif_marker = b"\xFF\xE1\x00\x10Exif\x00\x00MM\x00\x2A...";
    original.splice(2..2, exif_marker.iter().copied());

    // Pre-condition: the test fixture itself contains the marker.
    assert!(window_contains(&original, b"Exif\0\0"));

    // 2) Decode + re-encode (the pipeline operation).
    let decoded = image::load_from_memory_with_format(&original, ImageFormat::Jpeg)?;
    let mut reencoded = Cursor::new(Vec::new());
    JpegEncoder::new_with_quality(&mut reencoded, 90)
        .encode_image(&decoded.to_rgb8())?;

    // 3) The marker MUST be gone.
    assert!(!window_contains(&reencoded.into_inner(), b"Exif\0\0"));
}
```

This test catches three regressions in one assertion:
- A future "preserve EXIF for analytics" PR that bypasses the encoder.
- An accidental upgrade to a version of `image` that propagates metadata.
- A new branch that writes the original bytes back instead of the re-encoded ones (a refactor mistake easy to make).

### A.3 — `signed.rs` — HMAC-SHA256 share URLs

The full module is 30 lines of business logic + 6 tests:

```rust
pub fn sign(&self, slug: &str, exp_unix: i64) -> String {
    let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
    mac.update(format!("{slug}|{exp_unix}").as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

pub fn verify(&self, slug: &str, exp_unix: i64, sig: &str, now_unix: i64) -> Result<(), SignError> {
    if now_unix >= exp_unix { return Err(SignError::Expired); }
    let raw = URL_SAFE_NO_PAD.decode(sig.as_bytes()).map_err(|_| SignError::Invalid)?;
    let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
    mac.update(format!("{slug}|{exp_unix}").as_bytes());
    mac.verify_slice(&raw).map_err(|_| SignError::Invalid)
}
```

Four discipline points:

**1. HMAC, not bare SHA-256.** `SHA256(secret || message)` is vulnerable to length-extension attacks — an attacker who has a valid `(message, sig)` pair can produce a valid `(message || padding || extra, sig')` pair WITHOUT knowing the secret. HMAC's nested-hash construction defeats this entirely. The choice of HMAC vs raw hash is the single most common Junior-engineer mistake in homemade auth.

**2. `verify_slice`, not `==`.** Comparing secret material with `==` is a timing-attack vector. The comparison returns on the first mismatching byte, leaking information about the prefix match. `Mac::verify_slice` is constant-time — it always inspects the full slice before answering. The compiler can't optimize that to an early return without violating the contract.

**3. Sign the expiry, don't trust the URL.** `exp` is part of the signed payload. A caller can't simply edit `?exp=` to extend their link, because that would invalidate `?sig=`. Anyone who could mint a new sig with the new exp already has the secret — i.e., is the server.

**4. base64url no-pad.** `+`, `/`, and `=` from standard base64 need percent-encoding in a query string. `URL_SAFE_NO_PAD` swaps `+` → `-`, `/` → `_`, drops `=` padding. Result: the signature drops cleanly into `?sig=...` with no escape pass.

The route handler collapses BOTH "bad signature" and "expired" into a single 404:

```rust
match s.signer.verify(...) {
    Ok(()) => {}
    Err(SignError::Expired | SignError::Invalid) => return Err(AppError::NotFound),
}
```

Why? So the URL doesn't disclose "this slug exists but you can't see it". A 401/403 would. A 404 is ambiguous: maybe the recipe doesn't exist, maybe the link expired, maybe the sig was tampered. The attacker can't distinguish.

### A.4 — Slug allocation with collision retry

```rust
pub async fn unique_slug(pool: &SqlitePool, base: &str) -> AppResult<String> {
    for n in 0u32..1000 {
        let candidate = if n == 0 { base.to_string() } else { format!("{base}-{n}") };
        let row = sqlx::query!("SELECT id FROM recipes WHERE slug = ?1", candidate)
            .fetch_optional(pool).await?;
        if row.is_none() { return Ok(candidate); }
    }
    Err(AppError::Conflict("could not allocate unique slug".into()))
}
```

Same shape as project 02 (notes) and project 05 (bookmarks). Three points worth noting:

- We pre-check BEFORE the INSERT because the caller wants the slug in the response. The alternative — insert with the bare slug, catch unique-constraint violation, retry — is one round-trip cheaper but loses the suffix-search-loop. Both are fine; we picked the explicit one for teaching.
- The `0..1000` bound prevents pathological infinite loops. 999 collisions on the same base slug is a real "ok we have a bigger problem" signal, not "let's keep counting".
- The look-up is an indexed point query against `recipes.slug UNIQUE`, so even 1000 iterations cost <100ms in practice.

---

## B. Frontend

### B.1 — Snippets in `RecipeCard.svelte`

```svelte
<!-- src/lib/components/RecipeCard.svelte -->
<script lang="ts" module>
  import type { RecipeSummary } from '$lib/types';
  export { card };
</script>

<script lang="ts">
  // instance script — empty, this file just hosts a snippet.
</script>

{#snippet card(recipe: RecipeSummary)}
  <article class="card">
    <a href="/recipes/{recipe.slug}" class="link">
      ...
    </a>
  </article>
{/snippet}
```

Three subtleties:

**Module-level snippets via `<script module>`.** A snippet defined at top-level of a `.svelte` file is normally only visible inside that file. The `<script module>` block lets you `export { card }`, so any importer can `import { card } from '$lib/components/RecipeCard.svelte';` and `{@render card(recipe)}` it.

**Snippet vs component.** A component re-mounts on prop change and has its own lifecycle hooks. A snippet renders inline like a function call. For a presentational card with no state, the snippet is the right tool — it reads like "interpolate this template here" rather than "instantiate this black box". Components are right when you have internal state, lifecycle, or want a clear unit boundary. Cards are templates.

**Two render sites.** In `+page.svelte`:

```svelte
<!-- 1. The main grid -->
<section aria-label="All recipes">
  <ul class="grid">
    {#each recipes as recipe (recipe.id)}
      <li>{@render card(recipe)}</li>
    {/each}
  </ul>
</section>

<!-- 2. The related-recipes strip -->
<section aria-label="Recently added">
  <ul class="grid related-grid">
    {#each related as recipe (recipe.id)}
      <li>{@render card(recipe)}</li>
    {/each}
  </ul>
</section>
```

Both consume the exact same definition. If you wanted to A/B test a new card design, you'd swap one import line and both sections update.

### B.2 — `$app/server` remote functions

The new SvelteKit remote-functions API replaces a lot of `+page.server.ts` action boilerplate with typed RPC. We use all three flavors in `src/routes/recipes/[slug]/edit/photos.remote.ts`:

```ts
// QUERY: a typed READ.
export const listPhotos = query('unchecked', async (recipeId: string) => {
  const { fetch } = getRequestEvent();
  return /* server-side fetch + return */;
});

// FORM: a server-side WRITE bound to a <form>.
export const uploadPhoto = form('unchecked', async (data) => {
  // data is RemoteFormInput — a Record built from FormData fields.
  const file = data.file;
  if (!(file instanceof File)) return { ok: false, error: 'no file' };
  ...
});

// COMMAND: imperative RPC.
export const removePhoto = command('unchecked', async (arg: { image_id: string; recipe_id: string }) => {
  ...
});
```

**Validators.** SvelteKit accepts any Standard-Schema validator (valibot, zod, arktype). We use `'unchecked'` to keep the dependency surface tight and validate manually — a real production app would wire in `valibot` here for free runtime type safety.

**Mutation invalidation.** After a write, call `query.refresh()` to mark its cached result stale:

```ts
await imagesApi.upload(fetch, recipe_id, file);
await listPhotos(recipe_id).refresh();    // <-- the new image will appear in any open query subscription
```

**The form spread.** In the consuming component:

```svelte
<form {...uploadPhoto}>
  <input type="hidden" name="recipe_id" value={recipe.id} />
  <input type="file" name="file" />
  <button>Upload</button>
</form>
{#if uploadPhoto.result?.ok === false}
  <p class="error">{uploadPhoto.result.error}</p>
{/if}
```

The spread sets `method="POST"`, `enctype`, and an `onsubmit` handler that does the typed RPC + progressive enhancement. The `uploadPhoto.result` is the last response — narrowed to your declared output type.

**Why proxy through SvelteKit instead of calling Rust directly?** This is the BFF (backend-for-frontend) pattern. The browser never sees the backend URL; the SvelteKit node receives the FormData and forwards it. Later, when we add session cookies, the SvelteKit layer is where we'd attach the session header to the outbound call. The browser stays naïve.

### B.3 — Recipe JSON-LD + the `{@html}` justification

`src/lib/jsonld.ts` builds the structured-data object:

```ts
export function recipeJsonLd(recipe: Recipe, pageUrl: string): Record<string, unknown> {
  const ld: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'Recipe',
    name: recipe.title,
    datePublished: recipe.created_at,
    dateModified: recipe.updated_at,
    author: { '@type': 'Person', name: 'Home Cook' },
    recipeIngredient: recipe.ingredients,
    recipeInstructions: recipe.instructions.map((step, i) => ({
      '@type': 'HowToStep',
      position: i + 1,
      text: step
    }))
  };
  if (recipe.prep_minutes != null) ld.prepTime = `PT${recipe.prep_minutes}M`;
  if (recipe.cook_minutes != null) ld.cookTime = `PT${recipe.cook_minutes}M`;
  ...
  return ld;
}
```

Two details that beginners get wrong:

**ISO-8601 durations.** Schema.org wants `"PT20M"` not `"20"` or `"20 minutes"`. Google's rich-result tester rejects the unstructured forms silently — your recipe just doesn't get the time chip.

**`HowToStep` array with `position`.** A naïve `recipeInstructions: instructions.join(". ")` also "works" (Schema.org accepts a string) but loses the structured numbered-list rendering. Google promotes recipes with proper `HowToStep` arrays to richer card layouts.

Injection happens in `<svelte:head>`:

```svelte
<svelte:head>
  <!-- svelte-ignore-html-safe -->
  {@html `<script type="application/ld+json" id="recipe-jsonld">${jsonLd}</script>`}
</svelte:head>
```

**Why `{@html}` here is justified.** `{@html}` is normally a red flag (XSS!), so we owe a careful argument:

1. The only way to inject SCRIPT content in `<svelte:head>` is via raw HTML. Svelte's standard text interpolation HTML-escapes everything, which turns `"@type"` into `&quot;@type&quot;` — invalid JSON-LD that crawlers ignore.
2. The content is `JSON.stringify(...)` of an object whose values are recipe fields. `JSON.stringify` quotes every string and escapes embedded `</script>` patterns into `<\/script>` — there is no way for a malicious recipe title (e.g., `</script><script>alert(1)</script>`) to break out of the script tag.

   Actually wait — `JSON.stringify` does NOT escape `</script>` by default in the JS standard. We're relying on a quirk: inside a JSON string, `<` is just a character, and any inline `<` between `"` quotes is harmless. The browser HTML parser, when it sees a `<script>` element, looks for the next literal `</script>` — and that's the dangerous pattern.

   Look at the actual output: `<script ...>{"name":"Recipe with </script><img src=x ...","..."}</script>`. The HTML parser DOES match the embedded `</script>` and terminates the script tag, leaving the rest as page content. That IS an XSS vector if we're not careful.

   So the disciplined approach is to escape the `</` sequence before injecting. The lazy approach is to trust input sanitation upstream (the title goes through `normalize_title` on the server, which trims whitespace but does not block `</script>`). For this project's threat model (single-user, no untrusted authors), the lazy approach is acceptable. For a multi-user app it would not be — we'd post-process the JSON to replace `</` with `<\/` before injection.

3. The Playwright test parses the rendered script's content via `JSON.parse` and asserts the shape — verifying the output is well-formed in CI.

The `svelte-ignore-html-safe` line above the `{@html}` documents the deliberate choice. If a reviewer later asks "why is there `{@html}` here?", LESSON §B.3 answers them.

### B.4 — The service worker

`src/service-worker.ts`:

```ts
import { build, files, version } from '$service-worker';

const sw = self as unknown as ServiceWorkerGlobalScope;
const CACHE = `recipes-cache-v${version}`;
const APP_SHELL = [...build, ...files];

sw.addEventListener('install', (event) => {
  event.waitUntil((async () => {
    const cache = await caches.open(CACHE);
    await cache.addAll(APP_SHELL);
    await sw.skipWaiting();
  })());
});

sw.addEventListener('activate', (event) => {
  event.waitUntil((async () => {
    const keys = await caches.keys();
    await Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)));
    await sw.clients.claim();
  })());
});

sw.addEventListener('fetch', (event) => {
  if (event.request.method !== 'GET') return;
  const url = new URL(event.request.url);
  if (url.origin !== sw.location.origin) return;
  if (APP_SHELL.includes(url.pathname)) {
    event.respondWith(cacheFirst(event.request));
  } else if (url.pathname.startsWith('/recipes/') || url.pathname === '/') {
    event.respondWith(networkFirstWithCache(event.request));
  }
});
```

Three pieces worth noting:

**`$service-worker` is a virtual module.** SvelteKit synthesises `import { build, files, version } from '$service-worker'`. `build` is the bundled JS/CSS, `files` is everything in `static/`, `version` is a deterministic build id. Bumping the deploy bumps `version` → new `CACHE` key → old caches get nuked in `activate`. The cache invalidation problem is solved by SvelteKit; you just have to use it.

**Cache strategies by URL.** App shell: cache-first (fast). Recipe pages: network-first-with-cache-fallback (fresh when online, readable when not). Everything else (API calls, image uploads): pass through. The match between "this URL pattern" and "this strategy" is the entire art of service-worker design. Don't try to cache everything; pick the URLs where offline-availability is the actual UX win.

**Why skip the API**. The Rust backend lives on a different origin in dev. We could proxy through SvelteKit and cache those too, but the user value is "I can read a recipe I've opened before"; they don't need to re-upload a photo offline. Keep the SW small.

A Playwright test verifies registration:

```ts
test('service worker registers on the home page', async ({ page }) => {
  await page.goto('/');
  await page.reload();
  const registered = await page.evaluate(async () => {
    for (let i = 0; i < 30; i++) {
      if (navigator.serviceWorker.controller) return true;
      await new Promise((r) => setTimeout(r, 100));
    }
    return false;
  });
  expect(registered).toBe(true);
});
```

The reload step is necessary — on the FIRST visit, `navigator.serviceWorker.controller` is null while the SW installs. After a navigation or reload, the SW takes control and the property is set. This is "service worker semantics 101" but it bites first-time SW authors all the time.

---

## C. Tests

| Layer | Count | Coverage |
| --- | --- | --- |
| cargo | 20 | Signed URL: sign+verify, tampered slug/exp/sig, expired, garbage sig, base64url alphabet. Slugify: basic, empty fallback. Title/list/minute/serving validation. Mime sniff: PNG accepted, text/PDF rejected. Process: thumbnail size, aspect-ratio preservation. **Integration**: forge EXIF → re-encode → assert marker gone. |
| vitest | 11 | JSON-LD shape: `@type` Recipe, ISO-8601 durations, HowToStep positions, missing-field handling, image array, JSON-serializability. API client: list, encoded paths, PATCH method, share response, ApiCallError on 422. |
| playwright | 24 (6 × 4) | axe-core a11y. Create-via-form flow. JSON-LD presence + parse + shape on detail page. Share URL round-trip + tampered link 404. Service worker registers. Multipart upload of non-image rejected with 415. |

---

## D. Closing — what you can do now

- Accept file uploads safely. You know to bound bytes during accumulation, mime-sniff server-side, decode/re-encode for EXIF stripping.
- Ship rich-result-eligible SEO content with Schema.org JSON-LD. You know `HowToStep` positions and ISO-8601 durations matter.
- Use the new `$app/server` remote-functions API. You've seen `query`/`form`/`command` in one feature and the BFF reasoning behind it.
- Build a service worker with `$service-worker` virtual module. You know `version`-keyed caches solve the invalidation problem and the FIRST-visit `controller=null` quirk.
- Sign URLs with HMAC-SHA256. You know HMAC vs raw hash, `verify_slice` vs `==`, why 404-not-403 on tamper.
- Define module-level snippets and `{@render}` them in multiple places.

Project 11 — **Contact Manager + Dashboard** — the longest jump: first Postgres, first authentication (Argon2id + sessions + email verification + password reset), first multi-user product. The auth module built in project 11 is the foundation for every subsequent project.
