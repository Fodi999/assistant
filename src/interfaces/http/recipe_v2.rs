// Recipe V2 HTTP Handlers - REST API with automatic translations
use crate::application::recipe_v2_service::{CreateRecipeDto, RecipeResponseDto, RecipeV2Service, UpdateRecipeDto};
use crate::domain::recipe_v2::RecipeId;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppResult;
use axum::{
    extract::{Multipart, Path, Query, State},
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
    AuthUser {
        user_id,
        tenant_id,
        language: _,
    }: AuthUser,
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
    AuthUser {
        user_id: _,
        tenant_id,
        language,
    }: AuthUser,
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
    AuthUser {
        user_id,
        tenant_id,
        language,
    }: AuthUser,
    Query(params): Query<RecipeListParams>,
) -> AppResult<Json<Vec<RecipeResponseDto>>> {
    // For now, we use a simple implementation that ignores search but respects tenant isolation
    // The repository method find_by_user_id will be used
    let recipes = service
        .list_user_recipes(user_id, tenant_id, language)
        .await?;

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

/// PUT /api/recipes/v2/:id - Update recipe with localization trigger
/// Path param: recipe_id (UUID)
/// Body: UpdateRecipeDto
/// Returns: RecipeResponseDto
pub async fn update_recipe(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser {
        user_id: _,
        tenant_id,
        language: _,
    }: AuthUser,
    Path(recipe_id): Path<Uuid>,
    Json(dto): Json<UpdateRecipeDto>,
) -> AppResult<Json<RecipeResponseDto>> {
    let recipe = service
        .update_recipe(RecipeId(recipe_id), dto, tenant_id)
        .await?;
    Ok(Json(recipe))
}

/// POST /api/recipes/v2/:id/publish - Publish recipe (make public)
/// Path param: recipe_id (UUID)
/// Returns: 204 No Content
pub async fn publish_recipe(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser {
        user_id: _,
        tenant_id,
        language: _,
    }: AuthUser,
    Path(recipe_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    service
        .publish_recipe(RecipeId(recipe_id), tenant_id)
        .await?;
    Ok(StatusCode::OK)
}

/// DELETE /api/recipes/v2/:id - Delete recipe and all related data
/// Path param: recipe_id (UUID)
/// Returns: 204 No Content
pub async fn delete_recipe(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser {
        user_id: _,
        tenant_id,
        language: _,
    }: AuthUser,
    Path(recipe_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    service
        .delete_recipe(RecipeId(recipe_id), tenant_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/recipes/v2/:id/image
/// Body: Multipart file
pub async fn upload_recipe_image(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser {
        user_id: _,
        tenant_id,
        language: _,
    }: AuthUser,
    Path(recipe_id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<Json<crate::interfaces::http::admin_catalog::ImageUrlResponse>> {
    let mut file_data = None;
    let mut content_type = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| crate::shared::AppError::validation(&format!("Invalid multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("");
        if name == "file" || name == "image" {
            content_type = field.content_type().map(|ct| ct.to_string());
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| crate::shared::AppError::validation(&format!("Failed to read image: {}", e)))?,
            );
            break;
        }
    }

    let file_data = file_data.ok_or_else(|| crate::shared::AppError::validation("No file provided"))?;
    let content_type = content_type.ok_or_else(|| crate::shared::AppError::validation("No content-type"))?;

    let image_url = service
        .upload_image(RecipeId(recipe_id), tenant_id, file_data.to_vec(), &content_type)
        .await?;

    Ok(Json(crate::interfaces::http::admin_catalog::ImageUrlResponse { image_url }))
}

/// GET /api/recipes/v2/:id/image-url
/// Returns presigned URL for direct upload
pub async fn get_recipe_image_upload_url(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser {
        user_id: _,
        tenant_id,
        language: _,
    }: AuthUser,
    Path(recipe_id): Path<Uuid>,
    Query(query): Query<crate::interfaces::http::admin_catalog::GetUploadUrlQuery>,
) -> AppResult<Json<crate::application::user::AvatarUploadResponse>> {
    let content_type = query
        .content_type
        .unwrap_or_else(|| "image/webp".to_string());

    let response = service
        .get_image_upload_url(RecipeId(recipe_id), tenant_id, &content_type)
        .await?;

    Ok(Json(response))
}

#[derive(Debug, serde::Deserialize)]
pub struct SaveImageUrlRequest {
    pub image_url: String,
}

/// PUT /api/recipes/v2/:id/image
/// Body: { "image_url": "..." }
pub async fn save_recipe_image_url(
    State(service): State<Arc<RecipeV2Service>>,
    AuthUser {
        user_id: _,
        tenant_id,
        language: _,
    }: AuthUser,
    Path(recipe_id): Path<Uuid>,
    Json(req): Json<SaveImageUrlRequest>,
) -> AppResult<StatusCode> {
    service
        .save_image_url(RecipeId(recipe_id), tenant_id, req.image_url)
        .await?;
    Ok(StatusCode::OK)
}
