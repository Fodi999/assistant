use crate::shared::{AppError, AppResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher as Argon2PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

#[derive(Clone)]
pub struct PasswordHasher {
    argon2: Argon2<'static>,
}

impl PasswordHasher {
    pub fn new() -> Self {
        Self {
            argon2: Argon2::default(),
        }
    }

    pub fn hash_password(&self, password: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        
        let password_hash = Argon2PasswordHasher::hash_password(&self.argon2, password.as_bytes(), &salt)
            .map_err(|e| AppError::internal(format!("Failed to hash password: {}", e)))?;

        Ok(password_hash.to_string())
    }

    pub fn verify_password(&self, password: &str, password_hash: &str) -> AppResult<bool> {
        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|e| AppError::internal(format!("Failed to parse password hash: {}", e)))?;

        Ok(PasswordVerifier::verify_password(&self.argon2, password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let hasher = PasswordHasher::new();
        let password = "SecurePassword123!";

        let hash = hasher.hash_password(password).unwrap();
        assert_ne!(hash, password);

        assert!(hasher.verify_password(password, &hash).unwrap());
        assert!(!hasher.verify_password("WrongPassword", &hash).unwrap());
    }
}
