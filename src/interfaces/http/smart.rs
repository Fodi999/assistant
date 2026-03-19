//! HTTP handler for `POST /api/smart/ingredient`.

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::application::smart_service::{CulinaryContext, SmartResponse, SmartService};
use crate::shared::AppError;

/// POST /api/smart/ingredient
///
/// Request body: CulinaryContext (JSON)
/// Response: SmartResponse (JSON)
pub async fn smart_ingredient(
    State(service): State<Arc<SmartService>>,
    Json(ctx): Json<CulinaryContext>,
) -> Result<Json<SmartResponse>, AppError> {
    // Validate input
    if ctx.ingredient.is_empty() {
        return Err(AppError::validation("ingredient slug is required"));
    }
    if ctx.ingredient.len() > 100 {
        return Err(AppError::validation("ingredient slug too long"));
    }
    if ctx.additional_ingredients.len() > 20 {
        return Err(AppError::validation("too many additional ingredients (max 20)"));
    }

    let response = service.get_smart_ingredient(ctx).await?;
    Ok(Json(response))
}
