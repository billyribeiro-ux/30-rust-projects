# Project 01 — TODO Manager: The Lesson

Welcome. This is the first lesson. By the time you finish reading it, you will have written, with your own hands, a real, tested, full-stack web application.

This document is structured as a **walkthrough**. We will create one file at a time, in the order a senior engineer would actually create them. Above each code block I tell you:

- **The exact file to create**, with its full path inside the project.
- **Why this file exists** — what problem it solves.

After each code block we go through the code **line by line, in plain English**:

- *What* the line says.
- *Why* we wrote it that way.
- *What would break* if we wrote it the obvious-but-wrong way.

If something looks unfamiliar, slow down. Don't paste — type. Typing is how the patterns enter your fingers. By project 5 you'll be typing the boilerplate without thinking.

A note about *style*: production source files in this curriculum carry **almost no comments**. The teaching lives in this file, not in the code, because in real teams the next reader of your code needs clarity from names and structure — not paragraphs of explanation. This separation is itself a Principal-level lesson.

> **Note for the curious — the reading order**
>
> 1. Read this entire LESSON once before you write anything.
> 2. Then open `COMMANDS.md` and follow the steps top-to-bottom.
> 3. When `COMMANDS.md` says "create the file from step N of the LESSON", come back here.

---

## Part 0 — Mental model: what are we actually building?

Before any code, let's picture the system as two boxes connected by HTTP:

```
┌──────────────────────────┐         JSON over HTTP         ┌──────────────────────────┐
│  Browser                 │  ───────────────────────────▶  │  Rust backend (Axum)     │
│  SvelteKit page          │      GET    /api/todos         │  port 3000               │
│  - Svelte 5 runes        │      POST   /api/todos         │  - sqlx (SQLite)         │
│  - plain CSS             │      PATCH  /api/todos/:id     │  - tokio runtime         │
│  - Phosphor icons        │      DELETE /api/todos/:id     │  - tracing logs          │
│  port 5173 (dev)         │  ◀───────────────────────────  │                          │
└──────────────────────────┘                                └──────────────────────────┘
                                                                       │
                                                                       ▼
                                                              ┌─────────────────┐
                                                              │ SQLite (todo.db)│
                                                              └─────────────────┘
```

Two processes. Two languages. One product. The frontend doesn't know the backend is Rust. The backend doesn't know the frontend is Svelte. They only know JSON shapes. This is the deal that keeps systems clean.

We'll build the **backend first**. Why? Because the frontend depends on the backend's API shape. If we built the frontend first we'd be guessing.

---

# Part A — The backend

## A.1. Create the project folders

Create file (well, directories — same idea: *make a place for files to live*) at `projects/01-todo/`:

```bash
mkdir -p projects/01-todo/backend/src/routes
mkdir -p projects/01-todo/backend/migrations
mkdir -p projects/01-todo/frontend
cd projects/01-todo/backend
```

**Why this shape?** A monorepo (one repo, many projects) with a `backend/` + `frontend/` split per project mirrors how real teams ship. The `src/routes/` subfolder is conventional for "one file per HTTP resource".

## A.2. Initialize a Rust crate

Inside `projects/01-todo/backend/`, run:

```bash
cargo init --name todo-backend --bin .
```

This creates `Cargo.toml`, `src/main.rs`, and `.gitignore` for the crate. The `--bin` flag means "a binary, not a library". `--name todo-backend` is the crate name (what `cargo run` looks for). The `.` means "in this folder, don't create a new one".

## A.3. The dependency manifest

Create file `projects/01-todo/backend/Cargo.toml`. Cargo already made a stub for us, but we're going to overwrite it with the real deal:

```toml
[package]
name = "todo-backend"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
axum = { version = "0.8", features = ["macros"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal", "net"] }
tower-http = { version = "0.6", features = ["cors", "trace", "set-header"] }
sqlx = { version = "0.8", default-features = false, features = [
  "runtime-tokio",
  "tls-rustls",
  "sqlite",
  "macros",
  "migrate",
  "chrono",
] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }

[profile.dev]
opt-level = 0
debug = 1
```

### Line-by-line

- `[package]` block: identity card for the crate.
  - `name = "todo-backend"` — what `cargo` calls this. The compiled binary will be `target/debug/todo-backend`.
  - `edition = "2024"` — the modern Rust edition. Editions enable new syntax without breaking old code. **Why 2024?** Pattern matching improvements, better async ergonomics, default `dyn` keyword on traits.
  - `publish = false` — guardrail. If you accidentally type `cargo publish`, Cargo refuses. This crate is for us, not crates.io.

- `[dependencies]`: every library this crate uses at runtime.

  - `axum = { version = "0.8", features = ["macros"] }` — **Axum** is the HTTP framework. It's built on top of `tower` (a generic service abstraction) and `hyper` (HTTP). The `"macros"` feature enables `#[axum::debug_handler]` for clearer error messages.
  - `tokio = { ..., features = ["rt-multi-thread", "macros", "signal", "net"] }` — **Tokio** is the async runtime. Rust has no built-in event loop; Tokio provides one. Features:
    - `rt-multi-thread` — schedule async tasks across multiple OS threads.
    - `macros` — gives us `#[tokio::main]`.
    - `signal` — listen for SIGTERM / Ctrl+C (graceful shutdown).
    - `net` — TCP listener.
    - **Why not `features = ["full"]`?** Compile time. We pay for what we use.
  - `tower-http = { ..., features = ["cors", "trace", "set-header"] }` — pre-built middleware: CORS, request logging, header injection.
  - `sqlx = { ..., features = [...] }` — async SQL with compile-time-checked queries.
    - `default-features = false` is important: sqlx defaults pull in *both* native TLS and rustls. We pick `rustls` (pure Rust, no system OpenSSL).
    - `runtime-tokio` — sqlx supports multiple async runtimes; tell it we're using tokio.
    - `tls-rustls` — TLS for remote DBs (unused for SQLite, but harmless).
    - `sqlite` — the SQLite driver.
    - `macros` — gives us `sqlx::query!`, which validates SQL against a real DB at compile time. This is the single most important sqlx feature.
    - `migrate` — gives us `sqlx::migrate!()`, the embedded migration runner.
    - `chrono` — map SQL `TIMESTAMP` to `chrono::DateTime<Utc>`.
  - `serde = { ..., features = ["derive"] }` — the de-facto serialization library. `derive` enables `#[derive(Serialize, Deserialize)]`.
  - `serde_json` — JSON-specific helpers.
  - `thiserror = "2"` — derive macro for error enums. Lets us write `#[derive(thiserror::Error)]` instead of implementing `Display` and `Error` by hand.
  - `tracing` + `tracing-subscriber` — structured logging. `tracing::info!(user_id = 42, "logged in")` produces searchable log records, not just strings.
    - `env-filter` — filter by `RUST_LOG=todo_backend=debug`.
    - `json` — production JSON log output.
  - `chrono` — date/time. `Utc::now()` etc.
  - `uuid` — IDs for our todos. `v4` is random; `serde` lets us serialize them to JSON.

- `[dev-dependencies]`: only present when running tests, never in the production binary.
  - `reqwest` — HTTP client, used by integration tests later in the curriculum.

- `[profile.dev]`: tuning for the inner loop. We trade some optimization for faster recompiles, because we recompile dozens of times an hour.

> **What would break if we did it the obvious-but-wrong way?**
> If you forgot `features = ["macros"]` on sqlx, `sqlx::query!` would not exist, and you'd have to use the un-checked `sqlx::query` string. Typos in SQL would explode at runtime instead of compile time.

## A.4. The first migration

Create file `projects/01-todo/backend/migrations/0001_init.sql`:

```sql
CREATE TABLE IF NOT EXISTS todos (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL CHECK (length(trim(title)) > 0 AND length(title) <= 200),
    done        INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_todos_created_at ON todos (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_todos_done ON todos (done);
```

