//! Password hashing — Argon2id only. Ported from project 14/15.

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
}
