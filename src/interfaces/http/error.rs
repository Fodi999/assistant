use crate::shared::AppError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            AppError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                "Validation failed",
                Some(msg.clone()),
            ),
            AppError::Authentication(msg) => (
                StatusCode::UNAUTHORIZED,
                "AUTHENTICATION_ERROR",
                "Authentication failed",
                Some(msg.clone()),
            ),
            AppError::Authorization(msg) => (
                StatusCode::FORBIDDEN,
                "AUTHORIZATION_ERROR",
                "Authorization failed",
                Some(msg.clone()),
            ),
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Resource not found",
                Some(msg.clone()),
            ),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                "CONFLICT",
                "Conflict",
                Some(msg.clone()),
            ),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Internal server error",
                    None,
                )
            }
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "Database error occurred",
                    None,
                )
            }
            AppError::Jwt(e) => {
                tracing::error!("JWT error: {}", e);
                (
                    StatusCode::UNAUTHORIZED,
                    "JWT_ERROR",
                    "Invalid token",
                    None,
                )
            }
        };

        let error_response = ErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            details,
        };

        (status, Json(error_response)).into_response()
    }
}
