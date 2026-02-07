use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::RecipeService;
use crate::domain::{RecipeId, RecipeName, Servings, RecipeIngredient, CatalogIngredientId, Quantity};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

/// Request to create a new recipe
#[derive(Debug, Deserialize)]
pub struct CreateRecipeRequest {
    pub name: String,
    pub servings: u32,
    pub ingredients: Vec<RecipeIngredientRequest>,
}

#[derive(Debug, Deserialize)]
pub struct RecipeIngredientRequest {
    pub catalog_ingredient_id: Uuid,
    pub quantity: f64,
}

/// Response with recipe basic info
#[derive(Debug, Serialize)]
pub struct RecipeResponse {
    pub id: Uuid,
    pub name: String,
    pub servings: u32,
    pub ingredients: Vec<RecipeIngredientResponse>,
}

#[derive(Debug, Serialize)]
pub struct RecipeIngredientResponse {
    pub catalog_ingredient_id: Uuid,
    pub quantity: f64,
}

/// Response with recipe cost calculation
#[derive(Debug, Serialize)]
pub struct RecipeCostResponse {
    pub recipe_id: Uuid,
    pub recipe_name: String,
    pub ingredients: Vec<IngredientCostResponse>,
    pub total_cost_cents: i64,
    pub cost_per_serving_cents: i64,
    pub servings: u32,
}

#[derive(Debug, Serialize)]
pub struct IngredientCostResponse {
    pub ingredient_id: Uuid,
    pub ingredient_name: String,
    pub quantity: f64,
    pub unit_price_cents: i64,
    pub total_cost_cents: i64,
}

