//! Password hashing — Argon2id only. (Project 12's dual-verify lesson
//! is behind us; from here on we assume an all-Argon2 user base.)
//!
//! Argon2id parameters per OWASP 2025 for interactive logins:
//!     m_cost = 19_456 KiB (~19 MiB), t_cost = 2, p_cost = 1, output = 32 bytes

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use rand::rngs::OsRng;

use crate::error::{AppError, AppResult};

fn params() -> Params {
    Params::new(19_456, 2, 1, Some(32)).expect("argon2 params valid")
}

pub fn hash_password(password: &str) -> AppResult<String> {
    if password.chars().count() < 12 {
        return Err(AppError::Validation(
            "password must be at least 12 characters".into(),
        ));
    }
    if password.chars().count() > 256 {
        return Err(AppError::Validation(
            "password must be 256 characters or fewer".into(),
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params());
    let phc = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("argon2 hash failed: {e}")))?;
    Ok(phc.to_string())
}

/// Constant-time verify. Returns Ok(true) on match, Ok(false) on mismatch.
/// A malformed hash (parse failure) is an internal error, not a mismatch —
/// we'd rather error loudly than silently treat a corrupted hash as "no".
pub fn verify_password(password: &str, phc: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(phc)
        .map_err(|e| AppError::Internal(format!("argon2 hash parse failed: {e}")))?;
    let argon2 = Argon2::default();
    match argon2.verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(AppError::Internal(format!("argon2 verify error: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_verifies() {
        let phc = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &phc).unwrap());
        assert!(!verify_password("wrong password attempt!!!", &phc).unwrap());
    }

    #[test]
    fn rejects_short_password() {
        assert!(hash_password("short").is_err());
    }

    #[test]
    fn rejects_long_password() {
        let p = "x".repeat(257);
        assert!(hash_password(&p).is_err());
    }

    #[test]
    fn malformed_phc_is_internal_error() {
        let r = verify_password("anything", "not-a-real-phc-string");
        assert!(matches!(r, Err(AppError::Internal(_))));
    }
}
