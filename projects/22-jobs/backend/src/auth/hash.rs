use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

use crate::error::{AppError, AppResult};

const MIN_LEN: usize = 12;

pub fn hash_password(plaintext: &str) -> AppResult<String> {
    if plaintext.len() < MIN_LEN {
        return Err(AppError::Validation(format!(
            "password must be at least {MIN_LEN} characters"
        )));
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(plaintext.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("argon2: {e}")))?;
    Ok(hash.to_string())
}

pub fn verify_password(plaintext: &str, stored_hash: &str) -> AppResult<bool> {
    let parsed =
        PasswordHash::new(stored_hash).map_err(|e| AppError::Internal(format!("argon2: {e}")))?;
    Ok(Argon2::default()
        .verify_password(plaintext.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_verifies() {
        let h = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &h).unwrap());
        assert!(!verify_password("wrong", &h).unwrap());
    }
}