### Line-by-line

- `CREATE TABLE IF NOT EXISTS todos (...)` — make a table called `todos`. The `IF NOT EXISTS` makes the migration idempotent: running it twice doesn't crash. (sqlx's migrator already tracks "have I run this?" but belt-and-suspenders.)
- `id TEXT PRIMARY KEY` — SQLite has no native UUID type, so we store UUIDs as text. Primary key implies unique + non-null + indexed.
- `title TEXT NOT NULL CHECK (length(trim(title)) > 0 AND length(title) <= 200)` — a **database-level constraint**. Even if our application code has a bug, the DB refuses to store an empty or huge title. The database is your last line of defense; never skip CHECK constraints.
- `done INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1))` — SQLite has no boolean type. We use 0/1 with a CHECK to prevent garbage values.
- `created_at TEXT NOT NULL DEFAULT (strftime(...))` — timestamps as ISO-8601 strings. SQLite has no real datetime type either; text-based ISO-8601 sorts lexicographically (correct ordering for free) and round-trips through JSON cleanly.
- `CREATE INDEX ... ON todos (created_at DESC)` — make "list newest first" fast as the table grows. Without it, every list query becomes a full scan.
- `CREATE INDEX ... ON todos (done)` — supports filtering by `done = false`. Optional in v1; we add it now because it's free.

**Now apply the migration to a fresh database:**

```bash
sqlite3 todo.db < migrations/0001_init.sql
```

You need a DB *before* you compile, because `sqlx::query!` connects to it at compile time to verify your SQL.

## A.5. Errors that turn into HTTP responses

Create file `projects/01-todo/backend/src/error.rs`:

