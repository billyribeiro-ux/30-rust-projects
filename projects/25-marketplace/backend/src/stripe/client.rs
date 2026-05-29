//! Stripe Connect client. Hand-rolled (no `async-stripe` dep) because the
//! lesson is the surface: account creation, account-link onboarding,
//! Checkout sessions with `application_fee_amount` + `transfer_data`,
//! and refunds.

use serde::Deserialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub struct StripeClient {
    http: reqwest::Client,
    base_url: String,
    secret_key: String,
}

#[derive(Debug, Deserialize)]
pub struct AccountResp {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct AccountLinkResp {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutSessionResp {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct RefundResp {
    pub id: String,
}

impl StripeClient {
    pub fn new(secret_key: String) -> Self {
        Self::new_with_base(secret_key, "https://api.stripe.com".into())
    }

    pub fn new_with_base(secret_key: String, base_url: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("marketplace-backend/0.1")
                .build()
                .expect("reqwest builds"),
            base_url,
            secret_key,
        }
    }

    /// Create a Connect Express account for a new instructor.
    pub async fn create_express_account(
        &self,
        email: &str,
        country: &str,
    ) -> AppResult<AccountResp> {
        let body: Vec<(&str, String)> = vec![
            ("type", "express".into()),
            ("email", email.into()),
            ("country", country.into()),
            ("capabilities[card_payments][requested]", "true".into()),
            ("capabilities[transfers][requested]", "true".into()),
        ];
        let res = self
            .http
            .post(format!("{}/v1/accounts", self.base_url))
            .basic_auth(&self.secret_key, Some(""))
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .form(&body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("stripe accounts: {e}")))?;
        if !res.status().is_success() {
            return Err(AppError::Upstream(res.text().await.unwrap_or_default()));
        }
        res.json()
            .await
            .map_err(|e| AppError::Upstream(format!("decode: {e}")))
    }

    /// One-time URL the instructor visits to finish KYC.
    pub async fn create_account_link(
        &self,
        account_id: &str,
        return_url: &str,
        refresh_url: &str,
    ) -> AppResult<AccountLinkResp> {
        let body: Vec<(&str, String)> = vec![
            ("account", account_id.into()),
            ("return_url", return_url.into()),
            ("refresh_url", refresh_url.into()),
            ("type", "account_onboarding".into()),
        ];
        let res = self
            .http
            .post(format!("{}/v1/account_links", self.base_url))
            .basic_auth(&self.secret_key, Some(""))
            .form(&body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("stripe account_links: {e}")))?;
        if !res.status().is_success() {
            return Err(AppError::Upstream(res.text().await.unwrap_or_default()));
        }
        res.json()
            .await
            .map_err(|e| AppError::Upstream(format!("decode: {e}")))
    }

    /// One-shot purchase with platform fee + transfer to instructor.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_destination_checkout(
        &self,
        student_email: &str,
        course_title: &str,
        amount_cents: i64,
        currency: &str,
        platform_fee_cents: i64,
        destination_account_id: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> AppResult<CheckoutSessionResp> {
        let body: Vec<(&str, String)> = vec![
            ("mode", "payment".into()),
            ("customer_email", student_email.into()),
            ("success_url", success_url.into()),
            ("cancel_url", cancel_url.into()),
            ("line_items[0][quantity]", "1".into()),
            ("line_items[0][price_data][currency]", currency.into()),
            (
                "line_items[0][price_data][unit_amount]",
                amount_cents.to_string(),
            ),
            (
                "line_items[0][price_data][product_data][name]",
                course_title.into(),
            ),
            (
                "payment_intent_data[application_fee_amount]",
                platform_fee_cents.to_string(),
            ),
            (
                "payment_intent_data[transfer_data][destination]",
                destination_account_id.into(),
            ),
        ];
        let res = self
            .http
            .post(format!("{}/v1/checkout/sessions", self.base_url))
            .basic_auth(&self.secret_key, Some(""))
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .form(&body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("stripe checkout: {e}")))?;
        if !res.status().is_success() {
            return Err(AppError::Upstream(res.text().await.unwrap_or_default()));
        }
        res.json()
            .await
            .map_err(|e| AppError::Upstream(format!("decode: {e}")))
    }

    pub async fn refund_payment_intent(&self, pi_id: &str) -> AppResult<RefundResp> {
        let body: Vec<(&str, String)> = vec![
            ("payment_intent", pi_id.into()),
            ("reverse_transfer", "true".into()),
            ("refund_application_fee", "true".into()),
        ];
        let res = self
            .http
            .post(format!("{}/v1/refunds", self.base_url))
            .basic_auth(&self.secret_key, Some(""))
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .form(&body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("stripe refunds: {e}")))?;
        if !res.status().is_success() {
            return Err(AppError::Upstream(res.text().await.unwrap_or_default()));
        }
        res.json()
            .await
            .map_err(|e| AppError::Upstream(format!("decode: {e}")))
    }
}
