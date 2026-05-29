# Lesson — Project 19 (Kanban, GSAP #1)

Three things this project teaches that no earlier project did:

1. **Optimistic mutations with rollback** — the UI mutates before the
   network ACK, and undoes itself if the server says no.
2. **GSAP, lazy-loaded and reduced-motion-respecting** — cinematic
   *polish* without blocking SSR or punishing users who said "less
   motion, please."
3. **RBAC enforced at an Axum extractor**, not at the UI. The viewer
   can render a board they cannot mutate; the server is the source of
   truth.

The teaching is structured backend → frontend → tests → what's next.

---

## A. Backend

### A1. The router (Axum 0.8 path-param naming)

Axum 0.8 changed path syntax from `:slug` to `{slug}` and added a
strict rule: **at the same position, all routes must use the same
param name.** The earlier draft of this project had:

```rust
.route("/{slug}", get(read_by_slug))
.route("/{id}", axum::routing::delete(remove))
```

…which compiles but **panics at router construction** with:

```
Invalid route "/{id}": Insertion failed due to conflict with previously
registered route: /{slug}
```

The fix is to pick one name. We chose `{id}` and overloaded the handlers
— `GET` interprets the segment as a slug (the public addressing scheme
the frontend uses), `DELETE` interprets it as a UUID:

```rust
.route("/{id}", get(read_by_slug).delete(remove))
```

The handlers parse the segment in the shape they need (`String` for
the slug-based read, `Uuid` for delete). This is the same pattern
Stripe takes with its "ID or session_id" routes.

### A2. `require_role` — the RBAC gate

```rust
// src/auth/rbac.rs
pub async fn require_role(
    pool: &PgPool,
    board_id: Uuid,
    user_id: Uuid,
    minimum: Role,
) -> Result<Role, AppError> {
    let row = sqlx::query!(
        r#"SELECT role FROM memberships WHERE board_id = $1 AND user_id = $2"#,
        board_id, user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?; // not a 403: don't leak existence.

    let role: Role = row.role.parse()?;
    if role.rank() < minimum.rank() {
        return Err(AppError::Forbidden);
    }
    Ok(role)
}
```

Two design choices worth calling out:

