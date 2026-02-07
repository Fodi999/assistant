use crate::domain::{Tenant, TenantName};
use crate::shared::{AppResult, TenantId};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[async_trait]
pub trait TenantRepositoryTrait: Send + Sync {
    async fn create(&self, tenant: &Tenant) -> AppResult<()>;
    async fn find_by_id(&self, id: TenantId) -> AppResult<Option<Tenant>>;
}

#[derive(Clone)]
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantRepositoryTrait for TenantRepository {
    async fn create(&self, tenant: &Tenant) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, created_at)
            VALUES ($1, $2, $3)
            "#
        )
        .bind(tenant.id.as_uuid())
        .bind(tenant.name.as_str())
        .bind(tenant.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: TenantId) -> AppResult<Option<Tenant>> {
        let result = sqlx::query(
            r#"
            SELECT id, name, created_at
            FROM tenants
            WHERE id = $1
            "#
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| {
            let id: Uuid = row.get("id");
            let name: String = row.get("name");
            let created_at: OffsetDateTime = row.get("created_at");

            Tenant::from_parts(
                TenantId::from_uuid(id),
                TenantName::new(name).unwrap(),
                created_at,
            )
        }))
    }
}
