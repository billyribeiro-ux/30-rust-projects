//! End-to-end RBAC + board-flow integration tests.
//!
//! These drive the *real* Axum app (via `kanban_backend::build_app`) over HTTP
//! against a real Postgres, so the routing, the `AuthUser` cookie extractor, and
//! the `require_role` gate are all exercised exactly as they ship. The suite
//! skips gracefully when `DATABASE_URL` is unset so `cargo test` stays green on
//! a machine without a database (the CI job sets it; see COMMANDS.md).
//!
//! Coverage:
//!   * a fresh user can register, create a board, and is its implicit admin;
//!   * a viewer is blocked (403) from mutating, an editor is allowed (2xx);
//!   * a non-member gets 404 (we never leak board existence);
//!   * the card move path persists `list_id` + `position` (the contract the
//!     frontend's optimistic drag-and-drop relies on);
//!   * an editor cannot delete another member's comment, an admin can.

use std::net::SocketAddr;

use kanban_backend::build_app;
use kanban_backend::state::AppState;
use tokio::net::TcpListener;
use uuid::Uuid;

async fn maybe_pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    sqlx::PgPool::connect(&url).await.ok()
}

/// Bind an ephemeral port, serve the real app, return its address.
async fn spawn_app() -> Option<SocketAddr> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;

    let state = AppState {
        pool,
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5191");

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .ok()?;
    let addr = listener.local_addr().ok()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    Some(addr)
}

/// A reqwest client with its own cookie jar — one per logical user, so sessions
/// don't bleed across actors in a multi-role test.
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("client builds")
}

/// Register a brand-new user and return their authenticated client + user id.
async fn register(base: &str) -> (reqwest::Client, Uuid) {
    let c = client();
    let email = format!("u-{}@example.com", Uuid::new_v4());
    let res = c
        .post(format!("{base}/api/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "correct horse battery staple",
            "name": "Test User",
        }))
        .send()
        .await
        .expect("register sends");
    assert_eq!(res.status(), 201, "register should create the account");
    let body: serde_json::Value = res.json().await.expect("register returns json");
    let id = body["id"].as_str().expect("id present").parse().unwrap();
    (c, id)
}

