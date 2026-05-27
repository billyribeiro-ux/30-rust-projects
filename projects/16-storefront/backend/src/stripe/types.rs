//! The smallest subset of Stripe types we actually decode from JSON.
//!
//! Stripe sends `additional_properties` style payloads (huge nested objects
//! with optional fields). We don't model the whole thing — just what we
//! need to fulfil orders. `serde(default)` + `Option` everywhere keeps us
//! robust against Stripe adding fields.

use serde::{Deserialize, Serialize};

/// Top-level webhook envelope. Stripe sends one per event.
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: WebhookData,
    #[serde(default)]
    #[allow(dead_code)]
    pub created: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookData {
    pub object: serde_json::Value,
}

/// Subset of a `checkout.session` we look at after a successful payment.
#[derive(Debug, Clone, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    #[serde(default)]
    pub customer_email: Option<String>,
    #[serde(default)]
    pub customer_details: Option<CustomerDetails>,
    #[serde(default)]
    pub payment_intent: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub amount_total: Option<i64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub currency: Option<String>,
    #[serde(default)]
    pub payment_status: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomerDetails {
    #[serde(default)]
    pub email: Option<String>,
}

/// Just enough of a Charge / PaymentIntent to recognise a refund event.
#[derive(Debug, Clone, Deserialize)]
pub struct RefundedCharge {
    #[serde(default)]
    pub payment_intent: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub refunded: Option<bool>,
}

/// Response from `POST /v1/checkout/sessions`. Only need the URL we'll
/// redirect the customer to.
#[derive(Debug, Clone, Deserialize)]
pub struct CreatedSession {
    pub id: String,
    pub url: String,
}

/// Refund response — we use it only to confirm the call succeeded.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefundResponse {
    pub id: String,
    pub status: String,
}
