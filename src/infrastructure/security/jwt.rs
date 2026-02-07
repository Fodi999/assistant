use crate::shared::{AppError, AppResult, TenantId, UserId};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    access_token_ttl: Duration,
    refresh_token_ttl: Duration,
}

impl JwtService {
    pub fn new(
        secret: String,
        issuer: String,
        access_token_ttl_minutes: i64,
        refresh_token_ttl_days: i64,
    ) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            issuer,
            access_token_ttl: Duration::minutes(access_token_ttl_minutes),
            refresh_token_ttl: Duration::days(refresh_token_ttl_days),
        }
    }

    pub fn generate_access_token(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<String> {
        let now = OffsetDateTime::now_utc();
        let expires_at = now + self.access_token_ttl;

        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            iss: self.issuer.clone(),
            iat: now.unix_timestamp(),
            exp: expires_at.unix_timestamp(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to generate access token: {}", e)))
    }

    pub fn generate_refresh_token(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn get_refresh_token_ttl(&self) -> Duration {
        self.refresh_token_ttl
    }

    pub fn verify_access_token(&self, token: &str) -> AppResult<AccessTokenClaims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);

        let token_data = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| AppError::authentication(format!("Invalid access token: {}", e)))?;

        Ok(token_data.claims)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,          // user_id
    pub tenant_id: String,    // tenant_id
    pub iss: String,          // issuer
    pub iat: i64,             // issued at
    pub exp: i64,             // expiration
}

impl AccessTokenClaims {
    pub fn user_id(&self) -> AppResult<UserId> {
        Uuid::parse_str(&self.sub)
            .map(UserId::from_uuid)
            .map_err(|e| AppError::authentication(format!("Invalid user_id in token: {}", e)))
    }

    pub fn tenant_id(&self) -> AppResult<TenantId> {
        Uuid::parse_str(&self.tenant_id)
            .map(TenantId::from_uuid)
            .map_err(|e| AppError::authentication(format!("Invalid tenant_id in token: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_verification() {
        let jwt_service = JwtService::new(
            "test-secret".to_string(),
            "test-issuer".to_string(),
            15,
            30,
        );

        let user_id = UserId::new();
        let tenant_id = TenantId::new();

        let token = jwt_service
            .generate_access_token(user_id, tenant_id)
            .unwrap();

        let claims = jwt_service.verify_access_token(&token).unwrap();

        assert_eq!(claims.user_id().unwrap(), user_id);
        assert_eq!(claims.tenant_id().unwrap(), tenant_id);
        assert_eq!(claims.iss, "test-issuer");
    }

    #[test]
    fn test_invalid_token() {
        let jwt_service = JwtService::new(
            "test-secret".to_string(),
            "test-issuer".to_string(),
            15,
            30,
        );

        let result = jwt_service.verify_access_token("invalid-token");
        assert!(result.is_err());
    }
}
