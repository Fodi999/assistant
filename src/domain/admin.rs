use serde::{Deserialize, Serialize};

/// JWT Claims для Super Admin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminClaims {
    /// Subject (admin email)
    pub sub: String,
    /// Role (всегда "super_admin")
    pub role: String,
    /// Expiration time (timestamp)
    pub exp: usize,
    /// Issued at (timestamp)
    pub iat: usize,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct AdminLoginResponse {
    pub token: String,
    pub expires_in: usize, // seconds
}
