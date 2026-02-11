use crate::domain::{AdminClaims, AdminLoginRequest, AdminLoginResponse};
use crate::shared::AppError;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use jsonwebtoken::{encode, EncodingKey, Header};

/// Super Admin Authentication Service
#[derive(Clone)]
pub struct AdminAuthService {
    admin_email: String,
    admin_password_hash: String,
    jwt_secret: String,
    token_ttl_hours: usize,
}

impl AdminAuthService {
    pub fn new(
        admin_email: String,
        admin_password_hash: String,
        jwt_secret: String,
        token_ttl_hours: usize,
    ) -> Self {
        Self {
            admin_email,
            admin_password_hash,
            jwt_secret,
            token_ttl_hours,
        }
    }

    /// Authenticate admin and generate JWT token
    pub fn login(&self, req: AdminLoginRequest) -> Result<AdminLoginResponse, AppError> {
        // Check email
        if req.email != self.admin_email {
            return Err(AppError::authentication("Invalid credentials"));
        }

        // Verify password with Argon2
        let parsed_hash = PasswordHash::new(&self.admin_password_hash)
            .map_err(|_| AppError::internal("Invalid password hash configuration"))?;

        Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::authentication("Invalid credentials"))?;

        // Generate JWT
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let exp = now + (self.token_ttl_hours * 3600);

        let claims = AdminClaims {
            sub: self.admin_email.clone(),
            role: "super_admin".to_string(),
            exp,
            iat: now,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::internal(format!("Failed to generate token: {}", e)))?;

        Ok(AdminLoginResponse {
            token,
            expires_in: self.token_ttl_hours * 3600,
        })
    }

    /// Verify JWT token and extract claims
    pub fn verify_token(&self, token: &str) -> Result<AdminClaims, AppError> {
        let token_data = jsonwebtoken::decode::<AdminClaims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &jsonwebtoken::Validation::default(),
        )
        .map_err(|_| AppError::authentication("Invalid or expired token"))?;

        // Verify role
        if token_data.claims.role != "super_admin" {
            return Err(AppError::authorization("Insufficient permissions"));
        }

        Ok(token_data.claims)
    }
}
