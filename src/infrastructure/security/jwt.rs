//! Access-token signing and verification.
//!
//! The token carries only the identity (`sub`). Business memberships and roles
//! are resolved server-side per request, because one user can be a customer at
//! one business and an employee at another (see PRODUCT_SPEC §9.1, §17).
//! Refresh tokens are opaque random strings; only their hash is stored.

use crate::shared::{AppError, AppResult, UserId};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    access_token_ttl: Duration,
    refresh_token_ttl: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// User id.
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    /// Unique token id (enables future revocation lists).
    pub jti: String,
}

impl AccessTokenClaims {
    pub fn user_id(&self) -> AppResult<UserId> {
        Uuid::parse_str(&self.sub)
            .map(UserId::from_uuid)
            .map_err(|e| AppError::authentication(format!("Invalid user_id in token: {}", e)))
    }
}

impl JwtService {
    pub fn new(
        secret: &str,
        issuer: String,
        audience: String,
        access_token_ttl_minutes: i64,
        refresh_token_ttl_days: i64,
    ) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            issuer,
            audience,
            access_token_ttl: Duration::minutes(access_token_ttl_minutes),
            refresh_token_ttl: Duration::days(refresh_token_ttl_days),
        }
    }

    pub fn generate_access_token(&self, user_id: UserId) -> AppResult<String> {
        let now = OffsetDateTime::now_utc();
        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now.unix_timestamp(),
            exp: (now + self.access_token_ttl).unix_timestamp(),
            jti: Uuid::now_v7().to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to generate access token: {}", e)))
    }

    pub fn verify_access_token(&self, token: &str) -> AppResult<AccessTokenClaims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);

        decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| AppError::authentication(format!("Invalid access token: {}", e)))
    }

    /// Opaque refresh token: 244 bits of OS randomness. Store only its hash.
    pub fn generate_refresh_token(&self) -> String {
        format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
    }

    pub fn refresh_token_ttl(&self) -> Duration {
        self.refresh_token_ttl
    }

    pub fn access_token_ttl_seconds(&self) -> i64 {
        self.access_token_ttl.whole_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> JwtService {
        JwtService::new(
            "test-secret-that-is-long-enough-1234567890",
            "test-issuer".to_string(),
            "test-audience".to_string(),
            15,
            30,
        )
    }

    #[test]
    fn token_roundtrip() {
        let jwt = service();
        let user_id = UserId::new();

        let token = jwt.generate_access_token(user_id).unwrap();
        let claims = jwt.verify_access_token(&token).unwrap();

        assert_eq!(claims.user_id().unwrap(), user_id);
        assert_eq!(claims.iss, "test-issuer");
        assert_eq!(claims.aud, "test-audience");
    }

    #[test]
    fn garbage_token_is_rejected() {
        assert!(service().verify_access_token("invalid-token").is_err());
    }

    #[test]
    fn token_for_another_audience_is_rejected() {
        let jwt = service();
        let other = JwtService::new(
            "test-secret-that-is-long-enough-1234567890",
            "test-issuer".to_string(),
            "another-audience".to_string(),
            15,
            30,
        );
        let token = other.generate_access_token(UserId::new()).unwrap();
        assert!(jwt.verify_access_token(&token).is_err());
    }

    #[test]
    fn refresh_tokens_are_long_and_unique() {
        let jwt = service();
        let a = jwt.generate_refresh_token();
        let b = jwt.generate_refresh_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }
}
