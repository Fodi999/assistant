use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::inventory::{InventoryService, InventoryView};
use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{InventoryProduct, InventoryProductId},
};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AddProductRequest {
    pub catalog_ingredient_id: Uuid,
    pub price_per_unit_cents: i64,
    pub quantity: f64,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub price_per_unit_cents: Option<i64>,
    pub quantity: Option<f64>,
}

/// Legacy response (for backward compatibility if needed)
#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub id: Uuid,
    pub catalog_ingredient_id: Uuid,
    pub price_per_unit_cents: i64,
    pub quantity: f64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<InventoryProduct> for ProductResponse {
    fn from(product: InventoryProduct) -> Self {
        Self {
            id: product.id.as_uuid(),
            catalog_ingredient_id: product.catalog_ingredient_id.as_uuid(),
            price_per_unit_cents: product.price_per_unit.as_cents(),
            quantity: product.quantity.value(),
            expires_at: product.expires_at,
            created_at: product.created_at,
            updated_at: product.updated_at,
        }
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/inventory/products
/// List all inventory products with full details (ingredient name, category, unit)
/// Uses Query DTO pattern - single request returns everything needed for UI
/// 🎯 ЭТАЛОН B2B SaaS: Language source = user.language from database!
pub async fn list_products(
    State(service): State<InventoryService>,
    auth: AuthUser,
) -> Result<Json<Vec<InventoryView>>, AppError> {
    // 🎯 Backend = source of truth для языка!
    // auth.language загружается из БД в middleware
    // Frontend НЕ передает язык руками - правильный подход для SaaS!
    let products = service
        .list_products_with_details(auth.user_id, auth.tenant_id, auth.language)
        .await?;
    
    Ok(Json(products))
}

/// POST /api/inventory/products
/// Add a new product to inventory
pub async fn add_product(
    State(service): State<InventoryService>,
    auth: AuthUser,
    Json(req): Json<AddProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>), AppError> {
    let product_id = service
        .add_product(
            auth.user_id,
            auth.tenant_id,
            CatalogIngredientId::from_uuid(req.catalog_ingredient_id),
            req.price_per_unit_cents,
            req.quantity,
            req.expires_at,
        )
        .await?;

    // Retrieve the created product to return full details
    let products = service.list_products(auth.user_id, auth.tenant_id).await?;
    let product = products
        .into_iter()
        .find(|p| p.id == product_id)
        .ok_or_else(|| AppError::internal("Failed to retrieve created product"))?;

    Ok((StatusCode::CREATED, Json(ProductResponse::from(product))))
}

/// PUT /api/inventory/products/:id
/// Update an existing inventory product
pub async fn update_product(
    State(service): State<InventoryService>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<StatusCode, AppError> {
    service
        .update_product(
            InventoryProductId::from_uuid(id),
            auth.user_id,
            auth.tenant_id,
            req.price_per_unit_cents,
            req.quantity,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/inventory/products/:id
/// Delete an inventory product
pub async fn delete_product(
    State(service): State<InventoryService>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service
        .delete_product(InventoryProductId::from_uuid(id), auth.user_id, auth.tenant_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/inventory/status
/// Get aggregated inventory status (for assistant)
pub async fn get_status(
    State(service): State<InventoryService>,
    auth: AuthUser,
) -> Result<Json<crate::application::inventory::InventoryStatus>, AppError> {
    let status = service.get_status(auth.user_id, auth.tenant_id).await?;
    Ok(Json(status))
}
