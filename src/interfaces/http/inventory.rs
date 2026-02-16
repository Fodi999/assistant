use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::inventory::{InventoryService, InventoryView, InventoryStatus, LossReport};
use crate::application::inventory_alert::InventoryAlertService;
use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{InventoryBatch, InventoryBatchId, InventoryAlert},
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
    /// Product receipt/purchase date (дата поступления)
    #[serde(with = "time::serde::rfc3339")]
    pub received_at: OffsetDateTime,
    /// Expiration date (дата просрочки)
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
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
    #[serde(with = "time::serde::rfc3339")]
    pub received_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<InventoryBatch> for ProductResponse {
    fn from(product: InventoryBatch) -> Self {
        Self {
            id: product.id.as_uuid(),
            catalog_ingredient_id: product.catalog_ingredient_id.as_uuid(),
            price_per_unit_cents: product.price_per_unit.as_cents(),
            quantity: product.quantity.value(),
            received_at: product.received_at,
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
/// Returns enriched InventoryView with product details (name, category, unit, image_url)
pub async fn add_product(
    State(service): State<InventoryService>,
    auth: AuthUser,
    Json(req): Json<AddProductRequest>,
) -> Result<(StatusCode, Json<InventoryView>), AppError> {
    let product_id = service
        .add_product(
            auth.user_id,
            auth.tenant_id,
            CatalogIngredientId::from_uuid(req.catalog_ingredient_id),
            req.price_per_unit_cents,
            req.quantity,
            req.received_at,
            req.expires_at,
        )
        .await?;

    // 🎯 Return enriched InventoryView (Query DTO pattern)
    // Includes: product name, category, unit, image_url, expiration status
    let products = service
        .list_products_with_details(auth.user_id, auth.tenant_id, auth.language)
        .await?;
    
    let product_view = products
        .into_iter()
        .find(|p| p.id == product_id.as_uuid())
        .ok_or_else(|| AppError::internal("Failed to retrieve created product"))?;

    Ok((StatusCode::CREATED, Json(product_view)))
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
            InventoryBatchId::from_uuid(id),
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
        .delete_product(InventoryBatchId::from_uuid(id), auth.user_id, auth.tenant_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/inventory/health
/// Unified inventory health endpoint (for badge and dashboard)
pub async fn get_health(
    State(service): State<InventoryAlertService>,
    auth: AuthUser,
) -> Result<Json<InventoryStatus>, AppError> {
    let status = service.get_inventory_status(auth.tenant_id).await?;
    Ok(Json(status))
}

/// GET /api/inventory/alerts
/// Get all active alerts (expiring batches + low stock)
pub async fn get_alerts(
    State(service): State<InventoryAlertService>,
    auth: AuthUser,
) -> Result<Json<Vec<InventoryAlert>>, AppError> {
    let alerts = service.get_alerts(auth.tenant_id).await?;
    Ok(Json(alerts))
}

/// POST /api/inventory/process-expirations
/// Automatically exhaust expired batches and log losses
pub async fn process_expirations(
    State(service): State<InventoryService>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = service.process_expirations(auth.tenant_id).await?;
    Ok(Json(serde_json::json!({ "processed_count": count })))
}

#[derive(Debug, serde::Deserialize)]
pub struct LossReportQuery {
    #[serde(default = "default_loss_report_days")]
    pub days: i32,
}

fn default_loss_report_days() -> i32 {
    30
}

/// GET /api/inventory/reports/loss
/// Financial loss report (expired quantities/costs)
pub async fn get_loss_report(
    State(service): State<InventoryService>,
    auth: AuthUser,
    axum::extract::Query(query): axum::extract::Query<LossReportQuery>,
) -> Result<Json<LossReport>, AppError> {
    let report = service.get_loss_report(auth.tenant_id, query.days).await?;
    Ok(Json(report))
}
