use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;

use crate::email::Mailer;
use crate::stripe::client::StripeClient;

/// Wide-but-flat AppState. Everything cheap to Clone (Arc'd internally) so
/// the request handlers don't take exclusive ownership.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub mailer: Arc<Mailer>,
    pub stripe: Arc<StripeClient>,
    pub public_url: String,
    pub secure_cookies: bool,
    /// HMAC-SHA256 key for signed download URLs. 32 bytes recommended.
    pub download_signing_secret: Arc<Vec<u8>>,
    /// HMAC-SHA256 key Stripe sends in `Stripe-Signature` — raw bytes of
    /// whatever the `STRIPE_WEBHOOK_SECRET` env var contains. (Stripe's
    /// own secret looks like `whsec_...`; we hash with whatever's there.)
    pub stripe_webhook_secret: Arc<Vec<u8>>,
    pub product_files_dir: PathBuf,
}
