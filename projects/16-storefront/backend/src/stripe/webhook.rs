//! Hand-rolled webhook signature verification.
//!
//! The Stripe signature header looks like:
//!
//!     Stripe-Signature: t=1614265330,v1=<hex>,v1=<hex>,v0=<legacy>
//!
//! Per Stripe's docs we:
//!   1) Parse the header into `t` and ALL `v1=` schemes.
//!   2) Compute HMAC-SHA256(secret, "{t}.{raw_body}").
//!   3) Constant-time compare the computed digest against each `v1=` value.
//!   4) Reject if the |now - t| window exceeds 5 minutes (replay protection).
//!
//! Note that we hash with `secret` as-is (the bytes from STRIPE_WEBHOOK_SECRET).
//! Stripe documents their `whsec_...` secret value, but you pass that string
//! verbatim to the HMAC; there's no decoding step.

use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Maximum allowed difference between header timestamp and now, in seconds.
pub const REPLAY_WINDOW_SECS: i64 = 300;

#[derive(Debug, PartialEq, Eq)]
pub enum SigError {
    /// Header missing or malformed (no `t=` and at least one `v1=`).
    MalformedHeader,
    /// Timestamp in header is outside the replay window.
    TimestampSkewed,
    /// HMAC of `{t}.{body}` doesn't match any v1 scheme.
    SignatureMismatch,
}

/// Verify a webhook against the raw body bytes and the configured secret.
///
/// `now_unix` is injectable so tests can pin time without a clock.
pub fn verify(
    secret: &[u8],
    header_value: &str,
    body: &[u8],
    now_unix: i64,
) -> Result<(), SigError> {
    let (ts, sigs) = parse_header(header_value).ok_or(SigError::MalformedHeader)?;
    if sigs.is_empty() {
        return Err(SigError::MalformedHeader);
    }

    if (now_unix - ts).abs() > REPLAY_WINDOW_SECS {
        return Err(SigError::TimestampSkewed);
    }

    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length, including empty");
    mac.update(ts.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    let expected = mac.finalize().into_bytes();

    for sig_hex in sigs {
        // hex::decode is constant-time-friendly (length-prefix), but the
        // memcmp it would do is NOT — so we explicitly use subtle::ct_eq.
        if let Ok(decoded) = hex::decode(sig_hex)
            && bool::from(expected.ct_eq(&decoded))
        {
            return Ok(());
        }
    }
    Err(SigError::SignatureMismatch)
}

/// Parse `t=...,v1=...,v1=...,v0=...` into (timestamp, [v1 sigs]).
/// Returns `None` if `t=` is missing or unparsable.
fn parse_header(header: &str) -> Option<(i64, Vec<&str>)> {
    let mut ts: Option<i64> = None;
    let mut v1: Vec<&str> = Vec::new();
    for raw in header.split(',') {
        let part = raw.trim();
        if let Some(v) = part.strip_prefix("t=") {
            ts = v.parse::<i64>().ok();
        } else if let Some(v) = part.strip_prefix("v1=") {
            v1.push(v);
        }
        // ignore v0= and any future schemes
    }
    ts.map(|t| (t, v1))
}

/// Helper for the route: now-as-unix-seconds in one place so tests can override.
pub fn now_unix() -> i64 {
    Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::Mac as _;

    fn sign(secret: &[u8], ts: i64, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(ts.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn round_trip_ok() {
        let secret = b"whsec_test";
        let body = br#"{"id":"evt_1","type":"checkout.session.completed"}"#;
        let ts: i64 = 1_700_000_000;
        let sig = sign(secret, ts, body);
        let header = format!("t={ts},v1={sig}");
        assert!(verify(secret, &header, body, ts).is_ok());
    }

    #[test]
    fn wrong_secret_rejected() {
        let body = b"{}";
        let ts: i64 = 1_700_000_000;
        let sig = sign(b"correct", ts, body);
        let header = format!("t={ts},v1={sig}");
        assert_eq!(
            verify(b"wrong", &header, body, ts),
            Err(SigError::SignatureMismatch)
        );
    }

    #[test]
    fn tampered_body_rejected() {
        let secret = b"k";
        let body = b"{\"a\":1}";
        let ts: i64 = 1_700_000_000;
        let sig = sign(secret, ts, body);
        let header = format!("t={ts},v1={sig}");
        let tampered = b"{\"a\":2}";
        assert_eq!(
            verify(secret, &header, tampered, ts),
            Err(SigError::SignatureMismatch)
        );
    }

    #[test]
    fn old_timestamp_rejected() {
        let secret = b"k";
        let body = b"{}";
        let ts: i64 = 1_700_000_000;
        let sig = sign(secret, ts, body);
        let header = format!("t={ts},v1={sig}");
        // 10 minutes later → outside the 5-minute replay window
        assert_eq!(
            verify(secret, &header, body, ts + 600),
            Err(SigError::TimestampSkewed)
        );
    }

    #[test]
    fn future_timestamp_rejected() {
        let secret = b"k";
        let body = b"{}";
        let ts: i64 = 1_700_000_000;
        let sig = sign(secret, ts, body);
        let header = format!("t={ts},v1={sig}");
        // 10 minutes earlier → also outside window
        assert_eq!(
            verify(secret, &header, body, ts - 600),
            Err(SigError::TimestampSkewed)
        );
    }

    #[test]
    fn missing_t_is_malformed() {
        let header = "v1=abc";
        assert_eq!(
            verify(b"k", header, b"{}", 0),
            Err(SigError::MalformedHeader)
        );
    }

    #[test]
    fn missing_v1_is_malformed() {
        let header = "t=1700000000";
        assert_eq!(
            verify(b"k", header, b"{}", 1_700_000_000),
            Err(SigError::MalformedHeader)
        );
    }

    #[test]
    fn ignores_v0_scheme() {
        let secret = b"k";
        let body = b"{}";
        let ts: i64 = 1_700_000_000;
        let sig = sign(secret, ts, body);
        // v0 is legacy / different algorithm. We must still accept the v1.
        let header = format!("t={ts},v0=legacy_value,v1={sig}");
        assert!(verify(secret, &header, body, ts).is_ok());
    }

    #[test]
    fn accepts_any_of_multiple_v1() {
        let secret = b"k";
        let body = b"{}";
        let ts: i64 = 1_700_000_000;
        let sig = sign(secret, ts, body);
        // Stripe sometimes sends multiple v1 sigs during secret rotation.
        let header = format!("t={ts},v1=00ff00ff,v1={sig}");
        assert!(verify(secret, &header, body, ts).is_ok());
    }
}
