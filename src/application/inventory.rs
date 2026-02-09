use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{ExpirationStatus, InventoryProduct, InventoryProductId, Money, Quantity},
};
use crate::infrastructure::persistence::{InventoryProductRepository, InventoryProductRepositoryTrait};
use crate::shared::{AppResult, Language, TenantId, UserId};
use serde::Serialize;
use sqlx::{PgPool, Row};
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
    pool: PgPool,
}

impl InventoryService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inventory_repo: Arc::new(InventoryProductRepository::new(pool.clone())),
            pool,
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

    /// Get inventory view with joined catalog ingredient and category data (Query DTO pattern)
    /// Returns rich view with product details for frontend - no additional queries needed
    /// Uses proper i18n with translations table + COALESCE fallback to English (BEST PRACTICE)
    /// 🎯 ЭТАЛОН для B2B SaaS: язык из user.language, fallback на 'en'
    pub async fn list_products_with_details(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        language: Language,
    ) -> AppResult<Vec<InventoryView>> {
        let lang_code = language.code();

        // 🎯 ЭТАЛОННЫЙ SQL с COALESCE fallback (production-level)
        // Если нет перевода на язык пользователя - берем английский
        let query = r#"
            SELECT 
                ip.id,
                ip.catalog_ingredient_id,
                COALESCE(cit_user.name, cit_en.name, 'Unknown') as ingredient_name,
                COALESCE(cct_user.name, cct_en.name, 'Unknown') as category_name,
                ci.default_unit::TEXT as base_unit,
                ip.quantity,
                ip.price_per_unit_cents,
                ip.expires_at,
                ip.created_at,
                ip.updated_at
            FROM inventory_products ip
            INNER JOIN catalog_ingredients ci 
                ON ip.catalog_ingredient_id = ci.id
            LEFT JOIN catalog_ingredient_translations cit_user 
                ON cit_user.ingredient_id = ci.id AND cit_user.language = $3
            LEFT JOIN catalog_ingredient_translations cit_en 
                ON cit_en.ingredient_id = ci.id AND cit_en.language = 'en'
            LEFT JOIN catalog_categories cc 
                ON ci.category_id = cc.id
            LEFT JOIN catalog_category_translations cct_user 
                ON cct_user.category_id = cc.id AND cct_user.language = $3
            LEFT JOIN catalog_category_translations cct_en 
                ON cct_en.category_id = cc.id AND cct_en.language = 'en'
            WHERE ip.user_id = $1 AND ip.tenant_id = $2
            ORDER BY ip.created_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(user_id.as_uuid())
            .bind(tenant_id.as_uuid())
            .bind(lang_code)  // user.language ('en'|'pl'|'uk'|'ru')
            .fetch_all(&self.pool)
            .await?;

        let mut views = Vec::new();
        for row in rows {
            views.push(InventoryView {
                id: row.get("id"),
                product: ProductInfo {
                    id: row.get("catalog_ingredient_id"),
                    name: row.get("ingredient_name"),
                    category: row.get("category_name"),
                    base_unit: row.get("base_unit"),
                },
                quantity: row.get("quantity"),
                price_per_unit_cents: row.get("price_per_unit_cents"),
                expires_at: row.get("expires_at"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(views)
    }
}

/// Rich inventory view DTO (returned from query with JOINs)
#[derive(Debug, Clone, Serialize)]
pub struct InventoryView {
    pub id: uuid::Uuid,
    pub product: ProductInfo,
    pub quantity: f64,
    pub price_per_unit_cents: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub category: String,
    pub base_unit: String,
}
