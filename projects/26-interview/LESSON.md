# Lesson — Project 26 (Live Coding Interview Platform)

Three things this project teaches that no earlier project did:

1. **Axum WebSocket + `tokio::sync::broadcast`** — the simplest correct
   "broadcast every message to every subscriber" topology.
2. **The `CodeRunner` trait** as the seam between "test in CI without
   Docker" and "production with a sandboxed container."
3. **SAML SP metadata** — the XML enterprise IdPs need to wire your
   product into their SSO.

---

## A. Backend

### A1. The hub allocator

```rust
let tx = {
    let mut hubs = s.hubs.lock().await;
    hubs.entry(id)
        .or_insert_with(|| broadcast::channel::<String>(256).0)
        .clone()
};
```

One broadcaster per interview id. Created lazily on first connection,
removed when the last subscriber disconnects:

```rust
let mut hubs = s.hubs.lock().await;
if let Some(t) = hubs.get(&id) && t.receiver_count() == 0 {
    hubs.remove(&id);
}
```

**Why a `Mutex<HashMap>` and not a `DashMap`?** Hubs are short-lived,
contention is per-interview (rarely more than 2-3 participants), and
the lookup is dominated by HashMap, not lock acquisition. A DashMap
would shave a few nanoseconds and complicate the cleanup path.

### A2. Why this is **not** a CRDT (and what to upgrade to)

Two participants typing simultaneously will:

1. Send their own diff via `kind:code`.
2. Receive each other's diff via the broadcaster.
3. *Overwrite* their local state with whichever arrived last.

Last-write-wins is *correct* for a single typist with multiple
watchers (the common interview case — interviewer watches candidate)
and *broken* for genuine concurrent editing.

The upgrade is **`yrs`**, the Rust port of Yjs:

```rust
let doc = yrs::Doc::new();
let text = doc.get_or_insert_text("code");
// Each client sends Y.js binary updates; server applies + rebroadcasts.
```

…which gives you operational-transform-equivalent semantics without
writing a single line of OT. Documented in COMMANDS.md as the
production path. Tests against `yrs` would exercise the same WebSocket
endpoint with binary messages — the surface doesn't change.

### A3. The `CodeRunner` trait

```rust
#[async_trait]
pub trait CodeRunner: Send + Sync {
    async fn run(&self, language: &str, code: &str)
        -> AppResult<(String, String, i32, i32)>;
}

pub struct MockRunner;

#[async_trait]
impl CodeRunner for MockRunner {
    async fn run(&self, language: &str, code: &str)
        -> AppResult<(String, String, i32, i32)>
    { … }
}
```

Why a trait and not a feature flag:

- **Tests inject `MockRunner`** with no docker dependency. The
  integration test suite runs in CI without Docker installed.
- **Production injects `DockerRunner`** behind the same trait. Same
  routes, same handlers, different runner.
- **Adding a `GVisorRunner` later** doesn't touch any route code.

The mock returns `[mock-<lang>] N chars run` to stdout. The route
records the execution to `executions` regardless of which runner ran.
Future audits see "we executed something" without trusting the runner
itself.

### A4. The Docker hardening checklist (for the production runner)

```bash
docker run --rm \
  --network=none \
  --memory=256m \
  --cpus=1 \
  --read-only \
  --tmpfs /tmp \
  --user 65534:65534 \
  --security-opt no-new-privileges \
  --cap-drop=ALL \
  <image>:<tag> \
  /usr/bin/timeout 10 <runtime> <code-file>
```

Every flag is doing work:
- **`--network=none`** — code cannot exfiltrate.
- **`--memory=256m --cpus=1`** — bounded resource use; fork bombs die.
- **`--read-only` + `--tmpfs /tmp`** — code cannot write anywhere but
  `/tmp`, which is wiped on container exit.
- **`--user 65534:65534`** (`nobody`) + `no-new-privileges` + `--cap-drop=ALL`
  — code cannot escalate.
- **`timeout 10`** — even if the runtime ignores signals, the wall
  clock cuts the process.

This is the floor. Add gVisor or Firecracker on top for defence-in-depth.

---

## B. SAML

### B1. The metadata XML

```xml
<md:EntityDescriptor entityID="https://your-app/saml/metadata">
  <md:SPSSODescriptor AuthnRequestsSigned="false"
                      WantAssertionsSigned="true"
                      protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat>
    <md:AssertionConsumerService
        Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
        Location="https://your-app/saml/acs"
        index="0" isDefault="true"/>
  </md:SPSSODescriptor>
</md:EntityDescriptor>
```

The three fields the IdP actually reads:

- **`entityID`** — uniquely identifies the SP to the IdP. We use our
  metadata URL itself; some IdPs require a separate URN.
- **`AssertionConsumerService Location`** — where the IdP POSTs the
  `SAMLResponse`. Must be HTTPS in production.
- **`NameIDFormat`** — email is the standard for SaaS. The IdP can
  override per app.

### B2. The ACS endpoint (the work we left for future)

```rust
async fn acs(State(_s): State<AppState>) -> impl IntoResponse {
    // TODO: SAMLResponse verification via `samael`.
    StatusCode::NOT_IMPLEMENTED
}
```

What it should do:

1. Read the form-posted `SAMLResponse` (base64 XML).
2. Verify the IdP's signature against the configured X.509 cert.
3. Validate `NotBefore`/`NotOnOrAfter` and the destination URL.
4. Extract NameID (email) and attributes.
5. Look up the user; create one if first time (JIT provisioning).
6. Create a session, set the cookie, redirect to `/`.

The `samael` crate gives you `(1)` through `(4)` in one call. We left
the integration out so the project builds cleanly without `xmlsec` on
the host. Adding it is:

```toml
samael = { version = "0.0.17", default-features = false, features = ["xmlsec"] }
```

Plus the implementation in `routes/saml.rs::acs`. Production deployments
must do this.

---

## C. Tests

### C1. `cargo test` (3)

- `auth::hash::tests::round_trip` (Argon2)
- `ws_flow::two_clients_see_each_others_edits` — connects A and B,
  A sends, B receives, server persists.
- `ws_flow::saml_metadata_is_valid_xml` — endpoint produces parseable
  metadata.

### C2. `pnpm test:unit` (5)

`interviewsApi.list/create`, `execApi.run` URL-encoding,
`wsUrl()` swaps http→ws, non-2xx → ApiCallError.

### C3. `pnpm test:e2e` (3 × 4 = 12)

Unauthenticated redirect, login axe-clean, SAML metadata endpoint
serves XML.

---

## D. What you can do now

1. Ship a real-time shared editor with one Axum extension.
2. Make code execution swappable for testing without rewriting routes.
3. Hand an enterprise customer the SAML metadata they need to wire SSO.

Project 27 — Realtime Analytics Dashboard with DuckDB + GSAP.
