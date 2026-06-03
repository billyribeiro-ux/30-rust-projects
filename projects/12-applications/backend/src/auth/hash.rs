//! Password hashing — Argon2id for new hashes, bcrypt for the legacy
//! migration path.
//!
//! Argon2id parameters per OWASP 2025 for interactive logins:
//!     m_cost = 19_456 KiB (~19 MiB), t_cost = 2, p_cost = 1, output = 32 bytes
//!
//! Project 12's headline lesson is the **dual-verify pattern**. When an
//! acquired company hands us their bcrypt password table, we import their
//! hashes into `users.legacy_bcrypt_hash`. On login:
//!   1. Try Argon2 against `password_hash` (modern path).
//!   2. If absent OR mismatched, try bcrypt against `legacy_bcrypt_hash`.
//!   3. On a successful bcrypt verify, RE-HASH the password as Argon2,
//!      atomically update `password_hash` + clear `legacy_bcrypt_hash`.
//!
//! The legacy column drains naturally as users log in. Nobody is forced
//! through a password reset.

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use argon2::password_hash::rand_core::OsRng;

use crate::error::{AppError, AppResult};

fn params() -> Params {
    // m_cost in KiB; t_cost iterations; p_cost lanes; output_len = 32 bytes
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

// ---------- bcrypt (legacy) ----------

/// Verify a bcrypt hash. `bcrypt` is what most pre-2020 Rails / PHP /
/// Node shops used (and many still do). It's slow enough to be a real
/// defense, but not Argon2-good — its memory cost is fixed low so GPU
/// attacks scale linearly. We accept it for backwards compatibility and
/// upgrade on the next successful login (see `routes::auth::login`).
pub fn verify_bcrypt(password: &str, bcrypt_hash: &str) -> AppResult<bool> {
    bcrypt::verify(password, bcrypt_hash)
        .map_err(|e| AppError::Internal(format!("bcrypt verify failed: {e}")))
}

/// Cost factor 12 — the default for the `bcrypt` crate, matches what most
/// production systems use. Used ONLY by tests that need a sample bcrypt hash.
#[cfg(test)]
pub fn hash_bcrypt(password: &str) -> AppResult<String> {
    bcrypt::hash(password, 12).map_err(|e| AppError::Internal(format!("bcrypt hash failed: {e}")))
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
    fn bcrypt_round_trip_verifies() {
        let h = hash_bcrypt("correct horse battery staple").unwrap();
        assert!(verify_bcrypt("correct horse battery staple", &h).unwrap());
        assert!(!verify_bcrypt("wrong attempt", &h).unwrap());
    }

    #[test]
    fn bcrypt_legacy_hash_format() {
        // Real-world example: a hash exported from a Rails has_secure_password
        // table. Cost 12, generated with BCrypt::Password.create("hello world").
        // The "$2b$" prefix is the modern bcrypt identifier (rust bcrypt crate
        // also accepts $2a$ and $2y$ legacy variants).
        let h = hash_bcrypt("legacy_password_imported_from_rails").unwrap();
        assert!(h.starts_with("$2b$") || h.starts_with("$2a$") || h.starts_with("$2y$"));
        assert!(verify_bcrypt("legacy_password_imported_from_rails", &h).unwrap());
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
