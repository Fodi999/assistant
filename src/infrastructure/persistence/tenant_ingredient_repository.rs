use crate::domain::catalog::{CatalogIngredientId, Unit};
use crate::shared::{AppError, AppResult, TenantId};
use crate::domain::tenant_ingredient::{TenantIngredient, TenantIngredientId};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

#[async_trait]
pub trait TenantIngredientRepositoryTrait: Send + Sync {
    async fn save(&self, ingredient: &TenantIngredient) -> AppResult<()>;
    async fn find_by_catalog_id(&self, tenant_id: TenantId, catalog_id: CatalogIngredientId) -> AppResult<Option<TenantIngredient>>;
    async fn list_by_tenant(&self, tenant_id: TenantId) -> AppResult<Vec<TenantIngredient>>;
    async fn delete(&self, id: TenantIngredientId, tenant_id: TenantId) -> AppResult<()>;
}

#[derive(Clone)]
pub struct TenantIngredientRepository {
    pool: PgPool,
}

impl TenantIngredientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_tenant_ingredient(row: &sqlx::postgres::PgRow) -> AppResult<TenantIngredient> {
        let custom_unit_str: Option<String> = row.try_get("custom_unit").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let custom_unit = match custom_unit_str {
            Some(s) => Some(Unit::from_str(&s)?),
            None => None,
        };

        Ok(TenantIngredient::from_parts(
            TenantIngredientId::from_uuid(row.try_get("id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?),
            TenantId::from_uuid(row.try_get("tenant_id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?),
            CatalogIngredientId::from_uuid(row.try_get("catalog_ingredient_id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?),
            row.try_get("price").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
            row.try_get("supplier").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
            custom_unit,
            row.try_get("custom_expiration_days").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
            row.try_get("notes").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
            row.try_get("is_active").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
            row.try_get("created_at").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
            row.try_get("updated_at").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
        ))
    }
}

#[async_trait]
impl TenantIngredientRepositoryTrait for TenantIngredientRepository {
    async fn save(&self, ingredient: &TenantIngredient) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO tenant_ingredients (
                id, tenant_id, catalog_ingredient_id, price, 
                supplier, custom_unit, custom_expiration_days, notes, is_active,
                created_at, updated_at
            )
            VALUES (, , , , , ::unit_type, , , , , )
            ON CONFLICT (tenant_id, catalog_ingredient_id) 
            DO UPDATE SET
                price = EXCLUDED.price,
                supplier = EXCLUDED.supplier,
                custom_unit = EXCLUDED.custom_unit,
                custom_expiration_days = EXCLUDED.custom_expiration_days,
                notes = EXCLUDED.notes,
                is_active = EXCLUDED.is_active,
                updated_at = EXCLUDED.updated_at
            "#
        )
        .bind(ingredient.id.as_uuid())
        .bind(ingredient.tenant_id.as_uuid())
        .bind(ingredient.catalog_ingredient_id.as_uuid())
        .bind(ingredient.price)
        .bind(&ingredient.supplier)
        .bind(ingredient.custom_unit.as_ref().map(|u| u.as_str()))
        .bind(ingredient.custom_expiration_days)
        .bind(&ingredient.notes)
        .bind(ingredient.is_active)
        .bind(ingredient.created_at)
        .bind(ingredient.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_catalog_id(&self, tenant_id: TenantId, catalog_id: CatalogIngredientId) -> AppResult<Option<TenantIngredient>> {
        let row = sqlx::query(
            "SELECT * FROM tenant_ingredients WHERE tenant_id =  AND catalog_ingredient_id = "
        )
        .bind(tenant_id.as_uuid())
        .bind(catalog_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(Self::row_to_tenant_ingredient(&r)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(&self, tenant_id: TenantId) -> AppResult<Vec<TenantIngredient>> {
        let rows = sqlx::query(
            "SELECT * FROM tenant_ingredients WHERE tenant_id = "
        )
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(Self::row_to_tenant_ingredient(&row)?);
        }
        Ok(results)
    }

    async fn delete(&self, id: TenantIngredientId, tenant_id: TenantId) -> AppResult<()> {
        sqlx::query("DELETE FROM tenant_ingredients WHERE id =  AND tenant_id = ")
            .bind(id.as_uuid())
            .bind(tenant_id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
