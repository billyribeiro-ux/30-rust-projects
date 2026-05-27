//! Library crate exposing pure modules so integration tests can import
//! them. The binary in `main.rs` re-uses these.
//!
//! By keeping pure logic (signing, webhook HMAC, Stripe client, types)
//! here, the wiremock-backed tests in `tests/` don't have to spin up
//! the whole Axum app to exercise it.

pub mod auth;
pub mod db;
pub mod email;
pub mod error;
pub mod routes;
pub mod signing;
pub mod state;
pub mod stripe;

// Stable aliases for the test crate, since `stripe` is a sensitive name
// (we never pull `stripe-rust`, but it would conflict if we did).
pub mod stripe_client_for_tests {
    pub use crate::stripe::client::*;
}
pub mod stripe_types_for_tests {
    pub use crate::stripe::types::*;
}
pub mod stripe_webhook_for_tests {
    pub use crate::stripe::webhook::*;
}
