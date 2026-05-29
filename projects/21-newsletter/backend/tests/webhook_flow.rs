//! Integration tests for the Stripe webhook handler — proves signature
//! verification, idempotency, fulfilment, dunning, downgrade.

use std::net::SocketAddr;

use newsletter_backend::AppState;
use newsletter_backend::build_app;
use newsletter_backend::routes::stripe as stripe_routes;
use newsletter_backend::stripe::client::StripeClient;
use newsletter_backend::stripe::webhook;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::net::TcpListener;
use uuid::Uuid;

const SECRET: &str = "whsec_test_xyz";

async fn maybe_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn spawn_app() -> Option<(SocketAddr, PgPool)> {
    let pool = maybe_pool().await?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    // Truncate the tables this test mutates so reruns are idempotent.
    sqlx::query!(
        "TRUNCATE subscriptions, stripe_events, magic_links, subscriber_sessions, subscribers, posts CASCADE"
    )
    .execute(&pool)
    .await
    .ok()?;
    let stripe = Arc::new(StripeClient::new("sk_test_local".into()));
    let state = AppState {
        pool: pool.clone(),
        stripe,
        stripe_webhook_secret: SECRET.into(),
        stripe_price_id: "price_local".into(),
        public_url: "http://localhost:5193".into(),
        smtp_url: "smtp://localhost:1026".into(),
        smtp_from: "noreply@newsletter.local".into(),
        secure_cookies: false,
    };
    let app = build_app(state, "http://localhost:5193");
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
async fn checkout_completed_creates_pro_subscription_and_is_idempotent() {
    let Some((addr, pool)) = spawn_app().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // Pre-create the subscriber so the customer_details email matches.
    sqlx::query!(
        "INSERT INTO subscribers (email) VALUES ($1)",
        "alice@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    let body = serde_json::json!({
        "id": "evt_001",
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "customer": "cus_001",
                "subscription": "sub_001",
                "customer_details": { "email": "alice@example.com" }
            }
        }
    })
    .to_string();

    let res = post_webhook(&client, &base, &body).await;
    assert_eq!(res.status(), 200);

    let plan: Option<String> = sqlx::query_scalar!(
        "SELECT plan FROM subscriptions WHERE stripe_customer_id = $1",
        "cus_001"
    )
    .fetch_optional(&pool)
    .await
    .unwrap()
    .map(|v| v.to_string());
    assert_eq!(plan.as_deref(), Some("pro"));

    // Idempotency: replay returns 200 but doesn't double-fulfil.
    let res2 = post_webhook(&client, &base, &body).await;
    assert_eq!(res2.status(), 200);
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM subscriptions WHERE stripe_customer_id = $1",
        "cus_001"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(count, 1, "duplicate webhook must not double-insert");
}

#[tokio::test]
async fn payment_failed_marks_past_due_and_invoice_paid_recovers() {
    let Some((addr, pool)) = spawn_app().await else {
        return;
    };
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    let sid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO subscribers (id, email) VALUES ($1, $2)",
        sid,
        "bob@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO subscriptions (subscriber_id, stripe_customer_id, stripe_subscription_id, plan, status)
         VALUES ($1, $2, $3, 'pro', 'active')",
        sid,
        "cus_002",
        "sub_002"
    )
    .execute(&pool)
    .await
    .unwrap();

    let failed = serde_json::json!({
        "id": "evt_002",
        "type": "invoice.payment_failed",
        "data": { "object": { "customer": "cus_002" } }
    })
    .to_string();
    let res = post_webhook(&client, &base, &failed).await;
    assert_eq!(res.status(), 200);
    let status: String = sqlx::query_scalar!(
        "SELECT status FROM subscriptions WHERE stripe_customer_id = $1",
        "cus_002"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "past_due");

    let paid = serde_json::json!({
        "id": "evt_003",
        "type": "invoice.paid",
        "data": { "object": { "customer": "cus_002" } }
    })
    .to_string();
    let res = post_webhook(&client, &base, &paid).await;
    assert_eq!(res.status(), 200);
    let status: String = sqlx::query_scalar!(
        "SELECT status FROM subscriptions WHERE stripe_customer_id = $1",
        "cus_002"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "active");
}

#[tokio::test]
async fn dunning_downgrade_after_grace_period() {
    let Some((_addr, pool)) = spawn_app().await else {
        return;
    };
    let sid = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO subscribers (id, email) VALUES ($1, $2)",
        sid,
        "carl@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();
    // Past-due 15 days ago — beyond the 14-day grace.
    sqlx::query!(
        r#"INSERT INTO subscriptions (subscriber_id, stripe_customer_id, plan, status, past_due_since)
           VALUES ($1, $2, 'pro', 'past_due', now() - INTERVAL '15 days')"#,
        sid,
        "cus_003",
    )
    .execute(&pool)
    .await
    .unwrap();

    let ids = stripe_routes::find_past_due_to_downgrade(&pool)
        .await
        .unwrap();
    assert!(ids.contains(&sid));
    stripe_routes::downgrade_to_free(&pool, sid).await.unwrap();
    let plan: String = sqlx::query_scalar!(
        "SELECT plan FROM subscriptions WHERE subscriber_id = $1",
        sid
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(plan, "free");
}

#[tokio::test]
async fn webhook_signature_mismatch_is_401() {
    let Some((addr, _pool)) = spawn_app().await else {
        return;
    };
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    let body = r#"{"id":"x","type":"x"}"#;
    let bad_sig = webhook::sign_for_test(
        body.as_bytes(),
        "wrong-secret",
        chrono::Utc::now().timestamp(),
    );
    let res = client
        .post(format!("{base}/api/stripe/webhook"))
        .header("stripe-signature", bad_sig)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}
