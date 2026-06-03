//! PKCE + state + nonce primitives.
//!
//! Spec references:
//!   * PKCE: RFC 7636 — `code_challenge_method = S256`, `code_challenge`
//!     is `BASE64URL(SHA256(verifier))` with no padding. The verifier
//!     must be 43-128 unreserved ASCII chars.
//!   * `state`: RFC 6749 §10.12 — anti-CSRF nonce, opaque to client.
//!   * `nonce` (OIDC): bound into the id_token; we verify on callback.
//!
//! All three are produced from the OS CSPRNG and base64url-encoded so
//! they're URL-safe with no `=` padding.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngExt;
use sha2::{Digest, Sha256};

/// 32 random bytes → 43-char base64url string. Used for `state`, `nonce`,
/// and `code_verifier`. We standardise on 32 bytes for simplicity; spec
/// allows up to 96 for the verifier.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn code_challenge_s256(verifier: &str) -> String {
    let mut h = Sha256::new();
    h.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(h.finalize())
}

/// Constant-time equality so a timing side-channel can't fish out state
/// values one byte at a time.
pub fn constant_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_token_is_43_chars() {
        // 32 bytes -> ceil(32 * 4/3) = 43 chars, base64url no padding.
        let t = random_token();
        assert_eq!(t.len(), 43);
    }

    #[test]
    fn challenge_matches_rfc7636_appendix_b() {
        // The verifier/challenge pair from RFC 7636 Appendix B. The
        // SHA-256 of the verifier string must equal the published
        // challenge, base64url-encoded with no padding.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = code_challenge_s256(verifier);
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn constant_eq_rejects_different_lengths() {
        assert!(!constant_eq("abc", "abcd"));
        assert!(constant_eq("hello", "hello"));
        assert!(!constant_eq("hello", "world"));
    }
}
