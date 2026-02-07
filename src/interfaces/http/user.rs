use crate::application::UserService;
use crate::interfaces::http::middleware::AuthUser;
use axum::{extract::State, Json};
use serde::Serialize;

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