```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            AppError::Database(err) => {
                tracing::error!(error = ?err, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": self.to_string(),
            }
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

### Line-by-line

- `use ...` — bring names into scope. `IntoResponse` is the Axum trait that says "I know how to become an HTTP response".
- `pub enum AppError { Validation(String), NotFound, Database(#[from] sqlx::Error) }`
  - `pub` — visible to other files in this crate.
  - `enum` — a sum type. An `AppError` is **exactly one of** these variants.
  - `#[derive(thiserror::Error)]` — automatically implements `std::error::Error` for this enum.
  - `#[error("validation: {0}")]` — what the error prints as.
  - `#[error(transparent)] Database(#[from] sqlx::Error)` — a sqlx error becomes an `AppError` automatically (via the `?` operator). Without `#[from]` you'd have to write `.map_err(AppError::Database)?` on every query call.
- `impl IntoResponse for AppError` — the magic of Axum. Any handler that returns `Result<T, AppError>` Just Works because we've told the framework how to turn our errors into HTTP responses.
  - The `match` decides status code + machine-readable code based on the variant.
  - For `Database`, we **log the real error** (`tracing::error!`) and respond with a generic message. This is critical: never expose internal error messages to clients. They can leak schema, file paths, even credentials.
  - The body is a JSON envelope `{"error": {"code": "...", "message": "..."}}`. A stable shape clients can rely on.
- `pub type AppResult<T> = Result<T, AppError>;` — a type alias. Saves typing later.

> **Why this pattern is Principal-level:** every Axum handler returns `AppResult<T>`. There's *one* place that knows how errors become responses. Add a new error variant → only one match needs updating. This is the single-responsibility principle applied at the framework boundary.

## A.6. The database connection pool

Create file `projects/01-todo/backend/src/db.rs`:

```rust
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;

pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
```

### Line-by-line

- `pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error>` — async function returning a connection pool.
- `SqliteConnectOptions::from_str(url)?` — parse a connection string like `sqlite:./todo.db`.
- `.create_if_missing(true)` — if the file doesn't exist, create it. Convenient for first run.
- `.journal_mode(SqliteJournalMode::Wal)` — **WAL (Write-Ahead Log)**. This is the most important SQLite knob you'll ever turn. WAL lets readers and a single writer work concurrently without blocking each other. Without WAL, every write locks the entire DB. Production SQLite always uses WAL.
- `.synchronous(SqliteSynchronous::Normal)` — fsync after every transaction. `Normal` is the WAL-safe default. `Full` is slower; `Off` is dangerous.
- `.busy_timeout(Duration::from_secs(5))` — if another connection holds a lock, wait up to 5s before erroring. Prevents `SQLITE_BUSY` flakes under load.
- `.foreign_keys(true)` — SQLite has foreign keys but they're **off by default** (historical reasons). Always turn them on.
- `SqlitePoolOptions::new().max_connections(8)` — a connection pool. Even SQLite benefits — opening a connection has overhead, and the pool serializes writers safely.
- `.acquire_timeout(Duration::from_secs(3))` — if the pool is exhausted, fail fast.
- `sqlx::migrate!("./migrations").run(&pool).await?` — embed every `.sql` file in `./migrations/` into the binary at compile time, then run any that haven't been applied. The macro reads the folder at compile time, so a missing migration is a compile error, not a runtime surprise.

## A.7. The HTTP handlers

This is the largest file. Take it in chunks.

Create file `projects/01-todo/backend/src/routes/mod.rs`:

```rust
pub mod todos;
```

This is the **module index**. `mod todos;` tells Rust "there's a file `todos.rs` (or folder `todos/mod.rs`) in this directory; please load it as a submodule."

Now create file `projects/01-todo/backend/src/routes/todos.rs`:

```rust
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTodo {
    pub title: Option<String>,
    pub done: Option<bool>,
}

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", patch(update).delete(delete))
}
```

### Line-by-line (so far)

- `use axum::extract::{Path, State};` — **extractors**. Axum handlers don't reach into the request manually; they declare what they want as function arguments, and Axum extracts it.
  - `Path<T>` — pull a path parameter like `/:id`.
  - `State<T>` — pull shared state (our DB pool).
- `use axum::routing::{get, patch};` — HTTP method routers.
- `use chrono::{DateTime, Utc};` — timestamps in UTC. Never store local times in a DB.
- `use crate::error::...` — `crate` means "the root of this crate" (i.e. `src/`).
- The three `struct`s are our **DTOs** (data transfer objects):
  - `Todo` — what we send to the client. `#[derive(Serialize)]` turns it into JSON.
  - `CreateTodo` — what the client sends to create. `#[derive(Deserialize)]` parses JSON.
  - `UpdateTodo` — for partial updates. `Option<T>` fields allow "leave unchanged".
- `pub fn router() -> Router<SqlitePool>` — returns the **subrouter** for `/api/todos`. We mount it from `main.rs`. The `<SqlitePool>` type parameter says "this router expects a SqlitePool as State".
  - `.route("/", get(list).post(create))` — GET `/` lists; POST `/` creates.
  - `.route("/{id}", patch(update).delete(delete))` — Axum 0.8 syntax. (Older Axum used `/:id`. Both work; `{id}` is the new default.)

Now the four handler functions:

```rust
async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<Todo>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, title, done, created_at, updated_at
        FROM todos
        ORDER BY done ASC, created_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let todos = rows
        .into_iter()
        .map(|r| Todo {
            id: r.id.expect("id is non-null primary key"),
            title: r.title,
            done: r.done != 0,
            created_at: parse_ts(&r.created_at),
            updated_at: parse_ts(&r.updated_at),
        })
        .collect();

    Ok(Json(todos))
}
```

#### Line-by-line — `list`

- `async fn list(State(pool): State<SqlitePool>) -> AppResult<Json<Vec<Todo>>>` — the signature *is* the documentation. Inputs: state. Output: a JSON array of Todos, or an `AppError`.
- `sqlx::query!(r#"SELECT ..."#)` — the macro. At compile time, sqlx connects to `DATABASE_URL`, prepares this SQL, and figures out the column types. The returned struct has fields `r.id`, `r.title`, etc. **If you mistype `SELECT titel`, the build fails.**
  - `r#"..."#` is a "raw string" — no need to escape inner quotes.
- `.fetch_all(&pool).await?` — run the query against the pool, collect all rows. `await?` is "do the async thing, propagate the error".
- `ORDER BY done ASC, created_at DESC` — unchecked items first (done = 0 < 1), then newest first within each group. UX choice baked into SQL.
- `r.id.expect("id is non-null primary key")` — SQLite reports columns as nullable to sqlx unless you tell it otherwise. We know the column is `NOT NULL PRIMARY KEY`, so an `Option::None` here would be a *bug*, not a recoverable error. `expect` panics with a clear message — exactly what we want.
- `r.done != 0` — convert SQLite's 0/1 integer to a Rust `bool`.
- `Ok(Json(todos))` — wrap our Vec in `Json`, which Axum serializes and sets `Content-Type: application/json`.

```rust
async fn create(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateTodo>,
) -> AppResult<(StatusCode, Json<Todo>)> {
    let title = normalize_title(&payload.title)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = format_ts(now);

    sqlx::query!(
        r#"
        INSERT INTO todos (id, title, done, created_at, updated_at)
        VALUES (?1, ?2, 0, ?3, ?3)
        "#,
        id,
        title,
        now_str,
    )
    .execute(&pool)
    .await?;

    let todo = Todo {
        id,
        title,
        done: false,
        created_at: now,
        updated_at: now,
    };

    Ok((StatusCode::CREATED, Json(todo)))
}
```

#### Line-by-line — `create`

- `Json(payload): Json<CreateTodo>` — extract a JSON body. If the body is missing or malformed, Axum responds 400 *before* the handler runs.
- `normalize_title(&payload.title)?` — validate + clean (defined at the bottom of the file). The `?` propagates `AppError::Validation` if the title is bad.
- `Uuid::new_v4().to_string()` — random UUID, formatted as text for SQLite.
- `Utc::now()` — current UTC instant. We compute it *once* and reuse, so `created_at == updated_at` on insert (same millisecond).
- `format_ts(now)` — convert to ISO-8601 string (the format SQLite stores).
- `?1, ?2, ?3` — positional bind parameters. The values are passed by name after the SQL string. **Never** concatenate user input into SQL; that's how you get SQL injection. The macro guarantees parameterization.
- `(StatusCode::CREATED, Json(todo))` — return a tuple. Axum interprets a tuple as `(status, body)`. `201 Created` is the semantically correct status for a successful POST.

```rust
async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTodo>,
) -> AppResult<Json<Todo>> {
    let now = Utc::now();
    let now_str = format_ts(now);

    let title = payload.title.as_deref().map(normalize_title).transpose()?;
    let done = payload.done.map(|d| if d { 1 } else { 0 });

    let row = sqlx::query!(
        r#"
        UPDATE todos
        SET title       = COALESCE(?1, title),
            done        = COALESCE(?2, done),
            updated_at  = ?3
        WHERE id = ?4
        RETURNING id, title, done, created_at, updated_at
        "#,
        title,
        done,
        now_str,
        id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(Todo {
        id: row.id.expect("id is non-null primary key"),
        title: row.title,
        done: row.done != 0,
        created_at: parse_ts(&row.created_at),
        updated_at: parse_ts(&row.updated_at),
    }))
}
```

#### Line-by-line — `update`

- We accept **two** extractors in addition to State: the path parameter `:id` and the JSON body. The order matches Axum's body-consuming rule (body extractors come last).
- `payload.title.as_deref().map(normalize_title).transpose()?` — read this from the inside out:
  - `payload.title` is `Option<String>`.
  - `.as_deref()` turns `Option<String>` into `Option<&str>` (cheaper to pass).
  - `.map(normalize_title)` runs validation only if present; gives `Option<Result<String, AppError>>`.
  - `.transpose()` flips it to `Result<Option<String>, AppError>`.
  - `?` propagates the validation error.
- `payload.done.map(|d| if d { 1 } else { 0 })` — `Option<bool>` → `Option<i32>` (SQLite stores 0/1).
- `COALESCE(?1, title)` — SQL's "first non-null wins". If we pass `NULL` for `?1`, the column keeps its old value. This is the **partial update** trick: one SQL statement handles "update title only", "update done only", or "update both".
- `RETURNING ...` — SQLite 3.35+ feature. Same statement does the write **and** returns the updated row, so no second SELECT.
- `.fetch_optional(...).await?.ok_or(AppError::NotFound)?` — if the WHERE matched nothing, `fetch_optional` gives `Ok(None)`. We turn that into our `NotFound` variant. Result: missing ID → HTTP 404.

```rust
async fn delete(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let result = sqlx::query!("DELETE FROM todos WHERE id = ?1", id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
```

#### Line-by-line — `delete`

- `.execute(&pool)` instead of `.fetch_all`: we don't care about returned rows, just side effects.
- `result.rows_affected() == 0` — SQL `DELETE` returns success even if nothing matched. We turn "nothing deleted" into 404 so the client knows.
- `StatusCode::NO_CONTENT` — `204`. Convention for "I did the thing, there's no body to send back".

```rust
fn normalize_title(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("title must not be empty".into()));
    }
    if trimmed.chars().count() > 200 {
        return Err(AppError::Validation("title must be 200 chars or fewer".into()));
    }
    Ok(trimmed.to_string())
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
```

#### Line-by-line — helpers

- `normalize_title` — single chokepoint for title rules. Used by both create and update. If we ever want to allow longer titles, we change it in one place.
- `trimmed.chars().count()` — counts **Unicode scalar values**, not bytes. So an emoji-rich title is judged by its visual length. Using `.len()` would count bytes, which would be wrong for non-ASCII.
- `format_ts` — controlled output format. Three-decimal milliseconds, `Z` suffix for UTC.
- `parse_ts` — defensive read. SQLite gave us text; if it's somehow malformed, we don't crash the whole list query — we fall back to `now`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_title_rejects_empty() {
        assert!(matches!(normalize_title("   "), Err(AppError::Validation(_))));
    }

    #[test]
    fn normalize_title_trims() {
        assert_eq!(normalize_title("  buy milk  ").unwrap(), "buy milk");
    }

    #[test]
    fn normalize_title_rejects_too_long() {
        let long = "a".repeat(201);
        assert!(matches!(normalize_title(&long), Err(AppError::Validation(_))));
    }
}
```

#### Line-by-line — tests

- `#[cfg(test)]` — this whole module is only compiled when running `cargo test`. Zero impact on production binary size.
- `mod tests` — a submodule for tests. `use super::*` imports everything from the parent.
- `#[test]` — marks a function as a test. `cargo test` discovers them automatically.
- `matches!(value, pattern)` — pattern-match without writing a full `match`. Returns `bool`.
- Each test exercises **one** assertion. If a test fails, the name tells you exactly what broke.

Run them:

```bash
DATABASE_URL=sqlite:./todo.db cargo test
# → 3 passed
```

## A.8. The Axum bootstrap

Create file `projects/01-todo/backend/src/main.rs`:

```rust
mod db;
mod error;
mod routes;

use axum::http::{HeaderValue, Method};
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "todo_backend=info,tower_http=info".into()))
        .with_target(false)
        .compact()
        .init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://todo.db".to_string());
    let frontend_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);

    let pool = db::connect(&db_url).await?;
    tracing::info!(db = %db_url, "database ready");

    let cors = CorsLayer::new()
        .allow_origin(frontend_origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .nest("/api/todos", routes::todos::router())
        .route("/healthz", axum::routing::get(health))
        .with_state(pool)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("ctrl-c received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
```

### Line-by-line

- `mod db; mod error; mod routes;` — declare submodules so Rust loads the files we wrote.
- `#[tokio::main]` — a macro that wraps our `async fn main` in a Tokio runtime. Without this, `await` doesn't work in `main`.
- `Result<(), Box<dyn std::error::Error>>` — main can return any error. Useful while bootstrapping.
- `tracing_subscriber::fmt()...init()` — set up logging. Reads `RUST_LOG` from env; defaults to `info` for our crate and tower-http.
- `.with_target(false).compact()` — terser console output.
- `std::env::var("DATABASE_URL")` — 12-factor config: read from env, default sensibly.
- `db::connect(&db_url).await?` — opens the pool **and runs migrations**. By the time this returns, the DB is ready.
- The `CorsLayer` — the browser will block the frontend from calling our backend unless we explicitly allow its origin. **This is a security feature**, not an annoyance.
  - `.allow_origin(frontend_origin.parse()?)` — exact match. `*` would also work but disables credentialed requests.
  - `.allow_methods([...])` — only the methods we use.
  - `.allow_credentials(true)` — needed once we have cookies (project 11 onward). Costs nothing now.
- `Router::new().nest("/api/todos", routes::todos::router())` — mount the todos subrouter at `/api/todos`. So inside `todos.rs` the routes are `/` and `/{id}` — the prefix is added here. Modular.
- `.route("/healthz", get(health))` — a "is the process alive" probe. Containers and load balancers ping this.
- `.with_state(pool)` — make the SqlitePool available to every handler that asks for `State<SqlitePool>`.
- `.layer(TraceLayer::new_for_http())` — log every request and response with timing.
- `.layer(cors)` — apply CORS to all routes. Order matters: layers are applied **outside-in** (last added wraps first).
- `tokio::net::TcpListener::bind(addr).await?` — open the socket.
- `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?` — start serving. The `with_graceful_shutdown` makes the server finish in-flight requests when Ctrl+C / SIGTERM arrives, instead of dropping connections.
- `shutdown_signal()` — wait for whichever comes first: Ctrl+C or SIGTERM. `tokio::select!` is the async equivalent of "race these futures".

> **What would break if we did it the obvious-but-wrong way?**
> If we skipped graceful shutdown, every Ctrl+C in development would drop in-flight requests and corrupt a test client mid-response. In production, the next deploy would kill open requests instead of letting them finish.

## A.9. Environment example

Create file `projects/01-todo/backend/.env.example`:

```
DATABASE_URL=sqlite://todo.db
FRONTEND_ORIGIN=http://localhost:5173
PORT=3000
RUST_LOG=todo_backend=debug,tower_http=info
```

This file is **committed**. Its real sibling, `.env`, is **gitignored**. The example documents which env vars exist without leaking values. New contributors `cp .env.example .env` and fill in their own.

## A.10. Run it and poke it

```bash
DATABASE_URL=sqlite:./todo.db cargo run
```

In another terminal:

```bash
curl -X POST http://localhost:3000/api/todos -H 'content-type: application/json' -d '{"title":"buy milk"}'
curl http://localhost:3000/api/todos
```

Done. The backend is real. Tested. Logging. Graceful. Let's build the UI.

---

# Part B — The frontend

## B.1. Scaffold SvelteKit

From `projects/01-todo/`:

```bash
pnpm dlx sv@latest create --template minimal --types ts --no-add-ons --no-install frontend
cd frontend
```

`sv` is the official Svelte CLI. The flags:

- `--template minimal` — bare-bones starter (no demo blog).
- `--types ts` — TypeScript.
- `--no-add-ons` — we'll add Vitest/Playwright manually so you see exactly what gets added.
- `--no-install` — don't run `pnpm install` yet; we want to edit `package.json` first.

## B.2. Adjust dependencies

`sv create` adds `@sveltejs/adapter-auto`, which guesses a deploy target. We want to be explicit: Node, because that's what we run in Docker and in the e2e tests.

Edit `projects/01-todo/frontend/package.json` to replace the dev dependencies with this list (the exact versions may differ; pnpm will pick the latest matching `^`):

```json
{
  "name": "todo-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "prepare": "svelte-kit sync || echo ''",
    "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
    "test:unit": "vitest run",
    "test:e2e": "playwright test"
  },
  "devDependencies": {
    "@playwright/test": "^1.60.0",
    "@sveltejs/adapter-node": "^5.2.13",
    "@sveltejs/kit": "^2.57.0",
    "@sveltejs/vite-plugin-svelte": "^7.0.0",
    "@testing-library/jest-dom": "^6.9.1",
    "@testing-library/svelte": "^5.3.1",
    "@types/node": "^25.9.1",
    "@vitest/browser": "^4.1.7",
    "jsdom": "^29.1.1",
    "playwright": "^1.60.0",
    "svelte": "^5.55.2",
    "svelte-check": "^4.4.6",
    "typescript": "^6.0.2",
    "vite": "^8.0.7",
    "vitest": "^4.1.7"
  },
  "dependencies": {
    "phosphor-svelte": "^3.0.1"
  }
}
```

Then install:

```bash
pnpm install
pnpm exec playwright install chromium
```

### What each dep does (the short version)

- `@sveltejs/kit` — the framework itself (router, SSR, form actions, remote functions).
- `svelte` — the compiler. Svelte is unusual: there is no runtime "virtual DOM". The compiler turns components into surgical DOM updates.
- `@sveltejs/adapter-node` — at build time, bundles your app into a Node.js HTTP server (`build/index.js`).
- `@sveltejs/vite-plugin-svelte` — teaches Vite how to compile `.svelte` files.
- `vite` — the dev server and bundler. Sub-second hot reload.
- `typescript` + `svelte-check` — type-checking. `svelte-check` walks every `.svelte` file and runs the TS compiler.
- `phosphor-svelte` — the icon set. Each icon is a Svelte component you import by name. No external SVG files.
- `vitest`, `@vitest/browser`, `jsdom`, `@testing-library/*` — the test pyramid's lower layers.
- `playwright`, `@playwright/test` — real browser, end-to-end.
- `@types/node` — Node's type definitions, used by the test runner and adapter.

## B.3. SvelteKit config

The scaffolder gave us `svelte.config.js`. Edit `projects/01-todo/frontend/svelte.config.js` so it reads:

```js
import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  compilerOptions: {
    runes: ({ filename }) => (filename.split(/[/\\]/).includes('node_modules') ? undefined : true)
  },
  kit: {
    adapter: adapter()
  }
};

export default config;
```

### Line-by-line

- `import adapter from '@sveltejs/adapter-node';` — node adapter.
- `vitePreprocess()` — runs files through Vite's TS / PostCSS / Sass pipelines before Svelte compiles them. Needed for `<script lang="ts">`.
- `compilerOptions.runes` — **force runes mode** in every file in our project (`$state`, `$derived`, `$effect`), but **not** in `node_modules` (third-party Svelte components may still be in legacy mode). This is exactly the recommended Svelte 5 config.
- `kit.adapter: adapter()` — at build time, output a Node server.

## B.4. TypeScript config

Edit `projects/01-todo/frontend/tsconfig.json` to opt into the strictest settings:

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "exclude": ["e2e/**"],
  "compilerOptions": {
    "rewriteRelativeImportExtensions": true,
    "allowJs": true,
    "checkJs": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "sourceMap": true,
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "moduleResolution": "bundler"
  }
}
```

### Why these flags matter

- `strict: true` — the umbrella for all strictness flags. Treats `null` and `undefined` as distinct, refuses implicit `any`.
- `noUncheckedIndexedAccess: true` — accessing `arr[0]` returns `T | undefined`, not `T`. Forces you to handle "what if the array is empty?" Catches a real class of production bugs.
- `noImplicitOverride: true` — must say `override` when overriding a class method. Catches typos.
- `exclude: ["e2e/**"]` — Playwright tests have their own TS context; we don't want `svelte-check` to lint them.

## B.5. The HTML shell

The scaffolder gave us `src/app.html`. Edit `projects/01-todo/frontend/src/app.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="color-scheme" content="light dark" />
    <link
      rel="icon"
      href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'%3E%3Ctext y='.9em' font-size='90'%3E%E2%9C%93%3C/text%3E%3C/svg%3E"
    />
    %sveltekit.head%
  </head>
  <body data-sveltekit-preload-data="hover">
    <div style="display: contents">%sveltekit.body%</div>
  </body>