- A non-member sees **404, not 403**. Returning 403 leaks "this board
  exists, you just can't access it." The frontend already collapses
  both to "Board not found", so the UX is identical and the security
  is better. (See `tests/rbac_flow.rs`'s "non-members can't see the
  board exists".)
- Role comparison is a `rank()` integer (`viewer=1, editor=2, admin=3`),
  not a `match`. This keeps `require_role(.., Role::Editor)` readable
  and lets us add roles later without touching every call site.

### A3. Lexorank-ish positions

Cards and lists carry a `DOUBLE PRECISION position`. Inserting between
two neighbours is the midpoint:

```ts
const newPos = ((prev?.position ?? 0) + (next?.position ?? 0)) / 2;
```

In the limit (~50 sequential midpoints) you exhaust f64 precision and
need a rebalance pass. We don't ship one — it's a known follow-up.
Real lexorank (string-based, infinite resolution) is a future project.

### A4. The card-move contract the frontend depends on

`PATCH /api/cards/:id` accepts an optional `list_id` *and* `position`.
On move, the handler verifies the target list belongs to the same board
(crossing boards would be an authorization violation), then updates in
a single statement:

```rust
sqlx::query!(
    "UPDATE cards SET list_id = $2, position = $3, updated_at = now()
     WHERE id = $1",
    id, target_list, target_pos,
).execute(&s.pool).await?;
```

The optimistic UI relies on **both fields being persisted atomically**.
If the move succeeds partially (e.g., position updated but list_id
didn't), the rollback can't reach a clean state. That's why it's one
UPDATE, not two.

### A5. The integration tests live in `tests/rbac_flow.rs`

They spawn the *real* `build_app` against a real Postgres, talk to it
over HTTP with three reqwest clients (one cookie jar per actor), and
exercise the full permission matrix. When a teammate wants to know
"can a viewer move a card?", the answer is in test code, not prose.

---

## B. Frontend

### B1. `KanbanBoard.svelte` — optimistic move-with-rollback

The heart of this project:

```svelte
async function onDrop(e: DragEvent, targetListId: string, beforeIdx: number) {
  // 1. Pure-local position math: midpoint between neighbours.
  const newPos = midpoint(targetListId, beforeIdx);
  const prevState = { list_id: card.list_id, position: card.position };

  // 2. Optimistic mutation (state assignment, no await).
  card.list_id = targetListId;
  card.position = newPos;
  cards = [...cards];

  // 3. GSAP cinematic settle (lazy, reduced-motion safe).
  if (!prefersReducedMotion()) {
    const el = document.querySelector<HTMLElement>(`[data-card-id="${cardId}"]`);
    if (el) animateDrop(el);
  }

  // 4. Fire-and-rollback PATCH.
  try {
    await cardsApi.update(fetch, cardId, { list_id: targetListId, position: newPos });
  } catch (err) {
    card.list_id = prevState.list_id;
    card.position = prevState.position;
    cards = [...cards];
    error = `Move failed: ${err.message}`;
    setTimeout(() => (error = null), 4000);
  }
}
```

What's load-bearing:

- **`prevState` captured *before* the optimistic write.** If the server
  rejects, we know the exact pre-drop coordinates.
- **`cards = [...cards]`** to trigger the `$derived.by(...)` that
  rebuilds `cardsByList`. We can't mutate `card.list_id` in place and
  rely on the derived to refresh — fine-grained reactivity sees the
  field change, but `cardsByList` needs the array identity to change
  to re-run.
- **The `setTimeout` clears the banner** so a transient failure doesn't
  haunt the UI forever.

### B2. The `state_referenced_locally` silence

The Svelte autofixer flags:

```ts
let lists = $state<ListRow[]>(initial.lists.map(l => ({ ...l })));
```

as "this only captures the initial value of `initial`; did you mean
to reference it inside a $derived?" That warning exists for the
common bug where you wanted `lists` to *track* the prop. We don't.
The page is bound to a single board instance for its lifetime, and
re-seeding on prop change would discard in-flight optimistic
mutations — exactly what we don't want.

The fix is a documented `// svelte-ignore state_referenced_locally`.
The pattern is intentional. The comment is the proof.

### B3. GSAP, the lazy way

```ts
// src/lib/motion.ts
let gsapPromise: Promise<typeof import('gsap')> | null = null;
function loadGsap() {
  if (!gsapPromise) gsapPromise = import('gsap');
  return gsapPromise;
}

export async function animateDrop(el: HTMLElement): Promise<void> {
  if (prefersReducedMotion()) return;
  const { gsap } = await loadGsap();
  gsap.killTweensOf(el);
  gsap.timeline().fromTo(el,
    { scale: 1.04, y: -6, boxShadow: '0 12px 28px rgb(0 0 0 / 0.18)' },
    { scale: 1, y: 0, boxShadow: '0 1px 2px rgb(0 0 0 / 0.06)',
      duration: 0.28, ease: 'power3.out' });
}
```

Why this shape:

- **Dynamic `import('gsap')` keeps GSAP out of the main bundle and out
  of SSR.** First paint is unaffected; the chunk loads when the user
  drops their first card.
- **`prefersReducedMotion()` is checked twice** — once in the caller
  (to avoid even querying the DOM) and once here (defensive: a future
  caller might forget). Cheap.
- **`killTweensOf(el)`** before starting protects against rapid
  successive drops on the same card — without it, animations stack
  and produce a jittery card.

### B4. SvelteMap vs native Map (the suggestion we *didn't* take)

`$derived.by` rebuilds `cardsByList` from scratch each time `lists`
or `cards` changes:

```ts
let cardsByList = $derived.by(() => {
  const m = new Map<string, CardRow[]>();
  ...
  return m;
});
```

The autofixer suggests `SvelteMap`, which is reactive in-place. We
don't need that — we create a fresh Map every time the derived runs,
so reactivity comes from the assignment, not from mutating an existing
collection. Native `Map` is a tiny bit faster and one fewer import.

---

## C. Tests

### C1. Backend — `tests/rbac_flow.rs`

Three integration tests drive the real Axum app over HTTP against
real Postgres:

1. **`owner_is_implicit_admin_and_rbac_is_enforced`** — covers the
   four headline RBAC states: owner is admin; non-member is 404;
   viewer reads but can't write; promoted-to-editor can.
2. **`card_move_persists_list_and_position`** — the contract the
   optimistic UI depends on. If this fails, drag-and-drop visibly
   regresses on the frontend.
3. **`comment_deletion_respects_authorship`** — editors can delete
   their own comments; admins can delete anyone's.

The suite uses three reqwest clients with isolated cookie jars
(`reqwest::Client::builder().cookie_store(true).build()`) so a
viewer's session can't bleed into the owner's. Without isolation
you can't reliably test multi-actor RBAC over the same TCP socket.

### C2. Frontend — `api.test.ts`

Five small Vitest specs cover the API client surface: list parsing,
POST body shape, PATCH URL encoding, nested-route correctness, and
non-2xx → `ApiCallError` mapping. The strict-mode mock-cast dance
(`(fetcher.mock.calls as unknown as Array<[string, RequestInit]>)`)
is reused from project 06 — see `PATTERNS.md`.

### C3. E2E — `kanban.spec.ts`

Five specs × four viewports = 20 runs:

- Home redirects to /login when unauthenticated.
- /login is axe-clean (wcag2a + wcag2aa).
- Signup → /boards is axe-clean.
- Create board + open: card is rendered, has `draggable="true"` for
  editors.
- Logout returns to /login.

Drag-and-drop is *not* exercised end-to-end — HTML5 DnD in headless
Playwright is flaky enough to be a maintenance liability. The drag
behaviour is covered by (a) the unit test that proves the API
contract, (b) the integration test that proves the server persists,
and (c) the e2e check that the UI exposes the right ARIA + draggable
attributes.

---

## D. What you can do now

1. Ship a tactile board UI without sacrificing accessibility.
2. Enforce RBAC at the route extractor — the only place an honest
   security audit accepts.
3. Reason about optimistic-mutate-with-rollback as a state machine,
   not as "hope nothing fails."
4. Use GSAP without inflating the bundle or punishing reduced-motion
   users.

Project 20 — OAuth 2.0 + PKCE and PostGIS for geo-aware search.
