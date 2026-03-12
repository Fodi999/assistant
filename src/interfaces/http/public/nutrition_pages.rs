//! Public nutrition SEO handlers
//!
//! GET /public/nutrition/:slug           — full nutrition card
//! GET /public/diet/:flag                — products for a diet
//! GET /public/ranking/:metric           — top products by metric

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::application::public_nutrition::PublicNutritionService;

// ── Shared state ─────────────────────────────────────────────────────────────

pub type NutritionState = Arc<PublicNutritionService>;

// ── Query params ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LangQuery {
    pub lang: Option<String>,
}

#[derive(Deserialize)]
pub struct DietQuery {
    pub lang:  Option<String>,
    #[serde(rename = "type")]
    pub product_type: Option<String>,
    pub limit:  Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct RankingQuery {
    pub lang:   Option<String>,
    #[serde(rename = "type")]
    pub product_type: Option<String>,
    pub order:  Option<String>,
    pub limit:  Option<i64>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /public/nutrition/:slug
/// Returns full nutrition card for a product: macros, vitamins, minerals, diet flags, pairings.
pub async fn get_nutrition_page(
    State(svc): State<NutritionState>,
    Path(slug): Path<String>,
    Query(q): Query<LangQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let lang = q.lang.unwrap_or_else(|| "en".to_string());

    match svc.get_nutrition_page(&slug).await {
        Ok(mut page) => {
            page.lang = lang;
            Ok(Json(serde_json::to_value(page).unwrap()))
        }
        Err(crate::shared::AppError::NotFound(msg)) => {
            Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": msg })),
            ))
        }
        Err(e) => {
            tracing::error!("nutrition page error: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal error" })),
            ))
        }
    }
}

/// GET /public/diet/:flag?lang=ru&type=vegetable&limit=50&offset=0
/// Returns all products matching the given diet flag.
///
/// Valid flags: vegan, vegetarian, keto, paleo, gluten-free, mediterranean, low-carb
pub async fn get_diet_page(
    State(svc): State<NutritionState>,
    Path(flag): Path<String>,
    Query(q): Query<DietQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let lang   = q.lang.unwrap_or_else(|| "en".to_string());
    let limit  = q.limit.unwrap_or(50).min(200);
    let offset = q.offset.unwrap_or(0).max(0);
    let pt     = q.product_type.as_deref();

    match svc.get_diet_page(&flag, pt, limit, offset).await {
        Ok(mut page) => {
            page.lang = lang;
            Ok(Json(serde_json::to_value(page).unwrap()))
        }
        Err(crate::shared::AppError::Validation(msg)) => {
            Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": msg })),
            ))
        }
        Err(e) => {
            tracing::error!("diet page error: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal error" })),
            ))
        }
    }
}

/// GET /public/ranking/:metric?lang=ru&type=seafood&order=desc&limit=20
/// Returns ranked products by a nutritional metric.
///
/// Valid metrics: calories, protein, fat, carbs, fiber, sugar,
///                vitamin-c, vitamin-d, vitamin-b12,
///                iron, calcium, potassium, magnesium, zinc, sodium
pub async fn get_ranking_page(
    State(svc): State<NutritionState>,
    Path(metric): Path<String>,
    Query(q): Query<RankingQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let lang  = q.lang.unwrap_or_else(|| "en".to_string());
    let order = q.order.unwrap_or_else(|| "desc".to_string());
    let limit = q.limit.unwrap_or(20).min(111);
    let pt    = q.product_type.as_deref();

    match svc.get_ranking_page(&metric, pt, &order, limit).await {
        Ok(mut page) => {
            page.lang = lang;
            Ok(Json(serde_json::to_value(page).unwrap()))
        }
        Err(crate::shared::AppError::Validation(msg)) => {
            Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": msg })),
            ))
        }
        Err(e) => {
            tracing::error!("ranking page error: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal error" })),
            ))
        }
    }
}

/// GET /public/products-slugs
/// Returns an array of all product slugs — used by Next.js for SSG (generateStaticParams).
pub async fn get_all_slugs(
    State(svc): State<NutritionState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match svc.get_all_slugs().await {
        Ok(slugs) => Ok(Json(serde_json::json!(slugs))),
        Err(e) => {
            tracing::error!("get_all_slugs error: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal error" })),
            ))
        }
    }
}