</html>
```

### Line-by-line

- `<html lang="en">` — accessibility & SEO. Screen readers and Google both care about the language.
- `<meta name="viewport" ...>` — mobile responsiveness lives or dies on this line. Without it, mobile browsers pretend they're 980 px wide.
- `<meta name="color-scheme" content="light dark">` — tells the browser our CSS handles both. Avoids the brief flash of white when a dark-mode user loads the page.
- `<link rel="icon" href="data:image/svg+xml,...">` — a tiny favicon as a **data URL**. It's an inline SVG of a checkmark (`✓` = `%E2%9C%93` URL-encoded). The user's rule said "no external SVG icons" for the UI; a favicon is browser chrome, not UI, and an inline data URL means zero extra HTTP requests.
- `%sveltekit.head%` — SvelteKit injects per-page `<svelte:head>` content here.
- `data-sveltekit-preload-data="hover"` — when the user hovers a link, SvelteKit prefetches its data so the click feels instant.
- `<div style="display: contents">` — a wrapper that doesn't actually create a box. SvelteKit needs a root, but `display: contents` makes children behave as if there's no wrapper at all.

## B.6. The design system

Create file `projects/01-todo/frontend/src/app.css`:

```css
@import '../../../../shared/design-tokens.css';

@layer reset, base, components, utilities;

