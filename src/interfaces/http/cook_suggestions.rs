//! HTTP handler for POST /api/cook/suggestions — Smart recipe suggestions from inventory.
//!
//! Request: (no body needed — uses auth user's inventory)
//!
//! Response:
//!   {
//!     "can_cook": [...],
//!     "almost": [...],
//!     "strategic": [...]
//!   }

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::cook_suggestions::{CookSuggestionService, CookSuggestionsResponse};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

/// POST /api/cook/suggestions
pub async fn cook_suggestions(
    State(service): State<Arc<CookSuggestionService>>,
    auth: AuthUser,
) -> Result<Json<CookSuggestionsResponse>, AppError> {
    let result = service
        .suggest(auth.user_id, auth.tenant_id, auth.language)
        .await?;
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct DishImageRequest {
    pub dish_name: String,
    /// Top ingredient names (max 5 recommended)
    pub ingredients: Vec<String>,
}

#[derive(Serialize)]
pub struct DishImageResponse {
    /// data:image/png;base64,... or data:image/jpeg;base64,...
    pub dish_image_url: String,
}

/// POST /api/cook/suggestions/dish-image
/// On-demand AI photo generation for a suggested dish (called lazily on card expand).
pub async fn generate_dish_image(
    State(service): State<Arc<CookSuggestionService>>,
    _auth: AuthUser,
    Json(body): Json<DishImageRequest>,
) -> Result<Json<DishImageResponse>, AppError> {
    if body.dish_name.trim().is_empty() {
        return Err(AppError::validation("dish_name is required"));
    }
    let top_ingredients = body.ingredients.into_iter().take(5).collect();
    let dish_image_url = service
        .generate_dish_image(&body.dish_name, top_ingredients)
        .await?;
    Ok(Json(DishImageResponse {
        dish_image_url: format!("data:image/png;base64,{}", dish_image_url),
    }))
}
