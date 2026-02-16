// Recipe V2 HTTP Handlers - REST API with automatic translations
use crate::application::recipe_v2_service::{
    CreateRecipeDto, RecipeResponseDto, RecipeV2Service,
};
use crate::domain::recipe_v2::RecipeId;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppResult;
use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

/// POST /api/recipes/v2 - Create new recipe with automatic translations
/// Body: CreateRecipeDto
/// Returns: RecipeResponseDto with status 201
pub async fn create_recipe(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser { user_id, tenant_id, language: _ }: AuthUser,
    Json(dto): Json<CreateRecipeDto>,
) -> AppResult<(StatusCode, Json<RecipeResponseDto>)> {
    let recipe = service.create_recipe(dto, user_id, tenant_id).await?;
    Ok((StatusCode::CREATED, Json(recipe)))
}

/// GET /api/recipes/v2/:id - Get recipe with localized content
/// Path param: recipe_id (UUID)
/// Returns: RecipeResponseDto in user's language
pub async fn get_recipe(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser { user_id: _, tenant_id, language }: AuthUser,
    Path(recipe_id): Path<Uuid>,
) -> AppResult<Json<RecipeResponseDto>> {
    let recipe = service
        .get_recipe(RecipeId(recipe_id), tenant_id, language)
        .await?;
    Ok(Json(recipe))
}

#[derive(Debug, Deserialize)]
pub struct RecipeListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

/// GET /api/recipes/v2 - List all user's recipes with localized content
/// Returns: Vec<RecipeResponseDto> in user's language
pub async fn list_recipes(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser { user_id, tenant_id, language }: AuthUser,
    Query(params): Query<RecipeListParams>,
) -> AppResult<Json<Vec<RecipeResponseDto>>> {
    // For now, we use a simple implementation that ignores search but respects tenant isolation
    // The repository method find_by_user_id will be used
    let recipes = service.list_user_recipes(user_id, tenant_id, language).await?;
    
    // Manual slicing for basic pagination (temporary until repo supports it)
    let limit = params.limit.unwrap_or(100) as usize;
    let offset = params.offset.unwrap_or(0) as usize;
    
    let mut response = recipes;
    if offset < response.len() {
        let end = (offset + limit).min(response.len());
        response = response[offset..end].to_vec();
    } else {
        response = vec![];
    }

    Ok(Json(response))
}

/// POST /api/recipes/v2/:id/publish - Publish recipe (make public)
/// Path param: recipe_id (UUID)
/// Returns: 204 No Content
pub async fn publish_recipe(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser { user_id: _, tenant_id, language: _ }: AuthUser,
    Path(recipe_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    service.publish_recipe(RecipeId(recipe_id), tenant_id).await?;
    Ok(StatusCode::OK)
}

/// DELETE /api/recipes/v2/:id - Delete recipe and all related data
/// Path param: recipe_id (UUID)
/// Returns: 204 No Content
pub async fn delete_recipe(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser { user_id: _, tenant_id, language: _ }: AuthUser,
    Path(recipe_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    service.delete_recipe(RecipeId(recipe_id), tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
