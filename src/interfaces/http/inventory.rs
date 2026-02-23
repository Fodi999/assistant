use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::inventory::{InventoryService, InventoryStatus, InventoryView, LossReport};
use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{InventoryAlert, InventoryBatch, InventoryBatchId},
};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::{AppError, PaginationParams};

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

/// GET /api/inventory/products?page=1&per_page=50
/// List all inventory products with full details (paginated)
/// 🎯 ЭТАЛОН B2B SaaS: Language source = user.language from database!
pub async fn list_products(
    State(service): State<InventoryService>,
    auth: AuthUser,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 🎯 Backend = source of truth для языка!
    let products = service
        .list_products_with_details(auth.user_id, auth.tenant_id, auth.language)
        .await?;

    // Apply pagination in-memory (the query already JOINs with catalogs,
    // adding SQL pagination would require significant refactoring of the complex JOIN).
    let total = products.len() as i64;
    let start = pagination.offset() as usize;
    let end = (start + pagination.per_page() as usize).min(products.len());
    let items: Vec<InventoryView> = if start < products.len() {
        products[start..end].to_vec()
    } else {
        vec![]
    };

    let per_page = pagination.per_page();
    let total_pages = if total > 0 {
        ((total as u32) + per_page - 1) / per_page
    } else {
        0
    };

    Ok(Json(serde_json::json!({
        "items": items,
        "total": total,
        "page": pagination.page(),
        "per_page": per_page,
        "total_pages": total_pages,
    })))
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
        .delete_product(
            InventoryBatchId::from_uuid(id),
            auth.user_id,
            auth.tenant_id,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/inventory/dashboard
/// Comprehensive dashboard for the owner
pub async fn get_dashboard(
    State(service): State<InventoryService>,
    auth: AuthUser,
) -> Result<Json<crate::application::inventory::InventoryDashboard>, AppError> {
    let dashboard = service.get_dashboard(auth.tenant_id, auth.language).await?;
    Ok(Json(dashboard))
}

/// GET /api/inventory/health
/// Unified inventory health endpoint (for badge and dashboard)
pub async fn get_health(
    State(service): State<InventoryService>,
    auth: AuthUser,
) -> Result<Json<InventoryStatus>, AppError> {
    let status = service.get_status(auth.tenant_id).await?;
    Ok(Json(status))
}

/// GET /api/inventory/alerts
/// Get all active alerts (expiring batches + low stock)
pub async fn get_alerts(
    State(service): State<InventoryService>,
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
    let report = service
        .get_loss_report(auth.tenant_id, query.days, auth.language.code())
        .await?;
    Ok(Json(report))
}

pub fn router(service: InventoryService) -> Router {
    Router::new()
        .route("/products", get(list_products))
        .route("/products", post(add_product))
        .route("/products/:id", put(update_product))
        .route("/products/:id", delete(delete_product))
        .route("/health", get(get_health))
        .route("/alerts", get(get_alerts))
        .route("/dashboard", get(get_dashboard)) // New ownership dashboard
        .route("/reports/loss", get(get_loss_report))
        .route("/process-expirations", post(process_expirations))
        .with_state(service)
}
