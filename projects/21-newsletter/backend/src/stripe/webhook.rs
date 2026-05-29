//! Stripe webhook signature verification, hand-rolled per their spec:
//! https://docs.stripe.com/webhooks/signatures.
//!
//! Header shape: `t=1609459200,v1=hex_hmac,v1=hex_hmac_for_old_secret`
//! The MAC is `HMAC-SHA256("{t}.{raw_body}", secret)`. We accept multiple
//! `v1=` values so a secret rotation has a window where both work.
//! 5-minute replay window — anything older is dropped.

use hmac::{Hmac, Mac};
use sha2::Sha256;

const REPLAY_TOLERANCE_SECS: i64 = 300;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
    MissingHeader,
    Malformed,
    BadTimestamp,
    Replay,
    SignatureMismatch,
}

pub fn verify(
    raw_body: &[u8],
    sig_header: &str,
    secret: &str,
    now_unix: i64,
) -> Result<(), VerifyError> {
    if sig_header.is_empty() {
        return Err(VerifyError::MissingHeader);
    }
    let mut t: Option<i64> = None;
    let mut v1s: Vec<&str> = Vec::new();
    for part in sig_header.split(',') {
        let (k, v) = part.split_once('=').ok_or(VerifyError::Malformed)?;
        match k {
            "t" => {
                t = Some(v.parse().map_err(|_| VerifyError::BadTimestamp)?);
            }
            "v1" => v1s.push(v),
            _ => { /* future scheme */ }
        }
    }
    let ts = t.ok_or(VerifyError::Malformed)?;
    if (now_unix - ts).abs() > REPLAY_TOLERANCE_SECS {
        return Err(VerifyError::Replay);
    }
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| VerifyError::Malformed)?;
    mac.update(ts.to_string().as_bytes());
    mac.update(b".");
    mac.update(raw_body);
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex::encode(expected);

    for v in v1s {
        // Constant-time compare; we walk all to keep timing flat.
        if subtle::ConstantTimeEq::ct_eq(v.as_bytes(), expected_hex.as_bytes()).unwrap_u8() == 1 {
            return Ok(());
        }
    }
    Err(VerifyError::SignatureMismatch)
}

/// Sign helper for test-side fixtures.
pub fn sign_for_test(raw_body: &[u8], secret: &str, now_unix: i64) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(now_unix.to_string().as_bytes());
    mac.update(b".");
    mac.update(raw_body);
    let sig = hex::encode(mac.finalize().into_bytes());
    format!("t={now_unix},v1={sig}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let secret = "whsec_test";
        let body = br#"{"id":"evt_1","type":"customer.subscription.created"}"#;
        let h = sign_for_test(body, secret, 1_700_000_000);
        verify(body, &h, secret, 1_700_000_000).unwrap();
    }

    #[test]
    fn rejects_replay() {
        let secret = "whsec_test";
        let body = b"x";
        let h = sign_for_test(body, secret, 1_700_000_000);
        assert_eq!(
            verify(body, &h, secret, 1_700_000_400),
            Err(VerifyError::Replay)
        );
    }

    #[test]
    fn rejects_tampered_body() {
        let secret = "whsec_test";
        let body = b"original";
        let h = sign_for_test(body, secret, 1_700_000_000);
        assert_eq!(
            verify(b"tampered", &h, secret, 1_700_000_000),
            Err(VerifyError::SignatureMismatch)
        );
    }

    #[test]
    fn rejects_wrong_secret() {
        let h = sign_for_test(b"x", "right", 1_700_000_000);
        assert_eq!(
            verify(b"x", &h, "wrong", 1_700_000_000),
            Err(VerifyError::SignatureMismatch)
        );
    }

    #[test]
    fn accepts_multiple_v1_for_rotation() {
        let secret = "new";
        let body = b"y";
        let real = sign_for_test(body, secret, 1_700_000_000);
        // Splice in a stale v1 from a rotated-out secret.
        let stale = sign_for_test(body, "old", 1_700_000_000);
        let stale_v1 = stale.split(',').find(|s| s.starts_with("v1=")).unwrap();
        let combined = format!("{real},{stale_v1}");
        verify(body, &combined, secret, 1_700_000_000).unwrap();
    }
}
