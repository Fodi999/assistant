use crate::application::catalog_rule_bot::CatalogRuleBotService;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

/// POST /api/admin/catalog/states/generate/:ingredient_id
/// Generate all 10 processing states for one ingredient
pub async fn generate_states(
    Path(ingredient_id): Path<Uuid>,
    State(service): State<CatalogRuleBotService>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    match service.generate_states_for(ingredient_id).await {
        Ok(result) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "ingredient_id": result.ingredient_id,
                "name_en": result.name_en,
                "states_created": result.states_created,
                "states_total": result.states_total,
            })),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        )),
    }
}

/// POST /api/admin/catalog/states/generate-all
/// Generate missing states for ALL ingredients in catalog
pub async fn generate_all_states(
    State(service): State<CatalogRuleBotService>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    match service.generate_all_states().await {
        Ok(result) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "total_ingredients": result.total_ingredients,
                "ingredients_processed": result.ingredients_processed,
                "states_created": result.states_created,
                "errors": result.errors,
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        )),
    }
}

/// GET /api/admin/catalog/states/audit
/// Return full state coverage audit across the catalog
pub async fn state_audit(
    State(service): State<CatalogRuleBotService>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match service.state_audit().await {
        Ok(audit) => Ok(Json(serde_json::json!({
            "ok": true,
            "total_ingredients": audit.total_ingredients,
            "ingredients_with_all_states": audit.ingredients_with_all_states,
            "ingredients_missing_states": audit.ingredients_missing_states,
            "total_state_records": audit.total_state_records,
            "expected_state_records": audit.expected_state_records,
            "coverage_percent": audit.coverage_percent,
            "details": audit.details,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        )),
    }
}

/// GET /api/admin/catalog/states/data-quality
/// Return data quality/completeness scores for all products
pub async fn data_quality(
    State(service): State<CatalogRuleBotService>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match service.data_quality().await {
        Ok(rows) => {
            let avg_score = if rows.is_empty() {
                0.0
            } else {
                rows.iter().map(|r| r.score).sum::<f64>() / rows.len() as f64
            };
            Ok(Json(serde_json::json!({
                "ok": true,
                "total": rows.len(),
                "avg_score": (avg_score * 10.0).round() / 10.0,
                "products": rows,
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        )),
    }
}

/// GET /api/admin/catalog/states/products/:id
/// Return all states for a specific ingredient
pub async fn get_product_states(
    Path(ingredient_id): Path<Uuid>,
    State(service): State<CatalogRuleBotService>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match service.get_states(ingredient_id).await {
        Ok(states) => Ok(Json(serde_json::json!({
            "ok": true,
            "ingredient_id": ingredient_id,
            "count": states.len(),
            "states": states,
        }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        )),
    }
}

/// DELETE /api/admin/catalog/states/products/:id
/// Delete all states for a specific ingredient (allows regeneration)
pub async fn delete_product_states(
    Path(ingredient_id): Path<Uuid>,
    State(service): State<CatalogRuleBotService>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pool = service.pool();
    match sqlx::query("DELETE FROM ingredient_states WHERE ingredient_id = $1")
        .bind(ingredient_id)
        .execute(pool)
        .await
    {
        Ok(result) => Ok(Json(serde_json::json!({
            "ok": true,
            "ingredient_id": ingredient_id,
            "deleted": result.rows_affected(),
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        )),
    }
}
