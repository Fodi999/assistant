use crate::domain::{Tenant, User};
use crate::infrastructure::{TenantRepository, TenantRepositoryTrait, UserRepository, UserRepositoryTrait, R2Client};
use crate::shared::{AppError, AppResult, UserId, TenantId};
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct UserService {
    user_repo: UserRepository,
    tenant_repo: TenantRepository,
    r2_client: Option<R2Client>,
}

impl UserService {
    pub fn new(user_repo: UserRepository, tenant_repo: TenantRepository, r2_client: Option<R2Client>) -> Self {
        Self {
            user_repo,
            tenant_repo,
            r2_client,
        }
    }

    pub async fn get_user_with_tenant(&self, user_id: UserId) -> AppResult<UserWithTenant> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        let tenant = self
            .tenant_repo
            .find_by_id(user.tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Tenant not found"))?;

        Ok(UserWithTenant { user, tenant })
    }

    pub async fn get_avatar_upload_url(&self, tenant_id: TenantId, user_id: UserId, content_type: &str) -> AppResult<AvatarUploadResponse> {
        let r2 = self.r2_client.as_ref().ok_or_else(|| AppError::internal("R2 client not configured"))?;
        
        let ext = if content_type.contains("jpeg") || content_type.contains("jpg") {
            "jpg"
        } else if content_type.contains("png") {
            "png"
        } else {
            "webp"
        };
        
        let key = format!("avatars/{}/{}.{}", tenant_id.as_uuid(), user_id.as_uuid(), ext);
        
        let upload_url = r2.generate_presigned_upload_url(&key, content_type).await?;
        let public_url = r2.get_public_url(&key);

        Ok(AvatarUploadResponse {
            upload_url,
            public_url,
        })
    }

    pub async fn update_avatar_url(&self, user_id: UserId, avatar_url: String) -> AppResult<()> {
        self.user_repo.update_avatar_url(user_id, &avatar_url).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvatarUploadResponse {
    pub upload_url: String,
    pub public_url: String,
}

#[derive(Debug, Serialize)]
pub struct UserWithTenant {
    pub user: User,
    pub tenant: Tenant,
}
