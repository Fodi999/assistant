pub mod assistant_state_repository;
pub mod catalog_category_repository;
pub mod catalog_ingredient_repository;
pub mod refresh_token_repository;
pub mod tenant_repository;
pub mod user_repository;

pub use assistant_state_repository::*;
pub use catalog_category_repository::*;
pub use catalog_ingredient_repository::*;
pub use refresh_token_repository::*;
pub use tenant_repository::*;
pub use user_repository::*;

use sqlx::PgPool;

#[derive(Clone)]
pub struct Repositories {
    pub tenant: TenantRepository,
    pub user: UserRepository,
    pub refresh_token: RefreshTokenRepository,
    pub assistant_state: AssistantStateRepository,
}

impl Repositories {
    pub fn new(pool: PgPool) -> Self {
        Self {
            tenant: TenantRepository::new(pool.clone()),
            user: UserRepository::new(pool.clone()),
            refresh_token: RefreshTokenRepository::new(pool.clone()),
            assistant_state: AssistantStateRepository::new(pool),
        }
    }
}
