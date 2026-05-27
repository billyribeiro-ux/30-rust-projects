//! HMAC-SHA256 signed download URLs.
//!
//! We don't reuse Stripe's signature format for our own URLs: the download
//! endpoint receives a single opaque token in the path. Two layers of
//! defence:
//!  1. The URL is signed: the token is a `<random>.<hmac>` pair, and we
//!     compare in constant time, so an attacker can't forge a path.
//!  2. The token's SHA-256 hash is stored in `download_links` with an
//!     `expires_at` and an optional `used_at`. So even a leaked signed
//!     URL stops working after 24h or after first use (revoke-on-use
//!     is configurable in the handler).
//!
//! The hash-stored model is the same as session tokens (project 11) — we
//! never store the raw token, so a DB dump alone is not enough to impersonate
//! a customer's download link.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Generate a 32-byte random nonce, base64url-encoded (no padding).
pub fn random_nonce() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Sign `nonce` with `secret`. Returns `"<nonce>.<hmac_b64>"`.
///
/// The caller embeds the result in the URL: `/d/<signed>`.
pub fn sign(secret: &[u8], nonce: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length, including empty");
    mac.update(nonce.as_bytes());
    let sig = mac.finalize().into_bytes();
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig);
    format!("{nonce}.{sig_b64}")
}

/// Verify a signed token, returning the inner nonce if and only if the
/// signature is valid. Constant-time compare; resistant to tampering.
pub fn verify<'a>(secret: &[u8], token: &'a str) -> Option<&'a str> {
    let (nonce, sig_b64) = token.split_once('.')?;
    let provided_sig = URL_SAFE_NO_PAD.decode(sig_b64).ok()?;

    let mut mac = HmacSha256::new_from_slice(secret).ok()?;
    mac.update(nonce.as_bytes());
    let expected_sig = mac.finalize().into_bytes();

    if expected_sig.ct_eq(&provided_sig).into() {
        Some(nonce)
    } else {
        None
    }
}

/// SHA-256 of the *raw token* (the full `<nonce>.<sig>` string) — that's
/// what we store in `download_links.token_hash`, so even the signed URL
/// itself isn't recoverable from a DB dump.
pub fn hash_token(token: &str) -> Vec<u8> {
    use sha2::Digest;
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    h.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_verifies() {
        let secret = b"a-32-byte-secret-or-anything-tbh";
        let nonce = random_nonce();
        let signed = sign(secret, &nonce);
        assert_eq!(verify(secret, &signed), Some(nonce.as_str()));
    }

    #[test]
    fn wrong_secret_rejected() {
        let nonce = random_nonce();
        let signed = sign(b"correct-secret", &nonce);
        assert!(verify(b"wrong-secret", &signed).is_none());
    }

    #[test]
    fn tampered_nonce_rejected() {
        let secret = b"my-secret";
        let signed = sign(secret, "original-nonce");
        // Swap nonce, keep signature
        let parts: Vec<&str> = signed.splitn(2, '.').collect();
        let tampered = format!("malicious-nonce.{}", parts[1]);
        assert!(verify(secret, &tampered).is_none());
    }

    #[test]
    fn tampered_sig_rejected() {
        let secret = b"my-secret";
        let signed = sign(secret, "the-nonce");
        let parts: Vec<&str> = signed.splitn(2, '.').collect();
        let tampered = format!("{}.{}", parts[0], "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        assert!(verify(secret, &tampered).is_none());
    }

    #[test]
    fn malformed_token_rejected() {
        assert!(verify(b"k", "no-dot-here").is_none());
        assert!(verify(b"k", ".").is_none());
        assert!(verify(b"k", "x.not-base64!!!").is_none());
    }

    #[test]
    fn hash_is_deterministic_and_32_bytes() {
        let h1 = hash_token("hello");
        let h2 = hash_token("hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
    }
}
