use crate::domain::catalog::{CatalogIngredientId, Unit};
use crate::domain::tenant_ingredient::{TenantIngredient, TenantIngredientId};
use crate::infrastructure::persistence::TenantIngredientRepositoryTrait;
use crate::shared::{AppError, AppResult, Language, TenantId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Service for managing tenant-specific ingredient data (prices, suppliers)
#[derive(Clone)]
pub struct TenantIngredientService {
    repository: Arc<dyn TenantIngredientRepositoryTrait>,
}

impl TenantIngredientService {
    pub fn new(repository: Arc<dyn TenantIngredientRepositoryTrait>) -> Self {
        Self { repository }
    }

    /// Add ingredient from master catalog to tenant's catalog
    pub async fn add_ingredient(
        &self,
        tenant_id: TenantId,
        _language: Language,
        req: AddTenantIngredientRequest,
    ) -> AppResult<Uuid> {
        let catalog_id = CatalogIngredientId::from_uuid(req.catalog_ingredient_id);

        // Use repository to check if already exists
        if (self
            .repository
            .find_by_catalog_id(tenant_id, catalog_id)
            .await?)
            .is_some()
        {
            return Err(AppError::validation(
                "Ingredient already exists in tenant catalog",
            ));
        }

        let mut ingredient = TenantIngredient::new(
            tenant_id,
            catalog_id,
            req.price,
            req.supplier,
        );
        
        if let Some(unit_str) = req.custom_unit {
            ingredient.custom_unit = Some(Unit::from_str(&unit_str)?);
        }
        ingredient.custom_expiration_days = req.custom_expiration_days;
        ingredient.notes = req.notes;

        self.repository.save(&ingredient).await?;

        Ok(ingredient.id.as_uuid())
    }

    pub async fn list_ingredients(
        &self,
        tenant_id: TenantId,
        _language: Language,
    ) -> AppResult<Vec<TenantIngredientResponse>> {
        let ingredients = self.repository.list_by_tenant(tenant_id).await?;
        
        // Note: For a real app, we might want to JOIN with catalog_ingredients to get catalog_name, etc.
        // For now, this is a simplified conversion.
        Ok(ingredients.into_iter().map(|i| TenantIngredientResponse {
            id: i.id.as_uuid(),
            catalog_ingredient_id: i.catalog_ingredient_id.as_uuid(),
            catalog_name: "Catalog Ingredient".to_string(), // Placeholder, should be joined
            category_id: Uuid::nil(), // Placeholder
            default_unit: i.custom_unit.map(|u| u.as_str().to_string()).unwrap_or_else(|| "unit".to_string()),
            image_url: None,
            price: i.price,
            supplier: i.supplier,
            custom_unit: i.custom_unit.map(|u| u.as_str().to_string()),
            custom_expiration_days: i.custom_expiration_days,
            notes: i.notes,
        }).collect())
    }

    pub async fn get_ingredient(
        &self,
        tenant_id: TenantId,
        id: Uuid,
        _language: Language,
    ) -> AppResult<TenantIngredientResponse> {
        let ingredient = self.repository.list_by_tenant(tenant_id).await?
            .into_iter()
            .find(|i| i.id.as_uuid() == id)
            .ok_or_else(|| AppError::not_found("Tenant ingredient not found"))?;

        Ok(TenantIngredientResponse {
            id: ingredient.id.as_uuid(),
            catalog_ingredient_id: ingredient.catalog_ingredient_id.as_uuid(),
            catalog_name: "Catalog Ingredient".to_string(),
            category_id: Uuid::nil(),
            default_unit: ingredient.custom_unit.map(|u| u.as_str().to_string()).unwrap_or_else(|| "unit".to_string()),
            image_url: None,
            price: ingredient.price,
            supplier: ingredient.supplier,
            custom_unit: ingredient.custom_unit.map(|u| u.as_str().to_string()),
            custom_expiration_days: ingredient.custom_expiration_days,
            notes: ingredient.notes,
        })
    }

    pub async fn update_ingredient(
        &self,
        tenant_id: TenantId,
        id: Uuid,
        _language: Language,
        req: UpdateTenantIngredientRequest,
    ) -> AppResult<TenantIngredientResponse> {
        let mut ingredients = self.repository.list_by_tenant(tenant_id).await?;
        let ingredient = ingredients.iter_mut()
            .find(|i| i.id.as_uuid() == id)
            .ok_or_else(|| AppError::not_found("Tenant ingredient not found"))?;

        if let Some(price) = req.price {
            ingredient.price = Some(price);
        }
        if let Some(supplier) = req.supplier {
            ingredient.supplier = Some(supplier);
        }
        if let Some(unit_str) = req.custom_unit {
            ingredient.custom_unit = Some(Unit::from_str(&unit_str)?);
        }
        if let Some(days) = req.custom_expiration_days {
            ingredient.custom_expiration_days = Some(days);
        }
        if let Some(notes) = req.notes {
            ingredient.notes = Some(notes);
        }

        self.repository.save(ingredient).await?;

        Ok(TenantIngredientResponse {
            id: ingredient.id.as_uuid(),
            catalog_ingredient_id: ingredient.catalog_ingredient_id.as_uuid(),
            catalog_name: "Catalog Ingredient".to_string(),
            category_id: Uuid::nil(),
            default_unit: ingredient.custom_unit.map(|u| u.as_str().to_string()).unwrap_or_else(|| "unit".to_string()),
            image_url: None,
            price: ingredient.price,
            supplier: ingredient.supplier.clone(),
            custom_unit: ingredient.custom_unit.map(|u| u.as_str().to_string()),
            custom_expiration_days: ingredient.custom_expiration_days,
            notes: ingredient.notes.clone(),
        })
    }

    pub async fn remove_ingredient(&self, tenant_id: TenantId, id: Uuid) -> AppResult<()> {
        self.repository.delete(TenantIngredientId::from_uuid(id), tenant_id).await?;
        Ok(())
    }

    pub async fn search_available_ingredients(
        &self,
        _tenant_id: TenantId,
        _language: Language,
        _query: &str,
    ) -> AppResult<Vec<TenantIngredientResponse>> {
        // This would require a more complex repository method to find catalog ingredients 
        // that are NOT yet in tenant_ingredients.
        Ok(vec![])
    }
}

#[derive(Debug, Deserialize)]
pub struct AddTenantIngredientRequest {
    pub catalog_ingredient_id: Uuid,
    pub price: Option<Decimal>,
    pub supplier: Option<String>,
    pub custom_unit: Option<String>,
    pub custom_expiration_days: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantIngredientRequest {
    pub price: Option<Decimal>,
    pub supplier: Option<String>,
    pub custom_unit: Option<String>,
    pub custom_expiration_days: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TenantIngredientResponse {
    pub id: Uuid,
    pub catalog_ingredient_id: Uuid,
    pub catalog_name: String,
    pub category_id: Uuid,
    pub default_unit: String,
    pub image_url: Option<String>,
    pub price: Option<Decimal>,
    pub supplier: Option<String>,
    pub custom_unit: Option<String>,
    pub custom_expiration_days: Option<i32>,
    pub notes: Option<String>,
}
