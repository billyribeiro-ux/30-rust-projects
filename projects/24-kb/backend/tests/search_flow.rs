//! End-to-end search tests.
//!
//! Seeds three articles, runs `/api/search?q=…`, asserts the BM25
//! ranking puts a title-match above a body-match (`setweight A > C`).

use kb_backend::AppState;
use kb_backend::build_app;
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
    sqlx::query!("TRUNCATE faqs, articles, sessions, users CASCADE")
        .execute(&pool)
        .await
        .ok()?;
    let state = AppState {
        pool: pool.clone(),
        public_url: "http://localhost:5196".into(),
        meili_url: None,
        meili_key: None,
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5196");
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .ok()?;
    let addr = listener.local_addr().ok()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    Some((addr, pool))
}

async fn insert_article(pool: &PgPool, slug: &str, title: &str, summary: &str, body: &str) {
    sqlx::query!(
        r#"INSERT INTO articles (id, slug, title, summary, body_md, body_html, published_at)
           VALUES ($1,$2,$3,$4,$5,$6, now())"#,
        Uuid::new_v4(),
        slug,
        title,
        summary,
        body,
        format!("<p>{body}</p>")
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn fts_ranks_title_match_above_body_match() {
    let Some((addr, pool)) = fresh().await else {
        return;
    };
    insert_article(
        &pool,
        "kafka-intro",
        "Kafka basics for beginners",
        "Intro guide.",
        "Some unrelated body content.",
    )
    .await;
    insert_article(
        &pool,
        "trees",
        "Forests of the Pacific Northwest",
        "Ecology.",
        "Mentions Kafka once in passing.",
    )
    .await;

    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{base}/api/search?q=kafka"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let hits: Vec<serde_json::Value> = res.json().await.unwrap();
    assert!(hits.len() >= 2, "expected at least 2 hits: {hits:?}");
    assert_eq!(hits[0]["slug"], "kafka-intro", "title-match outranks body");
}

#[tokio::test]
async fn pg_trgm_fallback_handles_typos() {
    let Some((addr, pool)) = fresh().await else {
        return;
    };
    insert_article(
        &pool,
        "postgres-tuning",
        "Postgres tuning guide",
        "Performance tips.",
        "Body.",
    )
    .await;

    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    // "postgrss" — typo. FTS won't match; trgm should.
    let res = client
        .get(format!("{base}/api/search?q=postgrss"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let hits: Vec<serde_json::Value> = res.json().await.unwrap();
    assert!(!hits.is_empty(), "expected trgm fallback hit for typo");
    assert_eq!(hits[0]["slug"], "postgres-tuning");
}

#[tokio::test]
async fn empty_query_returns_empty() {
    let Some((addr, _)) = fresh().await else {
        return;
    };
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{base}/api/search?q=%20"))
        .send()
        .await
        .unwrap();
    let hits: Vec<serde_json::Value> = res.json().await.unwrap();
    assert!(hits.is_empty());
}
