use crate::application::{
    AdminNutritionService, AllergensDto, CulinaryBehaviorDto, CulinaryDto, DietFlagsDto,
    FattyAcidsDto, FoodPropertiesDto, HealthProfileDto, MacrosDto, MineralsDto,
    ProcessingEffectsDto, SugarProfileDto, UpdateProductBasicRequest, VitaminsDto,
};
use crate::domain::AdminClaims;
use crate::shared::AppError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Query params for list ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListProductsQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub product_type: Option<String>,
    pub search: Option<String>,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 50 }

// ── Paginated response ────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PaginatedProducts<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

// ══════════════════════════════════════════════════════
// HANDLERS
// ══════════════════════════════════════════════════════

/// GET /api/admin/nutrition/products
pub async fn list_products(
    _claims: AdminClaims,
    State(svc): State<AdminNutritionService>,
    Query(q): Query<ListProductsQuery>,
) -> Result<Json<PaginatedProducts<crate::application::NutritionProductRow>>, AppError> {
    let total = svc
        .count_products(q.product_type.clone(), q.search.clone())
        .await?;
    let items = svc
        .list_products(q.page, q.limit, q.product_type, q.search)
        .await?;
    Ok(Json(PaginatedProducts {
        items,
        total,
        page: q.page,
        limit: q.limit,
    }))
}

/// GET /api/admin/nutrition/products/:id
pub async fn get_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
) -> Result<Json<crate::application::NutritionProductDetail>, AppError> {
    let detail = svc.get_product(id).await?;
    Ok(Json(detail))
}

/// PUT /api/admin/nutrition/products/:id/basic
pub async fn update_basic(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<UpdateProductBasicRequest>,
) -> Result<StatusCode, AppError> {
    svc.update_basic(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/macros
pub async fn update_macros(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<MacrosDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_macros(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/vitamins
pub async fn update_vitamins(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<VitaminsDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_vitamins(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/minerals
pub async fn update_minerals(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<MineralsDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_minerals(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/fatty-acids
pub async fn update_fatty_acids(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<FattyAcidsDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_fatty_acids(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/diet-flags
pub async fn update_diet_flags(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<DietFlagsDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_diet_flags(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/allergens
pub async fn update_allergens(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<AllergensDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_allergens(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/food-props
pub async fn update_food_props(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<FoodPropertiesDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_food_properties(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/culinary
pub async fn update_culinary(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<CulinaryDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_culinary(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/health-profile
pub async fn update_health_profile(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<HealthProfileDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_health_profile(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/sugar-profile
pub async fn update_sugar_profile(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<SugarProfileDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_sugar_profile(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/processing-effects
pub async fn update_processing_effects(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<ProcessingEffectsDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_processing_effects(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/admin/nutrition/products/:id/culinary-behavior
pub async fn update_culinary_behavior(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<AdminNutritionService>,
    Json(body): Json<CulinaryBehaviorDto>,
) -> Result<StatusCode, AppError> {
    svc.upsert_culinary_behavior(id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}
