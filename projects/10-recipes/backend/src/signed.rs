//! HMAC-SHA256 signed URLs with explicit expiry.
//!
//! Use case: project 10 needs to hand a friend a link to a recipe without
//! exposing the entire site or rolling auth. Signed URLs are the classic
//! pattern — the server vouches for "(slug, expires_at) was minted by us"
//! via a secret-keyed HMAC.
//!
//! Design notes:
//! - `HMAC-SHA256` (NOT bare SHA-256 of "secret||message"). HMAC is the
//!   construction; SHA-256 is the hash. Bare hashes are vulnerable to
//!   length-extension attacks.
//! - Signature is `base64url` (URL-safe, no padding) so it lives cleanly in
//!   `?sig=` without percent-encoding surprises.
//! - Constant-time verify via `hmac::Mac::verify_slice`. NEVER `==` on
//!   secret material — that leaks timing information about the prefix match.
//! - Expiry is part of the signed payload. A caller can't extend their
//!   own link by editing `?exp=` because that would invalidate `?sig=`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Server-side signing key. Construct once at startup with a 32+ byte secret.
#[derive(Clone)]
pub struct Signer {
    key: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("invalid signature")]
    Invalid,
    #[error("link has expired")]
    Expired,
}

impl Signer {
    pub fn new(key: impl Into<Vec<u8>>) -> Self {
        Self { key: key.into() }
    }

    /// Sign `"{slug}|{exp_unix}"` with HMAC-SHA256 and return a URL-safe
    /// base64 string (no padding).
    pub fn sign(&self, slug: &str, exp_unix: i64) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(format!("{slug}|{exp_unix}").as_bytes());
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    }

    /// Verify a signature against `(slug, exp_unix)`. Returns `Ok(())` if
    /// the signature matches AND `now_unix < exp_unix`. The constant-time
    /// compare comes from `mac.verify_slice`.
    pub fn verify(
        &self,
        slug: &str,
        exp_unix: i64,
        sig: &str,
        now_unix: i64,
    ) -> Result<(), SignError> {
        if now_unix >= exp_unix {
            return Err(SignError::Expired);
        }
        let raw = URL_SAFE_NO_PAD
            .decode(sig.as_bytes())
            .map_err(|_| SignError::Invalid)?;
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(format!("{slug}|{exp_unix}").as_bytes());
        mac.verify_slice(&raw).map_err(|_| SignError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s() -> Signer {
        Signer::new(b"test-secret-32-bytes-of-key!!!!!".to_vec())
    }

    #[test]
    fn sign_then_verify_roundtrips() {
        let signer = s();
        let sig = signer.sign("apple-pie", 2_000_000_000);
        assert!(signer.verify("apple-pie", 2_000_000_000, &sig, 1).is_ok());
    }

    #[test]
    fn verify_rejects_tampered_slug() {
        let signer = s();
        let sig = signer.sign("apple-pie", 2_000_000_000);
        assert!(matches!(
            signer.verify("banana-pie", 2_000_000_000, &sig, 1),
            Err(SignError::Invalid)
        ));
    }

    #[test]
    fn verify_rejects_tampered_exp() {
        let signer = s();
        let sig = signer.sign("apple-pie", 2_000_000_000);
        // Caller bumps exp without re-signing — should fail.
        assert!(matches!(
            signer.verify("apple-pie", 2_100_000_000, &sig, 1),
            Err(SignError::Invalid)
        ));
    }

    #[test]
    fn verify_rejects_expired() {
        let signer = s();
        let sig = signer.sign("apple-pie", 100);
        // now > exp
        assert!(matches!(
            signer.verify("apple-pie", 100, &sig, 200),
            Err(SignError::Expired)
        ));
    }

    #[test]
    fn verify_rejects_garbage_sig() {
        let signer = s();
        assert!(matches!(
            signer.verify("apple-pie", 2_000_000_000, "!!!not-base64!!!", 1),
            Err(SignError::Invalid)
        ));
    }

    #[test]
    fn signatures_are_url_safe_no_pad() {
        let sig = s().sign("apple-pie", 2_000_000_000);
        // base64url alphabet only — no '+', '/' or '=' that would need
        // percent-encoding in a query string.
        assert!(
            sig.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }
}
