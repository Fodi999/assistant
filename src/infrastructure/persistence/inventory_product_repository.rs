use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{InventoryProduct, InventoryProductId, Money, Quantity},
};
use crate::shared::{AppResult, TenantId, UserId};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;

#[async_trait]
pub trait InventoryProductRepositoryTrait: Send + Sync {
    /// Add new product to inventory
    async fn create(&self, product: &InventoryProduct) -> AppResult<()>;
    
    /// Find product by ID
    async fn find_by_id(&self, id: InventoryProductId, user_id: UserId, tenant_id: TenantId) -> AppResult<Option<InventoryProduct>>;
    
    /// List all products for user
    async fn list_by_user(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<Vec<InventoryProduct>>;
    
    /// Update product quantity and price
    async fn update(&self, product: &InventoryProduct) -> AppResult<()>;
    
    /// Delete product from inventory
    async fn delete(&self, id: InventoryProductId, user_id: UserId, tenant_id: TenantId) -> AppResult<()>;
    
    /// Count products for user (to check if inventory is not empty)
    async fn count_by_user(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<i64>;
}

#[derive(Clone)]
pub struct InventoryProductRepository {
    pool: PgPool,
}

impl InventoryProductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_product(row: &sqlx::postgres::PgRow) -> AppResult<InventoryProduct> {
        let id: uuid::Uuid = row.try_get("id")?;
        let user_id: uuid::Uuid = row.try_get("user_id")?;
        let tenant_id: uuid::Uuid = row.try_get("tenant_id")?;
        let catalog_ingredient_id: uuid::Uuid = row.try_get("catalog_ingredient_id")?;
        let price_cents: i64 = row.try_get("price_per_unit_cents")?;
        let quantity: f64 = row.try_get("quantity")?;
        let received_at: OffsetDateTime = row.try_get("received_at")?;
        let expires_at: Option<OffsetDateTime> = row.try_get("expires_at")?;
        let created_at: OffsetDateTime = row.try_get("created_at")?;
        let updated_at: OffsetDateTime = row.try_get("updated_at")?;

        Ok(InventoryProduct::from_parts(
            InventoryProductId::from_uuid(id),
            UserId::from_uuid(user_id),
            TenantId::from_uuid(tenant_id),
            CatalogIngredientId::from_uuid(catalog_ingredient_id),
            Money::from_cents(price_cents)?,
            Quantity::new(quantity)?,
            received_at,
            expires_at,
            created_at,
            updated_at,
        ))
    }
}

#[async_trait]
impl InventoryProductRepositoryTrait for InventoryProductRepository {
    async fn create(&self, product: &InventoryProduct) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO inventory_products 
                (id, user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, quantity, received_at, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(product.id.as_uuid())
        .bind(product.user_id.as_uuid())
        .bind(product.tenant_id.as_uuid())
        .bind(product.catalog_ingredient_id.as_uuid())
        .bind(product.price_per_unit.as_cents())
        .bind(product.quantity.value())
        .bind(product.received_at)
        .bind(product.expires_at)
        .bind(product.created_at)
        .bind(product.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: InventoryProductId, user_id: UserId, tenant_id: TenantId) -> AppResult<Option<InventoryProduct>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, quantity, received_at, expires_at, created_at, updated_at
            FROM inventory_products
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            "#
        )
        .bind(id.as_uuid())
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(ref r) => Ok(Some(Self::row_to_product(r)?)),
            None => Ok(None),
        }
    }

    async fn list_by_user(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<Vec<InventoryProduct>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, quantity, received_at, expires_at, created_at, updated_at
            FROM inventory_products
            WHERE user_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            "#
        )
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::row_to_product)
            .collect()
    }

    async fn update(&self, product: &InventoryProduct) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE inventory_products
            SET price_per_unit_cents = $1, quantity = $2, expires_at = $3, updated_at = $4
            WHERE id = $5 AND user_id = $6 AND tenant_id = $7
            "#
        )
        .bind(product.price_per_unit.as_cents())
        .bind(product.quantity.value())
        .bind(product.expires_at)
        .bind(product.updated_at)
        .bind(product.id.as_uuid())
        .bind(product.user_id.as_uuid())
        .bind(product.tenant_id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: InventoryProductId, user_id: UserId, tenant_id: TenantId) -> AppResult<()> {
        sqlx::query(
            r#"
            DELETE FROM inventory_products
            WHERE id = $1 AND user_id = $2 AND tenant_id = $3
            "#
        )
        .bind(id.as_uuid())
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn count_by_user(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<i64> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM inventory_products
            WHERE user_id = $1 AND tenant_id = $2
            "#
        )
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count)
    }
}
