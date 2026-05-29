//! Capstone integration: signup → create tenant → create project → cross-tenant isolation.

use capstone_backend::AppState;
use capstone_backend::build_app;
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
        "TRUNCATE audit_log, comments, tasks, projects, subscriptions, stripe_events, memberships, sessions, users, tenants CASCADE"
    )
    .execute(&pool)
    .await
    .ok()?;
    let state = AppState {
        pool: pool.clone(),
        public_url: "http://localhost:5202".into(),
        stripe_webhook_secret: "whsec_local".into(),
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5202");
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
    reqwest::Client::builder().cookie_store(true).build().unwrap()
}

async fn register(base: &str, c: &reqwest::Client) -> Uuid {
    let email = format!("u-{}@example.com", Uuid::new_v4());
    let r = c
        .post(format!("{base}/api/auth/register"))
        .json(&serde_json::json!({"email": email, "password": "correct horse battery", "name": "U"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let body: serde_json::Value = r.json().await.unwrap();
    body["id"].as_str().unwrap().parse().unwrap()
}

#[tokio::test]
async fn full_flow_signup_tenant_project_isolation() {
    let Some((addr, pool)) = fresh().await else {
        eprintln!("skip: no DATABASE_URL");
        return;
    };
    let base = format!("http://{addr}");

    // Alice creates tenant Acme and a project.
    let alice = client();
    let _alice_id = register(&base, &alice).await;
    let r = alice
        .post(format!("{base}/api/tenants"))
        .json(&serde_json::json!({"slug": "acme", "name": "Acme"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let r = alice
        .post(format!("{base}/api/t/acme/projects"))
        .json(&serde_json::json!({"slug": "web", "name": "Web"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);

    // Bob creates his own tenant. He should not see Acme's projects.
    let bob = client();
    let _bob_id = register(&base, &bob).await;
    bob.post(format!("{base}/api/tenants"))
        .json(&serde_json::json!({"slug": "globex", "name": "Globex"}))
        .send()
        .await
        .unwrap();
    let r = bob.get(format!("{base}/api/t/acme/projects")).send().await.unwrap();
    assert_eq!(r.status(), 404, "non-member must not see tenant");

    // Audit row was written for the project insert (trigger).
    let count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "n!" FROM audit_log WHERE entity_kind = 'projects' AND action = 'INSERT'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(count >= 1, "audit trigger should have fired");
}