@layer reset {
  *,
  *::before,
  *::after {
    box-sizing: border-box;
  }

  * { margin: 0; }

  html, body { height: 100%; }

  body {
    line-height: var(--leading-normal);
    -webkit-font-smoothing: antialiased;
  }

  img, picture, video, canvas, svg {
    display: block;
    max-width: 100%;
  }

  input, button, textarea, select {
    font: inherit;
    color: inherit;
  }

  button {
    cursor: pointer;
    background: none;
    border: none;
    padding: 0;
  }

  p, h1, h2, h3, h4, h5, h6 {
    overflow-wrap: break-word;
  }

  :focus-visible {
    outline: 3px solid var(--color-focus);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }
}

@layer base {
  body {
    font-family: var(--font-sans);
    font-size: var(--text-base);
    color: var(--color-fg);
    background: var(--color-bg);
  }

  ::selection {
    background: var(--color-accent);
    color: var(--color-accent-fg);
  }
}
```

### Line-by-line — the modern CSS reset

- `@import '../../../../shared/design-tokens.css';` — the design-token file at the monorepo root. Shared colors, type scale, spacing, etc. We import it from `shared/` so all 30 projects look like they belong to the same product family.
- `@layer reset, base, components, utilities;` — declare the **cascade layer order**. Anything in `reset` is overridden by `base`, then `components`, then `utilities`. This kills specificity wars dead. Layers are the most important CSS feature of the last decade.
- The reset (Josh Comeau-style):
  - `box-sizing: border-box` — padding and border are inside the declared width. Sane.
  - `* { margin: 0 }` — explicit margins only, no inherited surprises.
  - `html, body { height: 100% }` — fills the viewport, enables `100vh`-style layouts.
  - `line-height: var(--leading-normal)` — 1.5 by default; comfortable to read.
  - `img, picture, ...` are `display: block` — kills the mysterious 4-px gap below images.
  - `input, button, ...` inherit font — by default they use the platform font, which clashes with your design.
  - `:focus-visible` — keyboard-only focus ring. Mouse clicks don't trigger this; only Tab does. The single most important accessibility primitive.

The base layer just applies our tokens to `body` and styles text selection. Tiny because most styling lives in component scopes.

## B.7. Type definitions

Create file `projects/01-todo/frontend/src/lib/types.ts`:

```ts
export type Todo = {
  id: string;
  title: string;
  done: boolean;
  created_at: string;
  updated_at: string;
};

export type ApiError = {
  error: {
    code: string;
    message: string;
  };
};
```

### Why

These types mirror what the Rust backend sends. **The shape is the contract.** If we change the backend, TypeScript reminds us to update this file. (Later, in project 24, we'll generate these from an OpenAPI schema. For now, hand-typed is fine — there are five fields.)

The `$lib` folder is a SvelteKit convention: anything inside it can be imported as `import x from '$lib/foo'` from anywhere. Saves you from `../../../lib/foo` purgatory.

## B.8. The typed API client

Create file `projects/01-todo/frontend/src/lib/api.ts`:

```ts
import type { Todo, ApiError } from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3000';

type FetchLike = typeof fetch;

async function request<T>(fetcher: FetchLike, path: string, init?: RequestInit): Promise<T> {
  const res = await fetcher(`${API_BASE}${path}`, {
    ...init,
    headers: {
      'content-type': 'application/json',
      accept: 'application/json',
      ...(init?.headers ?? {})
    }
  });

  if (res.status === 204) {
    return undefined as T;
  }

  const body = await res.json().catch(() => null);

  if (!res.ok) {
    const err = body as ApiError | null;
    throw new ApiCallError(err?.error?.message ?? `request failed: ${res.status}`, res.status);
  }

  return body as T;
}

export class ApiCallError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}

export const todosApi = {
  list: (fetcher: FetchLike) => request<Todo[]>(fetcher, '/api/todos'),

  create: (fetcher: FetchLike, title: string) =>
    request<Todo>(fetcher, '/api/todos', {
      method: 'POST',
      body: JSON.stringify({ title })
    }),

  update: (fetcher: FetchLike, id: string, patch: Partial<Pick<Todo, 'title' | 'done'>>) =>
    request<Todo>(fetcher, `/api/todos/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(patch)
    }),

  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/todos/${encodeURIComponent(id)}`, { method: 'DELETE' })
};
```

### Line-by-line

- `import.meta.env.VITE_BACKEND_URL` — Vite exposes any env var prefixed with `VITE_` to the client. Default to localhost for dev.
- `type FetchLike = typeof fetch;` — we accept any `fetch`-compatible function. **Why?** In SvelteKit `load` functions, you get a special `fetch` that proxies cookies correctly during SSR. We want to use *that* fetch on the server and the global `fetch` in the browser. By accepting it as an argument, we work in both contexts.
- `request<T>(...)` — generic helper. The `<T>` is the expected response body type.
- `Promise<T>` return + the explicit error class — callers get either typed data or a typed error. No `any`.
- `if (res.status === 204) return undefined as T;` — `DELETE` returns 204 No Content with no body. Calling `res.json()` on that would throw.
- `await res.json().catch(() => null)` — defensive parse. A malformed body shouldn't crash the whole flow.
- `if (!res.ok)` — `res.ok` is true for status 200–299.
- `class ApiCallError extends Error` — a real error type. `instanceof ApiCallError` works. `try { ... } catch (e) { if (e instanceof ApiCallError) ... }` is the pattern callers use.
- `todosApi.list/create/update/remove` — the public surface. We export an object so call sites read `todosApi.create(fetch, 'buy milk')` — self-documenting.
- `encodeURIComponent(id)` — UUIDs are URL-safe, but if you ever change to non-UUID IDs containing `/` or spaces, this saves you from a subtle bug.

## B.9. Test the API client

Create file `projects/01-todo/frontend/src/lib/api.test.ts`:

```ts
import { describe, it, expect, vi } from 'vitest';
import { todosApi, ApiCallError } from './api';
import type { Todo } from './types';

function makeFetch(response: { status: number; body: unknown }) {
  return vi.fn(
    async () =>
      new Response(response.body == null ? null : JSON.stringify(response.body), {
        status: response.status,
        headers: { 'content-type': 'application/json' }
      })
  );
}

