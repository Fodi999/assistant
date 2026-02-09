use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::{catalog::CatalogService, user::UserService};
use crate::domain::catalog::CatalogCategoryId;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

/// Shared state for catalog handlers (catalog service + user service for language lookup)
#[derive(Clone)]
pub struct CatalogState {
    pub catalog_service: CatalogService,
    pub user_service: UserService,
}

/// Response for a single category
#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub sort_order: i32,
}

/// Response for a list of categories
#[derive(Debug, Serialize)]
pub struct CategoriesResponse {
    pub categories: Vec<CategoryResponse>,
}

/// Query parameters for ingredient search
#[derive(Debug, Deserialize)]
pub struct SearchIngredientsQuery {
    pub category_id: Option<String>,
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

/// Response for a single ingredient
#[derive(Debug, Serialize)]
pub struct IngredientResponse {
    pub id: String,
    pub category_id: String,
    pub name: String,
    pub default_unit: String,
    pub default_shelf_life_days: Option<i32>,
    pub allergens: Vec<String>,
    pub calories_per_100g: Option<i32>,
    pub seasons: Vec<String>,
    pub image_url: Option<String>,
}

/// Response for a list of ingredients
#[derive(Debug, Serialize)]
pub struct IngredientsResponse {
    pub ingredients: Vec<IngredientResponse>,
}

/// GET /api/catalog/categories
/// Returns all categories in user's language
pub async fn get_categories(
    State(state): State<CatalogState>,
    auth_user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    // Get user to determine language
    let user_with_tenant = state.user_service.get_user_with_tenant(auth_user.user_id).await?;
    let language = user_with_tenant.user.language;

    let categories = state.catalog_service.get_categories(language).await?;

    let response = CategoriesResponse {
        categories: categories
            .into_iter()
            .map(|cat| CategoryResponse {
                id: cat.id.to_string(),
                name: cat.name(language).to_string(),
                sort_order: cat.sort_order,
            })
            .collect(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/catalog/ingredients?category_id=...&q=...&limit=50
/// Search ingredients with optional category filter and name query
pub async fn search_ingredients(
    State(state): State<CatalogState>,
    auth: AuthUser,
    Query(params): Query<SearchIngredientsQuery>,
) -> Result<impl IntoResponse, AppError> {
    // 🎯 ЭТАЛОН B2B SaaS: Language from AuthUser (backend source of truth)
    let language = auth.language;

    let ingredients = if let Some(category_id_str) = params.category_id {
        // Parse category UUID
        let category_uuid = Uuid::parse_str(&category_id_str)
            .map_err(|_| AppError::validation("Invalid category_id UUID format"))?;
        let category_id = CatalogCategoryId::from_uuid(category_uuid);

        // Search by category with optional name filter
        state
            .catalog_service
            .search_ingredients_by_category(
                category_id,
                params.q.as_deref(),
                language,
                params.limit,
            )
            .await?
    } else if let Some(query) = params.q.as_deref() {
        // Search all ingredients by name (only if query provided)
        state
            .catalog_service
            .search_ingredients(query, language, params.limit)
            .await?
    } else {
        // List all ingredients (no filter)
        state
            .catalog_service
            .list_ingredients(language, 0, params.limit)
            .await?
    };

    let response = IngredientsResponse {
        ingredients: ingredients
            .into_iter()
            .map(|ing| IngredientResponse {
                id: ing.id.to_string(),
                category_id: ing.category_id.to_string(),
                name: ing.name(language).to_string(),
                default_unit: ing.default_unit.as_str().to_string(),
                default_shelf_life_days: ing.default_shelf_life_days,
                allergens: ing.allergens.iter().map(|a| a.as_str().to_string()).collect(),
                calories_per_100g: ing.calories_per_100g,
                seasons: ing.seasons.iter().map(|s| s.as_str().to_string()).collect(),
                image_url: ing.image_url.clone(),
            })
            .collect(),
    };

    Ok((StatusCode::OK, Json(response)))
}
