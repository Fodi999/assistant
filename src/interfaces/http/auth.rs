use crate::application::{AuthService, LoginCommand, RefreshCommand, RegisterCommand};
use crate::shared::Language;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    #[validate(length(min = 1, max = 255))]
    pub restaurant_name: String,
    #[validate(length(min = 1, max = 255))]
    pub owner_name: Option<String>,
    pub language: Option<Language>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RefreshRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub user_id: String,
    pub tenant_id: String,
}

pub async fn register_handler(
    State(auth_service): State<AuthService>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthTokenResponse>, crate::shared::AppError> {
    // Validate request
    req.validate()
        .map_err(|e| crate::shared::AppError::validation(format!("Validation error: {}", e)))?;

    let command = RegisterCommand {
        email: req.email,
        password: req.password,
        restaurant_name: req.restaurant_name,
        owner_name: req.owner_name,
        language: req.language,
    };

    let response = auth_service.register(command).await?;

    Ok(Json(AuthTokenResponse {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        token_type: "Bearer".to_string(),
        user_id: response.user_id.to_string(),
        tenant_id: response.tenant_id.to_string(),
    }))
}

pub async fn login_handler(
    State(auth_service): State<AuthService>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthTokenResponse>, crate::shared::AppError> {
    // Validate request
    req.validate()
        .map_err(|e| crate::shared::AppError::validation(format!("Validation error: {}", e)))?;

    let command = LoginCommand {
        email: req.email,
        password: req.password,
    };

    let response = auth_service.login(command).await?;

    Ok(Json(AuthTokenResponse {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        token_type: "Bearer".to_string(),
        user_id: response.user_id.to_string(),
        tenant_id: response.tenant_id.to_string(),
    }))
}

pub async fn refresh_handler(
    State(auth_service): State<AuthService>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthTokenResponse>, crate::shared::AppError> {
    // Validate request
    req.validate()
        .map_err(|e| crate::shared::AppError::validation(format!("Validation error: {}", e)))?;

    let command = RefreshCommand {
        refresh_token: req.refresh_token,
    };

    let response = auth_service.refresh(command).await?;

    Ok(Json(AuthTokenResponse {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        token_type: "Bearer".to_string(),
        user_id: response.user_id.to_string(),
        tenant_id: response.tenant_id.to_string(),
    }))
}