describe('todosApi', () => {
  it('list parses an array of todos', async () => {
    const sample: Todo[] = [
      { id: '1', title: 'buy milk', done: false,
        created_at: '2026-05-26T12:00:00Z', updated_at: '2026-05-26T12:00:00Z' }
    ];
    const fetcher = makeFetch({ status: 200, body: sample });
    await expect(todosApi.list(fetcher)).resolves.toEqual(sample);
  });

  it('throws ApiCallError with status on 422', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: { error: { code: 'validation_failed', message: 'title must not be empty' } }
    });
    await expect(todosApi.create(fetcher, '')).rejects.toBeInstanceOf(ApiCallError);
    await expect(todosApi.create(fetcher, '')).rejects.toMatchObject({
      status: 422, message: 'title must not be empty'
    });
  });

  it('returns void on 204 delete', async () => {
    const fetcher = vi.fn(async () => new Response(null, { status: 204 }));
    await expect(todosApi.remove(fetcher, 'x')).resolves.toBeUndefined();
  });

  it('encodes the id in the path', async () => {
    const fetcher = makeFetch({ status: 204, body: null });
    await todosApi.remove(fetcher, 'a/b c');
    const url = fetcher.mock.calls[0]?.[0] as string;
    expect(url).toContain('/api/todos/a%2Fb%20c');
  });
});
```

### Line-by-line — the test ideas

- `vi.fn(...)` — a mock function. Records calls, returns whatever the implementation returns.
- `new Response(body, { status, headers })` — we hand-craft fetch responses. No network involved.
- `expect(...).resolves.toEqual(sample)` — async-aware assertion: "this promise resolves to this value".
- `expect(...).rejects.toBeInstanceOf(...)` — same for thrown errors.
- `fetcher.mock.calls[0]?.[0]` — the first argument of the first call (the URL). `?.` because `noUncheckedIndexedAccess` makes the array access `T | undefined`.

We test the **client**, not the network. Network tests live in the e2e suite.

Set up the test runner — create file `projects/01-todo/frontend/src/test-setup.ts`:

```ts
import '@testing-library/jest-dom/vitest';
```

One line: brings in matchers like `toBeVisible()`, `toBeInTheDocument()`. Used by future component tests.

And configure Vite — edit `projects/01-todo/frontend/vite.config.ts`:

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],

  test: {
    include: ['src/**/*.test.ts'],
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts']
  }
});
```

- `plugins: [sveltekit()]` — Vite + SvelteKit integration.
- `test.include` — only files matching this pattern are tests.
- `environment: 'jsdom'` — gives our tests a fake DOM (browser globals like `Response`, `Headers`). Fast.
- `setupFiles` — run this before each test file.

Run the tests:

```bash
pnpm test:unit
# → Tests 4 passed
```

## B.10. The Icon wrapper

Create file `projects/01-todo/frontend/src/lib/components/Icon.svelte`:

```svelte
<script lang="ts">
  import type { Component } from 'svelte';

  type Props = {
    icon: Component<{ size?: number | string; color?: string; weight?: 'thin' | 'light' | 'regular' | 'bold' | 'fill' | 'duotone' }>;
    size?: number;
    weight?: 'thin' | 'light' | 'regular' | 'bold' | 'fill' | 'duotone';
    color?: string;
    label?: string;
  };

  let { icon: IconComponent, size = 20, weight = 'regular', color = 'currentColor', label }: Props = $props();
</script>

<span class="icon-wrap" aria-hidden={label ? undefined : 'true'}>
  {#if label}
    <span class="sr-only">{label}</span>
  {/if}
  <IconComponent {size} {weight} {color} />
</span>

<style>
  .icon-wrap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
```

### Line-by-line — first encounter with Svelte 5 runes

- `<script lang="ts">` — TypeScript inside the component.
- `import type { Component } from 'svelte';` — the type for any Svelte component. Type-only import; doesn't ship to the runtime.
- `type Props = { ... };` — the shape of what this component accepts.
  - `icon: Component<{ size?: ...; color?: ...; weight?: ... }>` — the icon component itself, typed by the props *it* accepts. So you pass `Plus`, `Check`, `Trash` etc. from `phosphor-svelte` and TS knows which props go through.
  - `label?: string` — optional accessible label. If present, screen readers read it; if absent, the icon is decorative and we set `aria-hidden`.
- `let { icon: IconComponent, size = 20, ... }: Props = $props();` — **the `$props()` rune**. The single source of "what was passed in". Default values right in the destructure. The rename `icon: IconComponent` lets us use it as `<IconComponent />` in the markup (component names must be PascalCase).
- `<span class="icon-wrap" aria-hidden={label ? undefined : 'true'}>` — the wrap solves a Phosphor TS quirk: Phosphor's prop type doesn't expose `aria-*` props, so we hoist accessibility one level up.
- `{#if label}<span class="sr-only">{label}</span>{/if}` — the **`{#if}` block**. Conditional rendering. The `sr-only` class hides text visually but keeps it readable by screen readers — the standard a11y idiom.
- `<IconComponent {size} {weight} {color} />` — render the Phosphor icon. The `{size}` shorthand is sugar for `size={size}`.

**This component is generic and reusable for the rest of the curriculum.** All 30 projects use this wrapper.

## B.11. The TodoItem component

Create file `projects/01-todo/frontend/src/lib/components/TodoItem.svelte`:

```svelte
<script lang="ts">
  import { Check, Trash } from 'phosphor-svelte';
  import Icon from './Icon.svelte';
  import type { Todo } from '$lib/types';

  type Props = {
    todo: Todo;
    onToggle: (id: string, done: boolean) => void;
    onDelete: (id: string) => void;
  };

  let { todo, onToggle, onDelete }: Props = $props();

  let pending = $state(false);

  async function toggle() {
    pending = true;
    try {
      onToggle(todo.id, !todo.done);
    } finally {
      pending = false;
    }
  }

  async function remove() {
    pending = true;
    try {
      onDelete(todo.id);
    } finally {
      pending = false;
    }
  }
</script>

<li class="todo" class:done={todo.done} class:pending>
  <button
    type="button"
    class="checkbox"
    aria-pressed={todo.done}
    aria-label={todo.done ? `Mark "${todo.title}" as not done` : `Mark "${todo.title}" as done`}
    onclick={toggle}
    disabled={pending}
  >
    {#if todo.done}
      <Icon icon={Check} size={16} weight="bold" />
    {/if}
  </button>

  <span class="title">{todo.title}</span>

  <button
    type="button"
    class="remove"
    aria-label={`Delete "${todo.title}"`}
    onclick={remove}
    disabled={pending}
  >
    <Icon icon={Trash} size={18} />
  </button>
</li>

<style>
  /* ... see source file for full styles ... */
</style>
```

(The `<style>` block is in the source file. We covered the design-token approach in B.6.)

### Line-by-line

- `import { Check, Trash } from 'phosphor-svelte';` — tree-shaken icon imports. Only `Check` and `Trash` end up in your bundle.
- `import type { Todo } from '$lib/types';` — the `$lib` alias.
- `type Props = { todo: Todo; onToggle: (id, done) => void; onDelete: (id) => void; };` — **callback props**. The parent passes functions in; the child calls them. This is the Svelte 5 idiom — the old `createEventDispatcher` is legacy.
- `let pending = $state(false);` — **the `$state()` rune**. Reactive variable. When `pending = true`, every DOM expression that reads `pending` re-renders.
- `class:done={todo.done} class:pending` — the **`class:` directive**. Adds/removes the class based on truthiness. `class:pending` (no value) is shorthand for `class:pending={pending}`. Way cleaner than building a template string.
- `aria-pressed={todo.done}` — ARIA toggle button pattern. Screen readers announce "pressed" / "not pressed". We use a `<button>` instead of a `<input type="checkbox">` because a styled button gives us the icon + spacing we want without hidden-input tricks. The ARIA attributes preserve the semantics.
- `aria-label={...}` — dynamic, includes the todo title so screen readers read "Mark 'buy milk' as done", not just "Mark as done".
- `onclick={toggle}` — Svelte 5 event syntax. (The legacy `on:click` still works but is deprecated.)
- `disabled={pending}` — visual + interactive lock during the round-trip.
- `{#if todo.done}<Icon .../>{/if}` — the checkmark only renders when done.

