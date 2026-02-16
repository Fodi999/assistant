use crate::domain::CatalogIngredientId;
use crate::shared::{TenantId, Language, AppResult, AppError};
use crate::infrastructure::persistence::TenantIngredientRepositoryTrait;
use std::sync::Arc;
use uuid::Uuid;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
        if (self.repository.find_by_catalog_id(tenant_id, catalog_id).await?).is_some() {
            return Err(AppError::validation("Ingredient already exists in tenant catalog"));
        }
        
        Ok(Uuid::new_v4())
    }

    pub async fn list_ingredients(&self, _tenant_id: TenantId, _language: Language) -> AppResult<Vec<TenantIngredientResponse>> {
        Ok(vec![])
    }

    pub async fn get_ingredient(&self, _tenant_id: TenantId, _id: Uuid, _language: Language) -> AppResult<TenantIngredientResponse> {
        Err(AppError::not_found("Not implemented"))
    }

    pub async fn update_ingredient(&self, _tenant_id: TenantId, _id: Uuid, _language: Language, _req: UpdateTenantIngredientRequest) -> AppResult<TenantIngredientResponse> {
        Err(AppError::not_found("Not implemented"))
    }

    pub async fn remove_ingredient(&self, _tenant_id: TenantId, _id: Uuid) -> AppResult<()> {
        Ok(())
    }

    pub async fn search_available_ingredients(&self, _tenant_id: TenantId, _language: Language, _query: &str) -> AppResult<Vec<TenantIngredientResponse>> {
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
