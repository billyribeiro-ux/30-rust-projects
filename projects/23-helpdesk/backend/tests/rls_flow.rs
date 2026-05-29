//! End-to-end RLS proofs over HTTP.
//!
//! Covers:
//!   - cross-tenant isolation (Bob's tenant cannot see Alice's tickets)
//!   - role enforcement (customer cannot post `internal: true`)
//!   - the audit trigger writes a row on ticket insert

use helpdesk_backend::AppState;
use helpdesk_backend::build_app;
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use uuid::Uuid;

async fn maybe_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn fresh() -> Option<(SocketAddr, PgPool)> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    sqlx::query!(
        "TRUNCATE audit_log, ticket_messages, tickets, magic_links, invitations, memberships, sessions, users, tenants CASCADE"
    )
    .execute(&pool)
    .await
    .ok()?;
    let state = AppState {
        pool: pool.clone(),
        public_url: "http://localhost:5195".into(),
        smtp_url: "smtp://localhost:1027".into(),
        smtp_from: "noreply@helpdesk.local".into(),
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5195");
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .ok()?;
    let addr = listener.local_addr().ok()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    Some((addr, pool))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap()
}

async fn register(base: &str, c: &reqwest::Client) -> Uuid {
    let email = format!("u-{}@example.com", Uuid::new_v4());
    let r = c
        .post(format!("{base}/api/auth/register"))
        .json(&serde_json::json!({
            "email": email, "password": "correct horse battery", "name": "U"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let body: serde_json::Value = r.json().await.unwrap();
    body["id"].as_str().unwrap().parse().unwrap()
}

#[tokio::test]
async fn cross_tenant_isolation_via_rls() {
    let Some((addr, pool)) = fresh().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let base = format!("http://{addr}");

    // Alice creates tenant A, opens a ticket.
    let alice = client();
    let _ = register(&base, &alice).await;
    let r = alice
        .post(format!("{base}/api/tenants"))
        .json(&serde_json::json!({ "slug": "acme", "name": "Acme" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let r = alice
        .post(format!("{base}/api/t/acme/tickets"))
        .json(&serde_json::json!({ "subject": "secrets", "body": "do not share" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);

    // Bob signs up, creates his own tenant. He should NOT see Acme.
    let bob = client();
    let _ = register(&base, &bob).await;
    let r = bob
        .post(format!("{base}/api/tenants"))
        .json(&serde_json::json!({ "slug": "globex", "name": "Globex" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let r = bob
        .get(format!("{base}/api/t/acme/tickets"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404, "non-member must NOT see Acme tickets");

    // Audit row exists from the ticket insert.
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM audit_log WHERE entity_kind = 'ticket' AND action = 'INSERT'"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert!(count >= 1, "trigger should have written an audit row");
}

#[tokio::test]
async fn customer_cannot_post_internal_message() {
    let Some((addr, _pool)) = fresh().await else {
        return;
    };
    let base = format!("http://{addr}");

    // Admin creates tenant + a customer membership.
    let admin = client();
    let admin_id = register(&base, &admin).await;
    admin
        .post(format!("{base}/api/tenants"))
        .json(&serde_json::json!({ "slug": "ten", "name": "Ten" }))
        .send()
        .await
        .unwrap();
    let _ = admin_id;

    // Customer signs up via magic-link verify simulation: we directly add
    // them as a member via the admin path. (In real life they'd magic-link.)
    // Here we test the inverse: customer can't post internal even when
    // they ARE a member.
    let customer = client();
    let cust_id = register(&base, &customer).await;
    let tenant_id: Uuid = sqlx::query_scalar!("SELECT id FROM tenants WHERE slug = $1", "ten")
        .fetch_one(&_pool)
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO memberships (tenant_id, user_id, role) VALUES ($1, $2, 'customer')",
        tenant_id,
        cust_id
    )
    .execute(&_pool)
    .await
    .unwrap();

    // Customer creates a ticket.
    let r = customer
        .post(format!("{base}/api/t/ten/tickets"))
        .json(&serde_json::json!({ "subject": "help", "body": "stuck" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let body: serde_json::Value = r.json().await.unwrap();
    let ticket_id = body["id"].as_str().unwrap();

    // Customer attempts to post internal → 403.
    let r = customer
        .post(format!("{base}/api/t/ten/tickets/{ticket_id}/messages"))
        .json(&serde_json::json!({ "body": "internal note", "internal": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);
}