> **What would break if we did it the obvious-but-wrong way?**
> If you used `createEventDispatcher`, the parent would write `<TodoItem on:toggle={handle} />`. That still works but skips the type safety of callback props — TypeScript can't check the event payload shape as precisely.

## B.12. The layout

Edit `projects/01-todo/frontend/src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import '../app.css';

  let { children } = $props();
</script>

<svelte:head>
  <title>TODO Manager</title>
  <meta
    name="description"
    content="A tiny, fast, distraction-free TODO app. Add tasks, check them off, get them out of your head."
  />
</svelte:head>

{@render children()}
```

### Line-by-line

- `import '../app.css';` — global stylesheet. Importing in the layout means it loads once for every page.
- `let { children } = $props();` — every layout receives `children` from SvelteKit: the page rendered inside it.
- `<svelte:head>` — write into the document `<head>`. Title and description are the basic SEO baseline (we'll go deeper in project 2).
- `{@render children()}` — **the snippet render directive**. The page's content is a snippet; this is where we paint it. (Replaces `<slot />` from Svelte 4.)

## B.13. The page server module

Create file `projects/01-todo/frontend/src/routes/+page.server.ts`:

```ts
import { fail } from '@sveltejs/kit';
import { todosApi, ApiCallError } from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const todos = await todosApi.list(fetch);
  return { todos };
};

export const actions: Actions = {
  create: async ({ request, fetch }) => {
    const data = await request.formData();
    const title = String(data.get('title') ?? '').trim();

    if (!title) {
      return fail(422, { title, error: 'Title must not be empty.' });
    }
    if (title.length > 200) {
      return fail(422, { title, error: 'Title must be 200 characters or fewer.' });
    }

    try {
      await todosApi.create(fetch, title);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { title, error: err.message });
      }
      throw err;
    }
  },

  toggle: async ({ request, fetch }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    const done = data.get('done') === 'true';
    if (!id) return fail(422, { error: 'Missing id.' });
    try {
      await todosApi.update(fetch, id, { done });
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  },

  remove: async ({ request, fetch }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    if (!id) return fail(422, { error: 'Missing id.' });
    try {
      await todosApi.remove(fetch, id);
      return { success: true };
    } catch (err) {
      if (err instanceof ApiCallError) return fail(err.status, { error: err.message });
      throw err;
    }
  }
};
```

### Line-by-line — `load` and `actions`

- `export const load: PageServerLoad` — runs on the **server** before the page is rendered. Returns data the page can use as `data.todos`. Because this is `+page.server.ts` (not `+page.ts`), it never runs in the browser. Safe place for secrets.
- `({ fetch })` — SvelteKit provides a special `fetch` that, during SSR, knows the request's cookies and proxies them to internal endpoints. We pass it straight to our `todosApi` client.
- `export const actions: Actions = { create, toggle, remove }` — **form actions**. The page can `POST` a form to `?/create`, `?/toggle`, `?/remove` and SvelteKit dispatches to the named function. No API route file needed.
- `request.formData()` — parses the `multipart/form-data` body. Even when JS is disabled, a plain HTML form posts here and works. **Progressive enhancement.**
- `String(data.get('title') ?? '').trim()` — `FormData.get` returns `string | File | null`. We coerce to a string and trim.
- `fail(422, { ... })` — SvelteKit's "validation failed" return. The page re-renders with `form = { error, title }` available in `+page.svelte`. **Critical**: `fail` is *not* a thrown error. It's a typed return. This is how SvelteKit keeps progressive enhancement and JS-enabled paths in sync.
- The `try/catch` translates a backend `ApiCallError` into the same `fail()` shape, so the user sees consistent error UI whether validation failed locally or remotely.

> **Why form actions instead of remote functions?** Remote functions (project 8 onward) are great for JS-driven calls. Form actions degrade gracefully when JS is off. Project 1 teaches the foundational pattern; project 8 adds remote functions when we need them.

## B.14. The page

Edit `projects/01-todo/frontend/src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import { enhance } from '$app/forms';
  import { Plus, ListChecks } from 'phosphor-svelte';
  import TodoItem from '$lib/components/TodoItem.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();

  const todos = $derived(data.todos);
  const remaining = $derived(todos.filter((t) => !t.done).length);
  const total = $derived(todos.length);

  let title = $state('');
  let submitting = $state(false);

  let toggleForm: HTMLFormElement | undefined = $state();
  let toggleId = $state('');
  let toggleDone = $state(false);

  let deleteForm: HTMLFormElement | undefined = $state();
  let deleteId = $state('');

  function handleToggle(id: string, done: boolean) {
    toggleId = id;
    toggleDone = done;
    queueMicrotask(() => toggleForm?.requestSubmit());
  }

  function handleDelete(id: string) {
    deleteId = id;
    queueMicrotask(() => deleteForm?.requestSubmit());
  }
</script>

<main>
  <header>
    <div class="brand">
      <Icon icon={ListChecks} size={28} weight="duotone" />
      <h1>Today</h1>
    </div>
    <p class="meta" aria-live="polite">{remaining} of {total} remaining</p>
  </header>

  <form
    method="POST"
    action="?/create"
    use:enhance={() => {
      submitting = true;
      return async ({ result, update }) => {
        submitting = false;
        if (result.type === 'success') title = '';
        await update();
      };
    }}
    class="add-form"
  >
    <input
      type="text"
      name="title"
      bind:value={title}
      placeholder="What needs to get done?"
      autocomplete="off"
      maxlength="200"
      required
      aria-label="New task"
    />
    <button type="submit" class="primary" disabled={submitting || title.trim().length === 0}>
      <Icon icon={Plus} size={18} weight="bold" />
      <span>Add</span>
    </button>
  </form>

  <form bind:this={toggleForm} method="POST" action="?/toggle"
        use:enhance={() => async ({ update }) => update()} hidden>
    <input type="hidden" name="id" value={toggleId} />
    <input type="hidden" name="done" value={String(toggleDone)} />
  </form>

  <form bind:this={deleteForm} method="POST" action="?/remove"
        use:enhance={() => async ({ update }) => update()} hidden>
    <input type="hidden" name="id" value={deleteId} />
  </form>

  {#if form && 'error' in form && form.error}
    <p class="error" role="alert">{form.error}</p>
  {/if}

  {#if todos.length === 0}
    <p class="empty">No tasks yet — add your first above.</p>
  {:else}
    <ul class="list">
      {#each todos as todo (todo.id)}
        <TodoItem {todo} onToggle={handleToggle} onDelete={handleDelete} />
      {/each}
    </ul>
  {/if}
</main>
```

### Line-by-line — runes in practice

- `import { enhance } from '$app/forms';` — SvelteKit's progressive-enhancement helper. Sprinkled on a `<form>` as `use:enhance`, it intercepts the submit, posts via fetch, and re-runs `load` automatically — **without** a full page reload.
- `let { data, form }: PageProps = $props();` — every page gets:
  - `data` — what `load` returned.
  - `form` — what the last action returned (e.g. `fail(...)` payload or success object). `null` on first load.
- `const todos = $derived(data.todos);` — **the `$derived()` rune**. A computed value. Whenever `data.todos` changes, `todos` re-derives. We could just use `data.todos` directly, but assigning to a local makes the template cleaner.
- `const remaining = $derived(todos.filter(...).length)` — derived from derived. Re-runs only when `todos` changes.
- `let title = $state('');` — the bound input value. Two-way bound below with `bind:value={title}`.
- `let toggleForm: HTMLFormElement | undefined = $state();` — a **ref** to a DOM element. We populate it with `bind:this`. Used to programmatically submit the hidden form.
- `queueMicrotask(() => toggleForm?.requestSubmit())` — set the form values **then** submit on the next microtask. Without the microtask, Svelte's reactivity hasn't yet updated the hidden `<input value={toggleId}>`, so the submission would carry the old value.
- `use:enhance={() => { ... return async ({ result, update }) => { ... } }}` — the enhance callback:
  - **Sync part** runs *before* the request fires. We set `submitting = true`, clear errors.
  - **Async part** runs *after* the request returns. `result` is the action's payload; `update()` re-runs `load`. If we don't call `update`, the page won't refresh.
- The two hidden forms (`toggleForm`, `deleteForm`) are how we trigger form actions programmatically. The `TodoItem` callbacks set the hidden inputs and submit. Pure HTML semantics — no JSON wrangling here. Project 8 will switch to remote functions for this, but the form pattern still works.
- `aria-live="polite"` on the counter — screen readers announce the change without interrupting current speech.
- `{#each todos as todo (todo.id)}` — the **keyed each** block. The `(todo.id)` is the key. Svelte uses it to track identity across updates so reorder/insert is efficient (and animations don't break).

> **`bind:this` vs attachments (`@attach`)** — Svelte 5 added attachments as a cleaner alternative to `bind:this` + manual DOM work. We'll use attachments in project 14 when we wire up GSAP. For project 1, `bind:this` is the simpler primitive and the right teaching tool.

## B.15. Type-check

```bash
pnpm check
# → 0 errors, 0 warnings
```

If you see errors, **don't move on**. Errors mean the contract is broken somewhere — backend types don't match frontend types, or you mistyped a prop. Read the message, fix the source.

## B.16. Run it

In one terminal:

```bash
cd backend
DATABASE_URL=sqlite:./todo.db cargo run
```

In another:

```bash
cd frontend
pnpm dev --open
```

Click around. Add a task. Check it off. Delete it. Resize the window — at 768px the layout breathes more. At 390px (mobile) it stacks.

---

# Part C — End-to-end testing

## C.1. Playwright config

Create file `projects/01-todo/frontend/playwright.config.ts`:

```ts
import { defineConfig, devices } from '@playwright/test';

const PORT = 4173;

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  reporter: 'list',
  use: {
    baseURL: `http://localhost:${PORT}`,
    trace: 'on-first-retry'
  },
  webServer: {
    command: `pnpm build && pnpm preview --port ${PORT}`,
    port: PORT,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000
  },
  projects: [
    { name: 'mobile-portrait-390', use: { ...devices['Desktop Chrome'], viewport: { width: 390, height: 844 }, hasTouch: true } },
    { name: 'tablet-768', use: { ...devices['Desktop Chrome'], viewport: { width: 768, height: 1024 } } },
    { name: 'laptop-1024', use: { ...devices['Desktop Chrome'], viewport: { width: 1024, height: 768 } } },
    { name: 'desktop-1440', use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } } }
  ]
});
```

### Line-by-line

- `testDir: './e2e'` — separate from `src/` so vitest and svelte-check ignore them.
- `fullyParallel: true` — run tests concurrently across viewports.
- `webServer.command` — Playwright will start this before tests run and kill it after.
- `webServer.port` — wait for this port to be reachable.
- `reuseExistingServer: !process.env.CI` — in dev, reuse a running preview. In CI, always start fresh.
- `projects: [...]` — **same tests, four viewports.** This is how we prove responsive design works. Each project is a named browser config; `pnpm test:e2e` runs the suite once per project.
- `hasTouch: true` on mobile — emulates touch events. Catches "this only works with mouse" bugs.

## C.2. The end-to-end test

Create file `projects/01-todo/frontend/e2e/todos.spec.ts`:

```ts
import { test, expect } from '@playwright/test';

