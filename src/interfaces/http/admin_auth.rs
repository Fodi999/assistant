use crate::application::AdminAuthService;
use crate::domain::AdminLoginRequest;
use crate::shared::AppError;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

/// POST /api/admin/auth/login - Super Admin login
pub async fn login(
    State(service): State<AdminAuthService>,
    Json(req): Json<AdminLoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = service.login(req)?;
    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/admin/auth/verify - Verify token (protected route example)
pub async fn verify(
    _claims: crate::domain::AdminClaims, // Middleware adds this
) -> Result<impl IntoResponse, AppError> {
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Token is valid",
        "role": "super_admin"
    }))))
}
