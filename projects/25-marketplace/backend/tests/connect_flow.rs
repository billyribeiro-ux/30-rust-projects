//! Connect webhook flow proofs.
//!
//! We never call live Stripe in tests. We hand-craft signed webhook
//! bodies (`webhook::sign_for_test`) and POST them at the endpoint.

use marketplace_backend::AppState;
use marketplace_backend::build_app;
use marketplace_backend::stripe::client::StripeClient;
use marketplace_backend::stripe::webhook;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use uuid::Uuid;

const SECRET: &str = "whsec_test_marketplace";

async fn maybe_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn fresh() -> Option<(SocketAddr, PgPool)> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    sqlx::query!(
        "TRUNCATE enrollments, lessons, courses, instructors, stripe_events, sessions, users CASCADE"
    )
    .execute(&pool)
    .await
    .ok()?;
    let stripe = Arc::new(StripeClient::new("sk_test_local".into()));
    let state = AppState {
        pool: pool.clone(),
        stripe,
        stripe_webhook_secret: SECRET.into(),
        platform_fee_bps: 1000,
        public_url: "http://localhost:5197".into(),
        download_signing_key: "0".repeat(64),
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5197");
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .ok()?;
    let addr = listener.local_addr().ok()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    Some((addr, pool))
}

async fn post_webhook(client: &reqwest::Client, base: &str, body: &str) -> reqwest::Response {
    let now = chrono::Utc::now().timestamp();
    let sig = webhook::sign_for_test(body.as_bytes(), SECRET, now);
    client
        .post(format!("{base}/api/stripe/webhook"))
        .header("stripe-signature", sig)
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn account_updated_flips_payouts_enabled() {
    let Some((addr, pool)) = fresh().await else {
        eprintln!("skip: no DATABASE_URL");
        return;
    };
    let base = format!("http://{addr}");
    let user_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO users (id, email, password_hash, name, role)
           VALUES ($1, $2, $3, $4, 'instructor')"#,
        user_id,
        "i@example.com",
        "x",
        "I"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO instructors (user_id, stripe_account_id) VALUES ($1, $2)",
        user_id,
        "acct_xyz"
    )
    .execute(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({
        "id": "evt_acct_1",
        "type": "account.updated",
        "data": { "object": {
            "id": "acct_xyz",
            "payouts_enabled": true,
            "details_submitted": true
        }}
    })
    .to_string();

    let res = post_webhook(&reqwest::Client::new(), &base, &body).await;
    assert_eq!(res.status(), 200);
    let payouts: bool = sqlx::query_scalar!(
        "SELECT payouts_enabled FROM instructors WHERE stripe_account_id = $1",
        "acct_xyz"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(payouts);
}

#[tokio::test]
async fn checkout_completed_creates_enrollment_and_is_idempotent() {
    let Some((addr, pool)) = fresh().await else {
        return;
    };
    let base = format!("http://{addr}");

    let instructor_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO users (id, email, password_hash, name, role)
           VALUES ($1, $2, $3, $4, 'instructor')"#,
        instructor_id,
        "ins@example.com",
        "x",
        "Ins"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO instructors (user_id, stripe_account_id, payouts_enabled) VALUES ($1, 'acct_a', TRUE)",
        instructor_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let course_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO courses (id, instructor_id, slug, title, price_cents, status)
           VALUES ($1, $2, $3, $4, $5, 'published')"#,
        course_id,
        instructor_id,
        "rust-101",
        "Rust 101",
        4900_i64
    )
    .execute(&pool)
    .await
    .unwrap();
    let student_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO users (id, email, password_hash, name) VALUES ($1, $2, $3, $4)"#,
        student_id,
        "alice@example.com",
        "x",
        "Alice"
    )
    .execute(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({
        "id": "evt_co_1",
        "type": "checkout.session.completed",
        "data": { "object": {
            "payment_intent": "pi_001",
            "customer_details": { "email": "alice@example.com" },
            "metadata": { "course_slug": "rust-101" }
        }}
    })
    .to_string();
    let res = post_webhook(&reqwest::Client::new(), &base, &body).await;
    assert_eq!(res.status(), 200);
    let cnt: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "c!" FROM enrollments WHERE course_id = $1 AND student_user_id = $2"#,
        course_id,
        student_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cnt, 1);

    // Replay same event — no duplicate.
    let res = post_webhook(&reqwest::Client::new(), &base, &body).await;
    assert_eq!(res.status(), 200);
    let cnt: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "c!" FROM enrollments WHERE course_id = $1 AND student_user_id = $2"#,
        course_id,
        student_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cnt, 1, "replay must not double-enroll");
}

#[tokio::test]
async fn signature_mismatch_returns_401() {
    let Some((addr, _)) = fresh().await else {
        return;
    };
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    let body = r#"{"id":"x","type":"x","data":{"object":{}}}"#;
    let bad =
        webhook::sign_for_test(body.as_bytes(), "wrong", chrono::Utc::now().timestamp());
    let res = client
        .post(format!("{base}/api/stripe/webhook"))
        .header("stripe-signature", bad)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}
