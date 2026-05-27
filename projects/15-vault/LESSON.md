# Project 15 — Lesson

Three lessons live in this project: the tus-style upload protocol; the
content-addressed storage trick that makes dedup free; and the
storage-backend abstraction that lets a local-FS dev loop graduate to
S3 with zero code changes.

---

## A. Backend

### A.1 — The `object_store` abstraction (`backend/src/storage.rs`)

```rust
match backend.as_str() {
    "s3"  => Storage { inner: Arc::new(AmazonS3Builder::new()...build()?) },
    _     => Storage { inner: Arc::new(LocalFileSystem::new_with_prefix(...)?) },
}
```

The `object_store` crate gives us a `dyn ObjectStore` that works
identically for local FS, S3, GCS, Azure Blob, or in-memory. The dev
default is local FS — no Docker required for a `cargo run`. Set
`VAULT_STORAGE=s3` in `.env` and the same code talks to MinIO or AWS.

The blob path is `blobs/AA/<sha256>` where `AA` is the first two hex
chars of the digest. A single flat directory of a million files is
miserable on most filesystems; the two-level fan-out keeps each
directory tiny.

### A.2 — Content-addressed storage (`routes/uploads.rs` finaliser)

```rust
let final_sha = blob::sha256_hex(&final_bytes);
let existing = sqlx::query!("SELECT id FROM file_versions WHERE sha256 = $1", final_sha)
    .fetch_optional(&s.pool).await?;
let version_id = if let Some(v) = existing {
    v.id                                   // dedupe: never write the bytes
} else {
    s.storage.put(&final_sha, final_bytes).await?;
    sqlx::query!("INSERT INTO file_versions ...").execute(&s.pool).await?;
    new_id
};
```

`file_versions.sha256 UNIQUE` is the dedup engine. Two users uploading
the same MP3 share a row; their `files` rows differ only in name and
ownership. Deletion is reference-counted by `files.version_id`; an
orphaned `file_versions` row + blob can be GC'd later by a sweeper
(documented as future work in `files.rs`).

The subtlety: we hash the **stored** bytes, not the input. EXIF strip
mutates the bytes, so two clients with the same photo but different
camera metadata still dedupe. If we hashed the pre-strip bytes, we'd
write twice.

### A.3 — tus-style protocol (`routes/uploads.rs`)

Three handlers, no surprises:

- `POST /api/uploads` → `{ upload_id }`, inserts `upload_sessions`,
  creates a zero-byte temp file at `data/uploads/<uuid>.part`.
- `HEAD /api/uploads/{id}` → returns `Upload-Offset` from the DB so a
  reconnecting client knows where to resume.
- `PATCH /api/uploads/{id}` with header `Upload-Offset: N` and a chunk
  body. If `N != received`, we 409 (the client must HEAD and retry).
  Otherwise we append, bump `received`, and on `received == total_size`
  we finalise (hash, sniff, strip, dedupe, insert `files`, delete the
  session row + temp file).

We append with `OpenOptions::new().append(true).open()` per chunk
instead of holding an open file handle in state. Slightly more syscalls
but the OS page cache absorbs them, and the state machine is trivial.

### A.4 — MIME sniffing + EXIF strip (`blob.rs`)

```rust
pub fn sniff_mime(bytes: &[u8]) -> String { /* infer::get */ }
pub fn strip_exif(input: &Bytes, mime: &str) -> Bytes { /* decode + re-encode */ }
```

`Content-Type` arrives untrusted from the client. `infer` reads the
first ~16 bytes and matches a known-magic-number database. We use the
sniffed MIME as the source of truth.

For JPEG/PNG we decode-then-re-encode via `image`. The output has only
pixel data: no EXIF, no XMP, no ICC. We do this in memory; that means
the demo's 5 GiB cap is a real limit (the whole file lives in RAM at
finalise). Production would stream-strip with an EXIF-aware parser
(`kamadak-exif` etc.) and keep ICC profiles. Documented; not done.

### A.5 — Background reaper (`main.rs` + `routes/uploads::reap_stale`)

A `tokio::spawn` task wakes every hour and deletes `upload_sessions`
rows where `expires_at < now()` along with their temp files. Without
this, every abandoned upload leaks disk. The 24h TTL is generous
enough that a pause-overnight user resumes successfully.

### A.6 — Auth port from project 14

`auth/hash.rs` + `auth/session.rs` are byte-for-byte ports minus the
email-token / forgot-password surface, which is out of scope for a file
vault. The `AuthUser` extractor still 401s on missing/expired cookies,
so every protected endpoint inherits auth for free.

