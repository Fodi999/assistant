use crate::application::UserService;
use crate::interfaces::http::middleware::AuthUser;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct UpdateAvatarRequest {
    pub avatar_url: String,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserResponse,
    pub tenant: TenantResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub tenant_id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub language: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

pub async fn me_handler(
    auth_user: AuthUser,
    State(user_service): State<UserService>,
) -> Result<Json<MeResponse>, crate::shared::AppError> {
    let user_with_tenant = user_service.get_user_with_tenant(auth_user.user_id).await?;

    Ok(Json(MeResponse {
        user: UserResponse {
            id: user_with_tenant.user.id.to_string(),
            tenant_id: user_with_tenant.user.tenant_id.to_string(),
            email: user_with_tenant.user.email.to_string(),
            display_name: user_with_tenant.user.display_name.map(|n| n.to_string()),
            avatar_url: user_with_tenant.user.avatar_url,
            role: user_with_tenant.user.role.to_string(),
            language: user_with_tenant.user.language.to_string(),
            created_at: user_with_tenant.user.created_at.to_string(),
        },
        tenant: TenantResponse {
            id: user_with_tenant.tenant.id.to_string(),
            name: user_with_tenant.tenant.name.to_string(),
            created_at: user_with_tenant.tenant.created_at.to_string(),
        },
    }))
}

/// POST /api/profile/avatar/upload-url
pub async fn get_avatar_upload_url(
    auth_user: AuthUser,
    State(user_service): State<UserService>,
) -> Result<Json<crate::application::AvatarUploadResponse>, crate::shared::AppError> {
    let response = user_service.get_avatar_upload_url(auth_user.tenant_id, auth_user.user_id).await?;
    Ok(Json(response))
}

/// PUT /api/profile/avatar
pub async fn update_avatar_url(
    auth_user: AuthUser,
    State(user_service): State<UserService>,
    Json(req): Json<UpdateAvatarRequest>,
) -> Result<axum::http::StatusCode, crate::shared::AppError> {
    user_service.update_avatar_url(auth_user.user_id, req.avatar_url).await?;
    Ok(axum::http::StatusCode::OK)
}
