//! Tests for the hand-rolled Stripe client.
//!
//! We mount `wiremock` on a random port and point a freshly-built
//! `StripeClient` at it. No live network calls; the mock asserts that
//! we send the right form fields and headers, and replies with a
//! canned JSON body that matches Stripe's documented shape.

use std::collections::HashMap;
use storefront_backend::stripe_client_for_tests as stripe_client;
use storefront_backend::stripe_types_for_tests as types;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn create_checkout_session_posts_expected_form() {
    let mock_server = MockServer::start().await;

    let response = serde_json::json!({
        "id": "cs_test_a1B2c3",
        "url": "https://checkout.stripe.com/test/cs_test_a1B2c3",
        "object": "checkout.session"
    });

    Mock::given(method("POST"))
        .and(path("/v1/checkout/sessions"))
        .and(header("Idempotency-Key", "idem-001"))
        .and(body_string_contains("mode=payment"))
        .and(body_string_contains("customer_email=buyer%40example.com"))
        .and(body_string_contains("line_items%5B0%5D%5Bprice_data%5D%5Bunit_amount%5D=999"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = stripe_client::StripeClient::new(mock_server.uri(), "sk_test_dummy").unwrap();
    let mut metadata = HashMap::new();
    metadata.insert("order_id".into(), "abc".into());

    let session = client
        .create_checkout_session(
            "buyer@example.com",
            "My PDF",
            999,
            "usd",
            "http://x/ok",
            "http://x/cancel",
            "idem-001",
            &metadata,
        )
        .await
        .unwrap();

    assert_eq!(session.id, "cs_test_a1B2c3");
    assert!(session.url.starts_with("https://checkout.stripe.com/"));
}

#[tokio::test]
async fn refund_payment_intent_posts_expected_form() {
    let mock_server = MockServer::start().await;
    let response = serde_json::json!({
        "id": "re_1ABC",
        "object": "refund",
        "status": "succeeded"
    });
    Mock::given(method("POST"))
        .and(path("/v1/refunds"))
        .and(header("Idempotency-Key", "rf-1"))
        .and(body_string_contains("payment_intent=pi_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = stripe_client::StripeClient::new(mock_server.uri(), "sk_test_dummy").unwrap();
    let r = client.refund_payment_intent("pi_123", "rf-1").await.unwrap();
    assert_eq!(r.id, "re_1ABC");
    assert_eq!(r.status, "succeeded");
}

#[tokio::test]
async fn stripe_error_is_returned_as_upstream() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/checkout/sessions"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"error":{"message":"bad"}})),
        )
        .mount(&mock_server)
        .await;

    let client = stripe_client::StripeClient::new(mock_server.uri(), "sk_test_x").unwrap();
    let err = client
        .create_checkout_session(
            "a@b.com",
            "x",
            100,
            "usd",
            "http://x",
            "http://x",
            "i",
            &HashMap::new(),
        )
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("Stripe"));
}

#[tokio::test]
async fn parse_event_recognises_checkout_completed() {
    let raw = r#"{
        "id": "evt_test",
        "type": "checkout.session.completed",
        "data": { "object": { "id": "cs_test_x" } }
    }"#;
    let evt: types::WebhookEvent = serde_json::from_str(raw).unwrap();
    assert_eq!(evt.id, "evt_test");
    assert_eq!(evt.event_type, "checkout.session.completed");
}
