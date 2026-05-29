//! Ingest + KPI snapshot proofs.

use analytics_backend::AppState;
use analytics_backend::build_app;
use analytics_backend::routes::kpis;
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

async fn maybe_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn fresh() -> Option<(SocketAddr, PgPool)> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    sqlx::query!("TRUNCATE events, sessions, users CASCADE")
        .execute(&pool)
        .await
        .ok()?;
    let (kpis_tx, _) = broadcast::channel(64);
    let state = AppState {
        pool: pool.clone(),
        kpis_tx,
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5199");
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .ok()?;
    let addr = listener.local_addr().ok()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    Some((addr, pool))
}

#[tokio::test]
async fn ingest_then_snapshot_reflects_counts() {
    let Some((addr, pool)) = fresh().await else {
        eprintln!("skip: no DATABASE_URL");
        return;
    };
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // Send 7 events; 5 of kind 'click', 2 of kind 'view'.
    for _ in 0..5 {
        let r = client
            .post(format!("{base}/v1/ingest"))
            .json(&serde_json::json!({"kind":"click","session_id":"s1"}))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 204);
    }
    for _ in 0..2 {
        client
            .post(format!("{base}/v1/ingest"))
            .json(&serde_json::json!({"kind":"view","session_id":"s2"}))
            .send()
            .await
            .unwrap();
    }

    let snap = kpis::compute_snapshot(&pool).await.unwrap();
    assert_eq!(snap.events_last_hour, 7);
    assert!(snap.events_last_minute >= 7);
    assert_eq!(snap.unique_sessions_last_hour, 2);
    assert_eq!(
        snap.top_kinds.first().map(|t| t.kind.as_str()),
        Some("click")
    );
    assert_eq!(snap.top_kinds.first().map(|t| t.count), Some(5));
}

#[tokio::test]
async fn ingest_rejects_empty_kind() {
    let Some((addr, _)) = fresh().await else {
        return;
    };
    let base = format!("http://{addr}");
    let r = reqwest::Client::new()
        .post(format!("{base}/v1/ingest"))
        .json(&serde_json::json!({"kind":""}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
}
