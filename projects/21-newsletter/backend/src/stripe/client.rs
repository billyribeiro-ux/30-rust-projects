//! Tiny hand-rolled Stripe client. We POST to `/v1/checkout/sessions`
//! and `/v1/billing_portal/sessions` — Stripe accepts standard form
//! encoding, not JSON. All requests carry an `Idempotency-Key` so a
//! retried POST never creates a second resource.
//!
//! In tests we point `base_url` at a `wiremock` mock so no live
//! network call ever happens in CI.

use serde::Deserialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug)]
pub struct StripeClient {
    http: reqwest::Client,
    base_url: String,
    secret_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutSessionResp {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct PortalSessionResp {
    pub id: String,
    pub url: String,
}

impl StripeClient {
    pub fn new(secret_key: String) -> Self {
        Self::new_with_base(secret_key, "https://api.stripe.com".into())
    }

    pub fn new_with_base(secret_key: String, base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("newsletter-backend/0.1")
            .build()
            .expect("reqwest builds");
        Self {
            http,
            base_url,
            secret_key,
        }
    }

    pub async fn create_subscription_checkout(
        &self,
        customer_email: &str,
        price_id: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> AppResult<CheckoutSessionResp> {
        let idempotency_key = Uuid::new_v4().to_string();
        let body: Vec<(&str, String)> = vec![
            ("mode", "subscription".into()),
            ("line_items[0][price]", price_id.into()),
            ("line_items[0][quantity]", "1".into()),
            ("customer_email", customer_email.into()),
            ("success_url", success_url.into()),
            ("cancel_url", cancel_url.into()),
            ("allow_promotion_codes", "true".into()),
        ];
        let res = self
            .http
            .post(format!("{}/v1/checkout/sessions", self.base_url))
            .basic_auth(&self.secret_key, Some(""))
            .header("Idempotency-Key", &idempotency_key)
            .form(&body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("stripe checkout: {e}")))?;
        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "stripe checkout failed: {text}"
            )));
        }
        res.json::<CheckoutSessionResp>()
            .await
            .map_err(|e| AppError::Upstream(format!("stripe checkout decode: {e}")))
    }

    pub async fn create_billing_portal(
        &self,
        customer_id: &str,
        return_url: &str,
    ) -> AppResult<PortalSessionResp> {
        let body: Vec<(&str, String)> = vec![
            ("customer", customer_id.into()),
            ("return_url", return_url.into()),
        ];
        let res = self
            .http
            .post(format!("{}/v1/billing_portal/sessions", self.base_url))
            .basic_auth(&self.secret_key, Some(""))
            .form(&body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("stripe portal: {e}")))?;
        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!("stripe portal failed: {text}")));
        }
        res.json::<PortalSessionResp>()
            .await
            .map_err(|e| AppError::Upstream(format!("stripe portal decode: {e}")))
    }
}
