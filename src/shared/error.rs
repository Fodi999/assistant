use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error")]
    Internal(String),

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("JWT error")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    pub fn authorization(msg: impl Into<String>) -> Self {
        Self::Authorization(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}
