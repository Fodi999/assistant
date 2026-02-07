use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{ExpirationStatus, InventoryProduct, InventoryProductId, Money, Quantity},
};
use crate::infrastructure::persistence::{InventoryProductRepository, InventoryProductRepositoryTrait};
use crate::shared::{AppResult, TenantId, UserId};
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;
use time::OffsetDateTime;

/// Aggregated inventory status for assistant
#[derive(Debug, Clone, Serialize)]
pub struct InventoryStatus {
    pub total_products: usize,
    pub expired: usize,
    pub expiring_today: usize,
    pub expiring_soon: usize,
    pub fresh: usize,
}

impl InventoryStatus {
    pub fn has_warnings(&self) -> bool {
        self.expired > 0 || self.expiring_today > 0 || self.expiring_soon > 0
    }

    pub fn has_critical(&self) -> bool {
        self.expired > 0 || self.expiring_today > 0
    }
}

#[derive(Clone)]
pub struct InventoryService {
    inventory_repo: Arc<InventoryProductRepository>,
}

impl InventoryService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inventory_repo: Arc::new(InventoryProductRepository::new(pool)),
        }
    }

    /// Add product to inventory
    pub async fn add_product(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price_per_unit_cents: i64,
        quantity: f64,
        expires_at: Option<OffsetDateTime>,
    ) -> AppResult<InventoryProductId> {
        let price = Money::from_cents(price_per_unit_cents)?;
        let qty = Quantity::new(quantity)?;

        let product = InventoryProduct::new(
            user_id,
            tenant_id,
            catalog_ingredient_id,
            price,
            qty,
            expires_at,
        );

        let product_id = product.id;
        self.inventory_repo.create(&product).await?;

        Ok(product_id)
    }

    /// Get user's inventory list
    pub async fn list_products(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<Vec<InventoryProduct>> {
        self.inventory_repo.list_by_user(user_id, tenant_id).await
    }

    /// Update product quantity and price
    pub async fn update_product(
        &self,
        product_id: InventoryProductId,
        user_id: UserId,
        tenant_id: TenantId,
        price_per_unit_cents: Option<i64>,
        quantity: Option<f64>,
    ) -> AppResult<()> {
        let mut product = self
            .inventory_repo
            .find_by_id(product_id, user_id, tenant_id)
            .await?
            .ok_or_else(|| crate::shared::AppError::not_found("Product not found"))?;

        if let Some(price_cents) = price_per_unit_cents {
            let new_price = Money::from_cents(price_cents)?;
            product.update_price(new_price);
        }

        if let Some(qty) = quantity {
            let new_quantity = Quantity::new(qty)?;
            product.update_quantity(new_quantity);
        }

        self.inventory_repo.update(&product).await?;
        Ok(())
    }

    /// Delete product from inventory
    pub async fn delete_product(
        &self,
        product_id: InventoryProductId,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<()> {
        self.inventory_repo.delete(product_id, user_id, tenant_id).await
    }

    /// Check if user has any products in inventory
    pub async fn has_products(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<bool> {
        let count = self.inventory_repo.count_by_user(user_id, tenant_id).await?;
        Ok(count > 0)
    }

    /// Count products in inventory
    pub async fn count_products(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<i64> {
        self.inventory_repo.count_by_user(user_id, tenant_id).await
    }

    /// Get aggregated inventory status for assistant
    pub async fn get_status(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<InventoryStatus> {
        let products = self.inventory_repo.list_by_user(user_id, tenant_id).await?;
        
        let mut expired = 0;
        let mut expiring_today = 0;
        let mut expiring_soon = 0;
        let mut fresh = 0;

        for product in &products {
            match product.expiration_status() {
                ExpirationStatus::Expired => expired += 1,
                ExpirationStatus::ExpiresToday => expiring_today += 1,
                ExpirationStatus::ExpiringSoon => expiring_soon += 1,
                ExpirationStatus::Fresh | ExpirationStatus::NoExpiration => fresh += 1,
            }
        }

        Ok(InventoryStatus {
            total_products: products.len(),
            expired,
            expiring_today,
            expiring_soon,
            fresh,
        })
    }
}