#[tokio::test]
async fn owner_is_implicit_admin_and_rbac_is_enforced() {
    let Some(addr) = spawn_app().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let base = format!("http://{addr}");

    // Owner registers and creates a board.
    let (owner, _owner_id) = register(&base).await;
    let res = owner
        .post(format!("{base}/api/boards"))
        .json(&serde_json::json!({ "name": "Roadmap" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let board: serde_json::Value = res.json().await.unwrap();
    let board_id = board["id"].as_str().unwrap().to_string();
    let slug = board["slug"].as_str().unwrap().to_string();
    assert_eq!(board["role"], "admin", "creator is implicit admin");

    // A second user with no membership must get 404 (existence not leaked),
    // not 403.
    let (stranger, _) = register(&base).await;
    let res = stranger
        .get(format!("{base}/api/boards/{slug}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404, "non-members can't see the board exists");

    // Owner adds the stranger as a *viewer*.
    let viewer_email = stranger_email(&stranger, &base).await;
    let res = owner
        .post(format!("{base}/api/boards/{board_id}/memberships"))
        .json(&serde_json::json!({ "email": viewer_email, "role": "viewer" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    // Owner creates a list so the viewer has something to (not) edit.
    let res = owner
        .post(format!("{base}/api/boards/{board_id}/lists"))
        .json(&serde_json::json!({ "name": "Todo" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let list: serde_json::Value = res.json().await.unwrap();
    let list_id = list["id"].as_str().unwrap().to_string();

    // Viewer can READ the board now…
    let res = stranger
        .get(format!("{base}/api/boards/{slug}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "viewer can read");

    // …but CANNOT create a card (editor+ only) → 403.
    let res = stranger
        .post(format!("{base}/api/lists/{list_id}/cards"))
        .json(&serde_json::json!({ "title": "sneaky" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403, "viewer is forbidden from mutating");

    // Owner promotes them to editor; now the same call succeeds.
    let res = owner
        .post(format!("{base}/api/boards/{board_id}/memberships"))
        .json(&serde_json::json!({ "email": viewer_email, "role": "editor" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let res = stranger
        .post(format!("{base}/api/lists/{list_id}/cards"))
        .json(&serde_json::json!({ "title": "now allowed" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201, "editor may create cards");
}

#[tokio::test]
async fn card_move_persists_list_and_position() {
    let Some(addr) = spawn_app().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let base = format!("http://{addr}");

    let (owner, _) = register(&base).await;
    let board_id = create_board(&owner, &base, "Move Test").await;

    let list_a = create_list(&owner, &base, &board_id, "A").await;
    let list_b = create_list(&owner, &base, &board_id, "B").await;

    // Card starts in list A.
    let res = owner
        .post(format!("{base}/api/lists/{list_a}/cards"))
        .json(&serde_json::json!({ "title": "Card 1" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let card: serde_json::Value = res.json().await.unwrap();
    let card_id = card["id"].as_str().unwrap().to_string();

    // Move it to list B at a fresh position — this is the optimistic-DnD path.
    let res = owner
        .patch(format!("{base}/api/cards/{card_id}"))
        .json(&serde_json::json!({ "list_id": list_b, "position": 2048.0 }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let moved: serde_json::Value = res.json().await.unwrap();
    assert_eq!(moved["list_id"], list_b, "card moved lists");
    assert_eq!(moved["position"], 2048.0, "position persisted");
}

#[tokio::test]
async fn comment_deletion_respects_authorship() {
    let Some(addr) = spawn_app().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let base = format!("http://{addr}");

    let (owner, _) = register(&base).await;
    let board_id = create_board(&owner, &base, "Comments").await;
    let list_id = create_list(&owner, &base, &board_id, "List").await;
    let res = owner
        .post(format!("{base}/api/lists/{list_id}/cards"))
        .json(&serde_json::json!({ "title": "Discuss me" }))
        .send()
        .await
        .unwrap();
    let card: serde_json::Value = res.json().await.unwrap();
    let card_id = card["id"].as_str().unwrap().to_string();

    // Add an editor who will author a comment.
    let (editor, _) = register(&base).await;
    let editor_email = stranger_email(&editor, &base).await;
    owner
        .post(format!("{base}/api/boards/{board_id}/memberships"))
        .json(&serde_json::json!({ "email": editor_email, "role": "editor" }))
        .send()
        .await
        .unwrap();

    let res = editor
        .post(format!("{base}/api/cards/{card_id}/comments"))
        .json(&serde_json::json!({ "body": "first!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let comment: serde_json::Value = res.json().await.unwrap();
    let comment_id = comment["id"].as_str().unwrap().to_string();

    // A *different* editor cannot delete it.
    let (other, _) = register(&base).await;
    let other_email = stranger_email(&other, &base).await;
    owner
        .post(format!("{base}/api/boards/{board_id}/memberships"))
        .json(&serde_json::json!({ "email": other_email, "role": "editor" }))
        .send()
        .await
        .unwrap();
    let res = other
        .delete(format!("{base}/api/cards/{card_id}/comments/{comment_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403, "non-author editor can't delete");

    // The board admin (owner) can.
    let res = owner
        .delete(format!("{base}/api/cards/{card_id}/comments/{comment_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "admin can delete any comment");
}

// ---------- helpers ----------

async fn stranger_email(c: &reqwest::Client, base: &str) -> String {
    let res = c.get(format!("{base}/api/auth/me")).send().await.unwrap();
    let body: serde_json::Value = res.json().await.unwrap();
    body["email"].as_str().unwrap().to_string()
}

async fn create_board(c: &reqwest::Client, base: &str, name: &str) -> String {
    let res = c
        .post(format!("{base}/api/boards"))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let b: serde_json::Value = res.json().await.unwrap();
    b["id"].as_str().unwrap().to_string()
}

async fn create_list(c: &reqwest::Client, base: &str, board_id: &str, name: &str) -> String {
    let res = c
        .post(format!("{base}/api/boards/{board_id}/lists"))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);
    let l: serde_json::Value = res.json().await.unwrap();
    l["id"].as_str().unwrap().to_string()
}