test('full task lifecycle: add, toggle done, delete', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();

  const input = page.getByLabel('New task');
  await input.fill('write tests');
  await page.getByRole('button', { name: /add/i }).click();

  const item = page.getByRole('listitem').filter({ hasText: 'write tests' });
  await expect(item).toBeVisible();

  await expect(page.locator('p.meta')).toContainText('1 of 1 remaining');

  const checkbox = item.getByRole('button', { name: /mark "write tests" as done/i });
  await checkbox.click();

  await expect(page.locator('p.meta')).toContainText('0 of 1 remaining');

  const removeBtn = item.getByRole('button', { name: /delete "write tests"/i });
  await removeBtn.click();

  await expect(item).toHaveCount(0);
  await expect(page.getByText('No tasks yet')).toBeVisible();
});

test('validation: empty title is blocked client-side', async ({ page }) => {
  await page.goto('/');
  const addBtn = page.getByRole('button', { name: /add/i });
  await expect(addBtn).toBeDisabled();
});
```

### Line-by-line — accessible-first queries

- `page.getByRole('heading', { name: 'Today' })` — query by ARIA role and accessible name. **Why this style?** Because if a test can't find an element by its role, real users with screen readers can't find it either. Tests double as accessibility audits.
- `page.getByLabel('New task')` — finds the input by its `aria-label`. Same principle.
- `page.getByRole('button', { name: /add/i })` — case-insensitive regex match on the visible text + icon's accessible name.
- `expect(item).toBeVisible()` — Playwright auto-retries this assertion until the element appears or the timeout elapses. No manual `waitForSelector` needed.
- `expect(item).toHaveCount(0)` — assert the element is gone.
- The second test asserts an interaction *isn't possible*: the Add button is disabled when the input is empty. Just as important as positive cases.

## C.3. Run them

The backend must be running on port 3000 (Playwright's webServer brings up the frontend itself):

```bash
# Terminal 1
cd backend
rm -f todo.db && sqlite3 todo.db < migrations/0001_init.sql
DATABASE_URL=sqlite:./todo.db PORT=3000 FRONTEND_ORIGIN=http://localhost:4173 cargo run

# Terminal 2
cd frontend
pnpm test:e2e
# → 8 passed (4 viewports × 2 tests)
```

The same flow proves out at 390 px (mobile), 768 (tablet), 1024 (laptop), and 1440 (desktop). If any breakpoint breaks, you get a focused failure and an HTML report at `playwright-report/`.

---

# Part D — Closing thoughts

## What you learned

- **Rust + Axum** for HTTP. State, extractors, error mapping, graceful shutdown.
- **sqlx with compile-time-checked SQL** — typos in SQL break the build.
- **SQLite operational basics** — WAL, foreign keys, busy timeout.
- **Svelte 5 runes** — `$state`, `$derived`, `$props`.
- **SvelteKit `load` + form actions** — progressive enhancement built-in.
- **Plain CSS** with cascade layers, design tokens, mobile-first.
- **Testing pyramid** — Rust unit, TS unit (Vitest), browser E2E (Playwright × 4 viewports).
- **The discipline** of clean code + separate teaching docs.

## What's next

**Project 02 — Markdown Notes** introduces:

- Proper SEO (full `<svelte:head>` + JSON-LD + sitemap + robots).
- `<svelte:boundary>` for per-route error containment.
- Server-side HTML sanitization (`ammonia` crate).
- Page options — prerendered marketing pages.
- Your first Svelte transition (`fade` from `svelte/transition`).

Same template. Same teaching format. Same standard of "every line explained".

## A note on the LESSON viewer (Monaco)

This repo also ships (later in the curriculum) a small **Lesson Viewer** web app that loads each project's `LESSON.md`, renders it side-by-side with a **Monaco editor** so you can edit and run the code samples in-browser. Build that as a meta-project — perfect graduation exercise after you've absorbed projects 1–5. The Markdown format we use here (file paths in headings, ` ```ts ` / ` ```rust ` / ` ```svelte ` fenced blocks, "line-by-line" sections) is machine-parseable, so the viewer can split each chunk into "create file X" → editor → narration panel automatically.

Until then: read top-to-bottom in any Markdown viewer (GitHub renders it perfectly).
