//! Hand-rolled minimal Stripe API client.
//!
//! We don't pull `stripe-rust` for this project because the whole point of
//! the lesson is to demystify what's behind that crate: HTTP Basic auth
//! with `secret_key:` as the user (empty password), url-encoded form
//! bodies, idempotency keys, and JSON responses.
//!
//! All requests go through `reqwest::Client`. The base URL is injectable
//! so the tests can point us at `wiremock`.

use reqwest::Client;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

use crate::error::{AppError, AppResult};
use crate::stripe::types::{CreatedSession, RefundResponse};

#[derive(Clone, Debug)]
pub struct StripeClient {
    http: Client,
    api_base: String,
    secret_key: String,
}

impl StripeClient {
    pub fn new(api_base: impl Into<String>, secret_key: impl Into<String>) -> AppResult<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| AppError::Internal(format!("reqwest build: {e}")))?;
        Ok(Self {
            http,
            api_base: api_base.into(),
            secret_key: secret_key.into(),
        })
    }

    /// Create a Checkout Session.
    /// Stripe form-encodes `line_items[0][price_data][...]=...`.
    /// We pass the customer email so Stripe pre-fills the receipt.
    pub async fn create_checkout_session(
        &self,
        customer_email: &str,
        product_name: &str,
        price_cents: i64,
        currency: &str,
        success_url: &str,
        cancel_url: &str,
        idempotency_key: &str,
        metadata: &HashMap<String, String>,
    ) -> AppResult<CreatedSession> {
        // Build the form body. Stripe's nested-array form encoding is
        // `line_items[0][price_data][unit_amount]=...`.
        let mut form: Vec<(String, String)> = vec![
            ("mode".into(), "payment".into()),
            ("customer_email".into(), customer_email.into()),
            ("success_url".into(), success_url.into()),
            ("cancel_url".into(), cancel_url.into()),
            ("line_items[0][quantity]".into(), "1".into()),
            (
                "line_items[0][price_data][currency]".into(),
                currency.into(),
            ),
            (
                "line_items[0][price_data][unit_amount]".into(),
                price_cents.to_string(),
            ),
            (
                "line_items[0][price_data][product_data][name]".into(),
                product_name.into(),
            ),
        ];
        for (k, v) in metadata {
            form.push((format!("metadata[{k}]"), v.clone()));
        }

        self.post::<CreatedSession>(
            "/v1/checkout/sessions",
            &form,
            Some(idempotency_key),
        )
        .await
    }

    /// Refund a payment by its PaymentIntent id.
    pub async fn refund_payment_intent(
        &self,
        payment_intent: &str,
        idempotency_key: &str,
    ) -> AppResult<RefundResponse> {
        let form = vec![("payment_intent".to_string(), payment_intent.to_string())];
        self.post::<RefundResponse>("/v1/refunds", &form, Some(idempotency_key))
            .await
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        form: &[(String, String)],
        idempotency_key: Option<&str>,
    ) -> AppResult<T> {
        let url = format!("{}{}", self.api_base.trim_end_matches('/'), path);
        let mut req = self
            .http
            .post(&url)
            .basic_auth(&self.secret_key, Some(""))
            .form(form);
        if let Some(k) = idempotency_key {
            req = req.header("Idempotency-Key", k);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("send {path}: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| AppError::Upstream(format!("read {path}: {e}")))?;
        if !status.is_success() {
            return Err(AppError::Upstream(format!(
                "Stripe {path} returned {status}: {text}"
            )));
        }
        serde_json::from_str::<T>(&text)
            .map_err(|e| AppError::Upstream(format!("parse {path}: {e} (body: {text})")))
    }
}
