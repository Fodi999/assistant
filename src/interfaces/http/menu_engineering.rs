use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::application::MenuEngineeringService;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::{AppError, Language};

/// Query parameters for menu analysis
#[derive(Debug, Deserialize)]
pub struct AnalysisQuery {
    /// Period in days (default: 30)
    #[serde(default = "default_period")]
    period_days: u32,
    
    /// Language for recommendations (default: en)
    #[serde(default)]
    language: Language,
}

fn default_period() -> u32 {
    30
}

/// GET /api/menu-engineering/analysis
/// 
/// Returns Menu Engineering Matrix with all dishes classified
pub async fn analyze_menu(
    State(service): State<MenuEngineeringService>,
    AuthUser { user_id, tenant_id }: AuthUser,
    Query(params): Query<AnalysisQuery>,
) -> Result<impl IntoResponse, AppError> {
    let matrix = service
        .analyze_menu(user_id, tenant_id, params.language, params.period_days)
        .await?;

    Ok(Json(matrix))
}

/// POST /api/menu-engineering/sales
/// 
/// Record a dish sale (normally called from POS/order system)
#[derive(Debug, Deserialize)]
pub struct RecordSaleRequest {
    pub dish_id: uuid::Uuid,
    pub quantity: u32,
    pub selling_price_cents: i32,
    pub recipe_cost_cents: i32,
}

pub async fn record_sale(
    State(service): State<MenuEngineeringService>,
    AuthUser { user_id, tenant_id, .. }: AuthUser,
    Json(payload): Json<RecordSaleRequest>,
) -> Result<impl IntoResponse, AppError> {
    service
        .record_sale(
            tenant_id,
            payload.dish_id,
            user_id,
            payload.quantity,
            payload.selling_price_cents,
            payload.recipe_cost_cents,
        )
        .await?;

    Ok(StatusCode::CREATED)
}
