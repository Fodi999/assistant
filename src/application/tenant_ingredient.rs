use crate::domain::{CatalogIngredientId, TenantId, TenantIngredient, TenantIngredientId, Unit};
use crate::shared::{AppError, AppResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Tenant Ingredient Service
/// Manages user-specific ingredient data (prices, suppliers, etc.)
#[derive(Clone)]
pub struct TenantIngredientService {
    pool: PgPool,
}

/// Create Tenant Ingredient Request
#[derive(Debug, Deserialize)]
pub struct AddTenantIngredientRequest {
    pub catalog_ingredient_id: Uuid,
    pub price: Option<Decimal>,
    pub supplier: Option<String>,
    pub custom_unit: Option<String>,
    pub custom_expiration_days: Option<i32>,
    pub notes: Option<String>,
}

/// Update Tenant Ingredient Request
#[derive(Debug, Deserialize)]
pub struct UpdateTenantIngredientRequest {
    pub price: Option<Decimal>,
    pub supplier: Option<String>,
    pub custom_unit: Option<String>,
    pub custom_expiration_days: Option<i32>,
    pub notes: Option<String>,
}

/// Tenant Ingredient Response
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TenantIngredientResponse {
    pub id: Uuid,
    pub catalog_ingredient_id: Uuid,
    pub catalog_name_en: String,
    pub catalog_name_pl: Option<String>,
    pub catalog_name_uk: Option<String>,
    pub catalog_name_ru: Option<String>,
    pub category_id: Uuid,
    pub default_unit: String,
    pub image_url: Option<String>,
    
    // Tenant-specific
    pub price: Option<Decimal>,
    pub supplier: Option<String>,
    pub custom_unit: Option<String>,
    pub custom_expiration_days: Option<i32>,
    pub notes: Option<String>,
}

impl TenantIngredientService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Add ingredient from master catalog to tenant's catalog
    pub async fn add_ingredient(
        &self,
        tenant_id: Uuid,
        req: AddTenantIngredientRequest,
    ) -> AppResult<TenantIngredientResponse> {
        let id = Uuid::new_v4();

        // Verify catalog ingredient exists
        let catalog_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM catalog_ingredients WHERE id = $1 AND COALESCE(is_active, true) = true)"
        )
        .bind(req.catalog_ingredient_id)
        .fetch_one(&self.pool)
        .await?;

        if !catalog_exists {
            return Err(AppError::not_found("Catalog ingredient not found"));
        }

        // Insert tenant ingredient
        sqlx::query(
            r#"
            INSERT INTO tenant_ingredients (
                id, tenant_id, catalog_ingredient_id,
                price, supplier, custom_unit, custom_expiration_days, notes, is_active
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true)
            "#
        )
        .bind(id)
        .bind(tenant_id)
        .bind(req.catalog_ingredient_id)
        .bind(req.price)
        .bind(&req.supplier)
        .bind(req.custom_unit.as_deref())
        .bind(req.custom_expiration_days)
        .bind(&req.notes)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("tenant_ingredient_unique") {
                AppError::conflict("Ingredient already added to your catalog")
            } else {
                AppError::from(e)
            }
        })?;

        // Fetch the created ingredient with catalog data
        let ingredient = sqlx::query_as::<_, TenantIngredientResponse>(
            r#"
            SELECT
                ti.id,
                ti.catalog_ingredient_id,
                ci.name_en as catalog_name_en,
                ci.name_pl as catalog_name_pl,
                ci.name_uk as catalog_name_uk,
                ci.name_ru as catalog_name_ru,
                ci.category_id,
                ci.default_unit::text as default_unit,
                ci.image_url,
                ti.price,
                ti.supplier,
                ti.custom_unit::text as custom_unit,
                ti.custom_expiration_days,
                ti.notes
            FROM tenant_ingredients ti
            JOIN catalog_ingredients ci ON ci.id = ti.catalog_ingredient_id
            WHERE ti.id = $1
            "#
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(ingredient)
    }

    /// List tenant's ingredients with master catalog data
    pub async fn list_ingredients(&self, tenant_id: Uuid) -> AppResult<Vec<TenantIngredientResponse>> {
        let ingredients = sqlx::query_as::<_, TenantIngredientResponse>(
            r#"
            SELECT
                ti.id,
                ti.catalog_ingredient_id,
                ci.name_en as catalog_name_en,
                ci.name_pl as catalog_name_pl,
                ci.name_uk as catalog_name_uk,
                ci.name_ru as catalog_name_ru,
                ci.category_id,
                ci.default_unit::text as default_unit,
                ci.image_url,
                ti.price,
                ti.supplier,
                ti.custom_unit::text as custom_unit,
                ti.custom_expiration_days,
                ti.notes
            FROM tenant_ingredients ti
            JOIN catalog_ingredients ci ON ci.id = ti.catalog_ingredient_id
            WHERE ti.tenant_id = $1 
              AND COALESCE(ti.is_active, true) = true
              AND COALESCE(ci.is_active, true) = true
            ORDER BY ci.name_en ASC
            "#
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ingredients)
    }

    /// Get single tenant ingredient
    pub async fn get_ingredient(&self, tenant_id: Uuid, id: Uuid) -> AppResult<TenantIngredientResponse> {
        let ingredient = sqlx::query_as::<_, TenantIngredientResponse>(
            r#"
            SELECT
                ti.id,
                ti.catalog_ingredient_id,
                ci.name_en as catalog_name_en,
                ci.name_pl as catalog_name_pl,
                ci.name_uk as catalog_name_uk,
                ci.name_ru as catalog_name_ru,
                ci.category_id,
                ci.default_unit::text as default_unit,
                ci.image_url,
                ti.price,
                ti.supplier,
                ti.custom_unit::text as custom_unit,
                ti.custom_expiration_days,
                ti.notes
            FROM tenant_ingredients ti
            JOIN catalog_ingredients ci ON ci.id = ti.catalog_ingredient_id
            WHERE ti.id = $1 
              AND ti.tenant_id = $2
              AND COALESCE(ti.is_active, true) = true
            "#
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Ingredient not found in your catalog"))?;

        Ok(ingredient)
    }

    /// Update tenant ingredient (price, supplier, etc.)
    pub async fn update_ingredient(
        &self,
        tenant_id: Uuid,
        id: Uuid,
        req: UpdateTenantIngredientRequest,
    ) -> AppResult<TenantIngredientResponse> {
        // Verify ownership
        self.get_ingredient(tenant_id, id).await?;

        let ingredient = sqlx::query_as::<_, TenantIngredientResponse>(
            r#"
            UPDATE tenant_ingredients ti
            SET
                price = COALESCE($3, price),
                supplier = COALESCE($4, supplier),
                custom_unit = CASE WHEN $5::text IS NOT NULL THEN $5::unit_type ELSE custom_unit END,
                custom_expiration_days = COALESCE($6, custom_expiration_days),
                notes = COALESCE($7, notes),
                updated_at = CURRENT_TIMESTAMP
            FROM catalog_ingredients ci
            WHERE ti.id = $1 
              AND ti.tenant_id = $2
              AND ci.id = ti.catalog_ingredient_id
              AND COALESCE(ti.is_active, true) = true
            RETURNING
                ti.id,
                ti.catalog_ingredient_id,
                ci.name_en as catalog_name_en,
                ci.name_pl as catalog_name_pl,
                ci.name_uk as catalog_name_uk,
                ci.name_ru as catalog_name_ru,
                ci.category_id,
                ci.default_unit::text as default_unit,
                ci.image_url,
                ti.price,
                ti.supplier,
                ti.custom_unit::text as custom_unit,
                ti.custom_expiration_days,
                ti.notes
            "#
        )
        .bind(id)
        .bind(tenant_id)
        .bind(req.price)
        .bind(&req.supplier)
        .bind(req.custom_unit.as_deref())
        .bind(req.custom_expiration_days)
        .bind(&req.notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(ingredient)
    }

    /// Remove ingredient from tenant catalog (soft delete)
    pub async fn remove_ingredient(&self, tenant_id: Uuid, id: Uuid) -> AppResult<()> {
        let result = sqlx::query(
            "UPDATE tenant_ingredients SET is_active = false WHERE id = $1 AND tenant_id = $2 AND COALESCE(is_active, true) = true"
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Ingredient not found in your catalog"));
        }

        Ok(())
    }

    /// Search available catalog ingredients not yet added by tenant
    pub async fn search_available_ingredients(
        &self,
        tenant_id: Uuid,
        query: &str,
    ) -> AppResult<Vec<AvailableIngredientResponse>> {
        let ingredients = sqlx::query_as::<_, AvailableIngredientResponse>(
            r#"
            SELECT
                ci.id,
                ci.name_en,
                ci.name_pl,
                ci.name_uk,
                ci.name_ru,
                ci.category_id,
                ci.default_unit::text as default_unit,
                ci.image_url,
                EXISTS(
                    SELECT 1 FROM tenant_ingredients ti
                    WHERE ti.catalog_ingredient_id = ci.id
                      AND ti.tenant_id = $1
                      AND COALESCE(ti.is_active, true) = true
                ) as already_added
            FROM catalog_ingredients ci
            WHERE COALESCE(ci.is_active, true) = true
              AND ci.name_en ILIKE '%' || $2 || '%'
            ORDER BY ci.name_en ASC
            LIMIT 50
            "#
        )
        .bind(tenant_id)
        .bind(query)
        .fetch_all(&self.pool)
        .await?;

        Ok(ingredients)
    }
}

/// Available catalog ingredient (for adding to tenant)
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AvailableIngredientResponse {
    pub id: Uuid,
    pub name_en: String,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub category_id: Uuid,
    pub default_unit: String,
    pub image_url: Option<String>,
    pub already_added: bool,
}