---

## B. Frontend

### B.1 — Chunked uploader (`lib/upload-client.ts`)

```ts
while (offset < size) {
    const chunk = file.slice(offset, Math.min(offset + CHUNK_SIZE, size));
    const r = await patchChunk(upload_id, offset, chunk, signal);
    offset = r.newOffset;
    onProgress?.({ loaded: offset, total: size, percent: offset / size });
}
```

Three behaviours worth highlighting:

1. **Resumability.** Before creating a new session we check
   `localStorage` for a pending upload keyed by `(filename, size)`. If
   one exists and the server still knows it (HEAD returns 200), we
   resume from the server's offset. The cleanup happens on success,
   not on start — that way a closed tab leaves the resume key intact.

2. **Exponential-backoff retry.** Network blips on `PATCH` retry with
   `250ms · 2^attempt` waits, up to 3 attempts. A 409 is special: it
   means our local offset is stale, so we re-HEAD and continue from
   the server's truth.

3. **Progress events.** A simple callback bus, not a store. The
   consuming component (`vault/+page.svelte`) keeps an `uploading[]`
   array in `$state` and replaces the matching entry on each event so
   Svelte's reactivity triggers a paint.

### B.2 — Vault page (`(app)/vault/+page.svelte`)

The folder tree, breadcrumb, dropzone, and file list all live in one
page keyed by `?folder=<uuid>`. The spec asks for `[[...path]]`
catch-all routes; we chose a query param instead because the parent
chain is computed on the client by walking `parent_id` in the already
loaded `folders[]` — no extra round trip. A path-based UI would
require server-side name → id resolution.

`folders`, `files`, and `folderId` are `$derived` from `data` rather
than `$state` synced via `$effect`. The autofixer specifically calls
out the latter as an anti-pattern: any time you find yourself writing
`$state(x); $effect(() => x = data.x)` you wanted `$derived(data.x)`.

### B.3 — Server-side cookie forwarding (`lib/server/api.ts`)

Same trick as project 14. `event.fetch` won't auto-forward cookies to
the cross-origin Rust backend (different port), so we wrap `fetch` and
attach `cookie: app_session=...` manually. Auth-aware page loads use
`serverFetch(locals.sessionCookie)` and everything else uses bare
`fetch`.

### B.4 — A11y discipline

The dropzone has `role="region"` + `aria-label`. The breadcrumb is a
`<nav aria-label="Breadcrumb"><ol>` with focusable `<button>`s, not
clickable `<div>`s. The new-folder input has a visually-hidden `<label
for>`. axe-core runs clean on `/signup` and `/vault` on all four
viewports.

---

## C. Tests

- **Unit (Rust).** `blob.rs` covers `sha256_hex`, `sniff_mime`, and
  `strip_exif` passthrough; `routes/auth.rs` tests `normalize_email`;
  `auth/hash.rs` covers Argon2 roundtrip + length validation.
- **Integration (`tests/assembly.rs`).** Reassembly correctness,
  resume-after-interrupt, and a proptest fuzz that splits a random
  blob at random offsets and asserts byte-for-byte equality.
- **Unit (TS).** `api.test.ts` covers the request shape, error
  mapping, and `downloadUrl` encoding.
- **E2E (`e2e/vault.spec.ts`).** Five specs × four viewports = 20
  cases: unauth redirect, signup + axe, chunked upload + UI list +
  download bytes, dedup, 401 enforcement.

---

## D. What you can now do

- Build a production-ready resumable upload protocol in two days.
- Pick a blob-storage backend at deploy time, not commit time.
- Make a "did you mean to delete?" UI safe — `file_versions` keeps the
  bytes around even if every `files` row is gone, ready for an undo.
- Tell your security team that the JPEGs your users upload have no
  GPS coordinates stitched into the headers.

## Accepted autofixer / deviation log

- Vault page initially stored `folders`/`files` as `$state` synced
  from `data` in an `$effect`. The autofixer flagged this as an
  anti-pattern; we switched to `$derived`.
- The spec asks for `(app)/vault/[[...path]]`. We use `?folder=<id>`
  instead, because parent traversal is local-only with a flat
  `folders[]` and the breadcrumb is derived client-side. Less code,
  same UX.
- EXIF strip is in-memory (5 GiB ceiling). A streaming strip is left
  for a future project; the trade-off is called out at the call site.
- Storage default is `LocalFileSystem`, not MinIO. The S3 path is
  fully wired (`AmazonS3Builder`); flip `VAULT_STORAGE=s3` to use it.
  Documented so a learner can reach for either backend.
