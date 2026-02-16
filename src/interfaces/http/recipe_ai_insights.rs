// Recipe AI Insights HTTP Handlers - AI-generated recipe analysis
use crate::application::RecipeAIInsightsService;
use crate::domain::recipe_v2::RecipeId;
use crate::domain::RecipeAIInsightsResponse;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

/// GET /api/recipes/v2/:id/insights/:language - Get or generate AI insights
/// Path params: recipe_id (UUID), language (en/ru/pl/uk)
/// Returns: RecipeAIInsightsResponse (with generation time if newly created)
pub async fn get_or_generate_insights(
    State(ai_service): State<Arc<RecipeAIInsightsService>>,
    AuthUser { user_id: _, tenant_id, language: _ }: AuthUser,
    Path((recipe_id, target_language)): Path<(Uuid, String)>,
) -> AppResult<Json<RecipeAIInsightsResponse>> {
    let response = ai_service
        .get_or_generate_insights_by_id(RecipeId(recipe_id), tenant_id, &target_language)
        .await?;
    
    Ok(Json(response))
}

/// POST /api/recipes/v2/:id/insights/:language/refresh - Force regenerate AI insights
/// Path params: recipe_id (UUID), language (en/ru/pl/uk)
/// Returns: RecipeAIInsightsResponse with generation_time_ms
pub async fn refresh_insights(
    State(ai_service): State<Arc<RecipeAIInsightsService>>,
    AuthUser { user_id: _, tenant_id, language: _ }: AuthUser,
    Path((recipe_id, target_language)): Path<(Uuid, String)>,
) -> AppResult<(StatusCode, Json<RecipeAIInsightsResponse>)> {
    let response = ai_service
        .refresh_insights_by_id(RecipeId(recipe_id), tenant_id, &target_language)
        .await?;
    
    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/recipes/v2/:id/insights - Get all language insights for a recipe
/// Path param: recipe_id (UUID)
/// Returns: Vec<RecipeAIInsightsResponse>
pub async fn get_all_insights(
    State(ai_service): State<Arc<RecipeAIInsightsService>>,
    AuthUser { user_id: _, tenant_id: _, language: _ }: AuthUser,
    Path(recipe_id): Path<Uuid>,
) -> AppResult<Json<Vec<RecipeAIInsightsResponse>>> {
    let insights = ai_service.get_all_insights(recipe_id).await?;
    
    // Convert to response DTOs
    let responses: Vec<RecipeAIInsightsResponse> = insights
        .into_iter()
        .map(|insight| RecipeAIInsightsResponse {
            insights: insight,
            generated_in_ms: 0,  // Historical data, no generation time
        })
        .collect();
    
    Ok(Json(responses))
}
