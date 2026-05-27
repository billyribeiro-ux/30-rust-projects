# Project 15 — File Vault with Chunked Uploads

A self-hosted Dropbox-shaped file vault. Drag a 200 MB video into the browser;
it streams up in 1 MiB chunks, resumes after a tab close, and lands on disk
exactly once even if you upload the same bytes twice. Built on Axum +
Postgres + `object_store`, with a SvelteKit 5 file browser and a tus-style
chunked uploader written from scratch.

## What's new vs. project 14

- **tus-style resumable uploads** — POST creates a session, PATCH appends
  chunks at `Upload-Offset`, HEAD lets the client resume after a crash.
- **Content-addressed storage** — every blob is keyed by its SHA-256.
  Re-uploading the same file dedupes to a single `file_versions` row
  and never re-writes the bytes.
- **MIME sniffing** with `infer` — we don't trust the client's
  `Content-Type` claim.
- **EXIF stripping** for JPEG/PNG via `image` decode-then-encode roundtrip.
- **`object_store` abstraction** — same code path runs against local FS
  (the dev default) and S3/MinIO in production.
- **Background reaper** — stale `upload_sessions` are pruned every hour
  along with their temp files.

## Stack delta

- **+** `object_store`, `infer`, `image`, `sha2`, `hex`, `bytes`,
  `tokio-util`, `futures` in Rust; `proptest` + `tempfile` for tests.
- **−** `chrono-tz`, `rrule`, `lettre` (calendar-specific deps).

## Layout

```
projects/15-vault/
├── README.md              (this file)
├── COMMANDS.md            every shell command in execution order
├── LESSON.md              line-by-line walkthrough
├── docker-compose.yml     postgres:5436 + minio:9011/9012
├── backend/               Rust crate (port 3014)
│   ├── Cargo.toml
│   ├── .env.example
│   ├── migrations/0001_init.sql
│   ├── src/
│   │   ├── main.rs        bootstrap + CORS + reaper task
│   │   ├── db.rs          PgPool builder
│   │   ├── error.rs       AppError + IntoResponse
│   │   ├── state.rs       AppState (pool, storage, upload_tmp)
│   │   ├── storage.rs     object_store wrapper (LocalFS or S3)
│   │   ├── blob.rs        SHA-256 / sniff / EXIF-strip helpers
│   │   ├── auth/          hash + session (ported from 14)
│   │   └── routes/        auth, folders, files, uploads
│   └── tests/assembly.rs  proptest + integration
└── frontend/              SvelteKit (dev 5187, preview 4187)
    ├── package.json
    ├── e2e/vault.spec.ts
    └── src/
        ├── lib/
        │   ├── api.ts             typed client
        │   ├── upload-client.ts   tus-style chunked uploader
        │   └── server/api.ts      cookie-forwarding fetch
        └── routes/
            ├── (auth)/login + signup
            └── (app)/vault/+page.svelte
```

## Quality gates

| Gate                  | Status |
| --------------------- | ------ |
| `cargo fmt --check`   | pass   |
| `cargo clippy -D warnings` | pass |
| `cargo test`          | 13 pass (9 unit + 4 integration/proptest) |
| `pnpm check`          | 0 errors 0 warnings |
| `pnpm test:unit`      | 4 pass |
| `pnpm test:e2e`       | 20 pass (5 specs × 4 viewports incl. axe-core) |
| Svelte autofixer      | 0 issues remaining |
| `cargo sqlx prepare`  | `.sqlx/` committed |

## What's next

Project 16 introduces Stripe Checkout — money on the internet, depending
on this project's blob-store so digital goods download links can hand
out short-lived presigned URLs.