/// POST /api/recipes - Create new recipe
pub async fn create_recipe(
    auth_user: AuthUser,
    State(recipe_service): State<RecipeService>,
    Json(payload): Json<CreateRecipeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = auth_user.user_id;
    let tenant_id = auth_user.tenant_id;

    // Parse and validate
    let name = RecipeName::new(payload.name)?;
    let servings = Servings::new(payload.servings)?;
    
    let ingredients: Vec<RecipeIngredient> = payload.ingredients
        .into_iter()
        .map(|ing| {
            let quantity = Quantity::new(ing.quantity)?;
            Ok(RecipeIngredient::new(
                CatalogIngredientId::from_uuid(ing.catalog_ingredient_id),
                quantity
            ))
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    // Create recipe
    let recipe = recipe_service
        .create_recipe(name, servings, ingredients, user_id, tenant_id)
        .await?;

    // Build response
    let response = RecipeResponse {
        id: recipe.id().as_uuid(),
        name: recipe.name().as_str().to_string(),
        servings: recipe.servings().count(),
        ingredients: recipe.ingredients()
            .iter()
            .map(|ing| RecipeIngredientResponse {
                catalog_ingredient_id: ing.catalog_ingredient_id().as_uuid(),
                quantity: ing.quantity().value(),
            })
            .collect(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/recipes/:id - Get recipe by ID
pub async fn get_recipe(
    auth_user: AuthUser,
    State(recipe_service): State<RecipeService>,
    Path(recipe_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = auth_user.user_id;
    let recipe_id = RecipeId::from_uuid(recipe_id);

    let recipe = recipe_service
        .get_recipe(recipe_id, user_id)
        .await?
        .ok_or(AppError::NotFound("Recipe not found".to_string()))?;

    let response = RecipeResponse {
        id: recipe.id().as_uuid(),
        name: recipe.name().as_str().to_string(),
        servings: recipe.servings().count(),
        ingredients: recipe.ingredients()
            .iter()
            .map(|ing| RecipeIngredientResponse {
                catalog_ingredient_id: ing.catalog_ingredient_id().as_uuid(),
                quantity: ing.quantity().value(),
            })
            .collect(),
    };

    Ok(Json(response))
}

/// GET /api/recipes - List all recipes for user
pub async fn list_recipes(
    auth_user: AuthUser,
    State(recipe_service): State<RecipeService>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = auth_user.user_id;

    let recipes = recipe_service.list_recipes(user_id).await?;

    let response: Vec<RecipeResponse> = recipes
        .into_iter()
        .map(|recipe| RecipeResponse {
            id: recipe.id().as_uuid(),
            name: recipe.name().as_str().to_string(),
            servings: recipe.servings().count(),
            ingredients: recipe.ingredients()
                .iter()
                .map(|ing| RecipeIngredientResponse {
                    catalog_ingredient_id: ing.catalog_ingredient_id().as_uuid(),
                    quantity: ing.quantity().value(),
                })
                .collect(),
        })
        .collect();

    Ok(Json(response))
}

/// DELETE /api/recipes/:id - Delete recipe
pub async fn delete_recipe(
    auth_user: AuthUser,
    State(recipe_service): State<RecipeService>,
    Path(recipe_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = auth_user.user_id;
    let recipe_id = RecipeId::from_uuid(recipe_id);

    let deleted = recipe_service.delete_recipe(recipe_id, user_id).await?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("Recipe not found".to_string()))
    }
}

/// GET /api/recipes/:id/cost - Calculate recipe cost
/// 
/// This is the KEY endpoint for cost analysis.
/// Returns detailed breakdown of recipe cost based on current inventory prices.
/// 
/// Response includes:
/// - total_cost_cents: Full cost of recipe
/// - cost_per_serving_cents: Cost per single serving
/// - ingredients breakdown with individual costs
/// 
/// Use case: Owner wants to know "How much does this dish cost me?"
pub async fn calculate_recipe_cost(
    auth_user: AuthUser,
    State(recipe_service): State<RecipeService>,
    Path(recipe_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = auth_user.user_id;
    let recipe_id = RecipeId::from_uuid(recipe_id);

    // Calculate cost using RecipeService
    let recipe_cost = recipe_service
        .calculate_cost(recipe_id, user_id)
        .await?;

    // Build response
    let response = RecipeCostResponse {
        recipe_id: recipe_cost.recipe_id.as_uuid(),
        recipe_name: recipe_cost.recipe_name.clone(),
        ingredients: recipe_cost.ingredients_breakdown
            .iter()
            .map(|ing| IngredientCostResponse {
                ingredient_id: ing.ingredient_id.as_uuid(),
                ingredient_name: ing.ingredient_name.clone(),
                quantity: ing.quantity.value(),
                unit_price_cents: ing.unit_price.as_cents(),
                total_cost_cents: ing.total_cost.as_cents(),
            })
            .collect(),
        total_cost_cents: recipe_cost.total_cost.as_cents(),
        cost_per_serving_cents: recipe_cost.cost_per_serving.as_cents(),
        servings: recipe_cost.servings,
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_request_deserializes() {
        let json = r#"{
            "name": "Tomato Soup",
            "servings": 4,
            "ingredients": [
                {
                    "catalog_ingredient_id": "a7bb61b5-17c3-45c2-9b97-f052a7818df3",
                    "quantity": 0.5
                }
            ]
        }"#;
        
        let request: CreateRecipeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Tomato Soup");
        assert_eq!(request.servings, 4);
        assert_eq!(request.ingredients.len(), 1);
    }

    #[test]
    fn test_recipe_cost_response_serializes() {
        let response = RecipeCostResponse {
            recipe_id: Uuid::new_v4(),
            recipe_name: "Test Recipe".to_string(),
            ingredients: vec![
                IngredientCostResponse {
                    ingredient_id: Uuid::new_v4(),
                    ingredient_name: "Tomatoes".to_string(),
                    quantity: 0.5,
                    unit_price_cents: 500,
                    total_cost_cents: 250,
                }
            ],
            total_cost_cents: 310,
            cost_per_serving_cents: 78,
            servings: 4,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("total_cost_cents"));
        assert!(json.contains("cost_per_serving_cents"));
    }
}
