use crate::application::{
    AddTenantIngredientRequest, TenantIngredientService, UpdateTenantIngredientRequest,
};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// Add ingredient from master catalog to tenant's catalog
/// POST /api/tenant/ingredients
pub async fn add_ingredient(
    AuthUser { user_id, tenant_id, language }: AuthUser,
    State(service): State<TenantIngredientService>,
    Json(req): Json<AddTenantIngredientRequest>,
) -> AppResult<impl IntoResponse> {
    let ingredient = service.add_ingredient(*tenant_id.as_uuid(), req).await?;
    Ok((StatusCode::CREATED, Json(ingredient)))
}

/// List tenant's ingredients
/// GET /api/tenant/ingredients
pub async fn list_ingredients(
    AuthUser { user_id, tenant_id, language }: AuthUser,
    State(service): State<TenantIngredientService>,
) -> AppResult<Json<serde_json::Value>> {
    let ingredients = service.list_ingredients(*tenant_id.as_uuid()).await?;
    Ok(Json(serde_json::json!({
        "ingredients": ingredients
    })))
}

/// Get single tenant ingredient
/// GET /api/tenant/ingredients/:id
pub async fn get_ingredient(
    AuthUser { user_id, tenant_id, language }: AuthUser,
    State(service): State<TenantIngredientService>,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let ingredient = service.get_ingredient(*tenant_id.as_uuid(), id).await?;
    Ok(Json(ingredient))
}

/// Update tenant ingredient (price, supplier, etc.)
/// PUT /api/tenant/ingredients/:id
pub async fn update_ingredient(
    AuthUser { user_id, tenant_id, language }: AuthUser,
    State(service): State<TenantIngredientService>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTenantIngredientRequest>,
) -> AppResult<impl IntoResponse> {
    let ingredient = service.update_ingredient(*tenant_id.as_uuid(), id, req).await?;
    Ok(Json(ingredient))
}

/// Remove ingredient from tenant catalog (soft delete)
/// DELETE /api/tenant/ingredients/:id
pub async fn remove_ingredient(
    AuthUser { user_id, tenant_id, language }: AuthUser,
    State(service): State<TenantIngredientService>,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    service.remove_ingredient(*tenant_id.as_uuid(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Search available catalog ingredients not yet added by tenant
/// GET /api/tenant/ingredients/search?q=tomato
pub async fn search_available_ingredients(
    AuthUser { user_id, tenant_id, language }: AuthUser,
    State(service): State<TenantIngredientService>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let ingredients = service
        .search_available_ingredients(*tenant_id.as_uuid(), &query.q)
        .await?;
    Ok(Json(serde_json::json!({
        "ingredients": ingredients
    })))
}
