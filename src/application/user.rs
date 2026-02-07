use crate::domain::{Tenant, User};
use crate::infrastructure::{TenantRepository, TenantRepositoryTrait, UserRepository, UserRepositoryTrait};
use crate::shared::{AppError, AppResult, UserId};
use serde::Serialize;

#[derive(Clone)]
pub struct UserService {
    user_repo: UserRepository,
    tenant_repo: TenantRepository,
}

impl UserService {
    pub fn new(user_repo: UserRepository, tenant_repo: TenantRepository) -> Self {
        Self {
            user_repo,
            tenant_repo,
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
}

#[derive(Debug, Serialize)]
pub struct UserWithTenant {
    pub user: User,
    pub tenant: Tenant,
}
