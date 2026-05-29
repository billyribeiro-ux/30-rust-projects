//! End-to-end: signup, create an API key, call /v1/inference, observe
//! usage_events row, verify revoked key + RPM gating.

use ai_api_backend::AppState;
use ai_api_backend::{build_app, build_webauthn};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn maybe_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn fresh() -> Option<(SocketAddr, PgPool)> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    sqlx::query!(
        "TRUNCATE usage_events, api_keys, webauthn_credentials, webauthn_states, sessions, users CASCADE"
    )
    .execute(&pool)
    .await
    .ok()?;
    let webauthn = build_webauthn("localhost", "AI API", "http://localhost:5200");
    let state = AppState {
        pool: pool.clone(),
        webauthn,
        free_tier_calls: 10_000,
        per_call_cents: 0.1,
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5200");
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .ok()?;
    let addr = listener.local_addr().ok()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    Some((addr, pool))
}

fn cookie_client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap()
}

#[tokio::test]
async fn create_key_and_call_inference_records_usage() {
    let Some((addr, pool)) = fresh().await else {
        eprintln!("skip: no DATABASE_URL");
        return;
    };
    let base = format!("http://{addr}");
    let c = cookie_client();

    // Register a user via password auth.
    let r = c
        .post(format!("{base}/api/auth/register"))
        .json(&serde_json::json!({
            "email": "dev@example.com",
            "password": "correct horse battery",
            "name": "Dev"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);

    // Create an API key.
    let r = c
        .post(format!("{base}/api/keys"))
        .json(&serde_json::json!({ "name": "ci", "rpm_limit": 5 }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let body: serde_json::Value = r.json().await.unwrap();
    let secret = body["secret"].as_str().unwrap().to_string();
    assert!(secret.starts_with("sk_live_"));

    // Use it.
    let r = reqwest::Client::new()
        .post(format!("{base}/v1/inference"))
        .bearer_auth(&secret)
        .json(&serde_json::json!({ "prompt": "hello" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    let usage: i64 = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "n!" FROM usage_events"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(usage, 1);

    // Revoke and prove it bounces.
    let id = body["id"].as_str().unwrap();
    let r = c
        .post(format!("{base}/api/keys/{id}/revoke"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    let r = reqwest::Client::new()
        .post(format!("{base}/v1/inference"))
        .bearer_auth(&secret)
        .json(&serde_json::json!({ "prompt": "hi" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
}

#[tokio::test]
async fn rpm_limit_returns_403_after_threshold() {
    let Some((addr, _)) = fresh().await else {
        return;
    };
    let base = format!("http://{addr}");
    let c = cookie_client();
    c.post(format!("{base}/api/auth/register"))
        .json(&serde_json::json!({
            "email": "rl@example.com",
            "password": "correct horse battery",
            "name": "RL"
        }))
        .send()
        .await
        .unwrap();
    let r = c
        .post(format!("{base}/api/keys"))
        .json(&serde_json::json!({ "name": "rl", "rpm_limit": 3 }))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = r.json().await.unwrap();
    let secret = body["secret"].as_str().unwrap().to_string();
    let bare = reqwest::Client::new();
    for _ in 0..3 {
        let r = bare
            .post(format!("{base}/v1/inference"))
            .bearer_auth(&secret)
            .json(&serde_json::json!({ "prompt": "x" }))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200);
    }
    // 4th call → 403.
    let r = bare
        .post(format!("{base}/v1/inference"))
        .bearer_auth(&secret)
        .json(&serde_json::json!({ "prompt": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);
}
