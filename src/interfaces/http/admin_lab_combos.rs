//! Admin HTTP handlers for Lab Combo SEO pages.

use crate::application::lab_combos::{
    GenerateComboRequest, LabComboPage, LabComboService, ListCombosQuery, UpdateComboRequest,
};
use crate::application::lab_combos::dish_classifier::{classify_dish, DishProfile};
use crate::application::lab_combos::metrics;
use crate::domain::admin::AdminClaims;
use crate::shared::AppError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

/// POST /api/admin/lab-combos/generate
pub async fn generate_combo(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Json(req): Json<GenerateComboRequest>,
) -> Result<(StatusCode, Json<LabComboPage>), AppError> {
    let page = svc.generate(req).await?;
    Ok((StatusCode::CREATED, Json(page)))
}

/// POST /api/admin/lab-combos/generate-all-locales
/// Generate a combo for all 4 locales (en, pl, ru, uk) in one call.
pub async fn generate_all_locales(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Json(req): Json<GenerateComboRequest>,
) -> Result<(StatusCode, Json<Vec<LabComboPage>>), AppError> {
    let pages = svc.generate_all_locales(req).await?;
    Ok((StatusCode::CREATED, Json(pages)))
}

/// GET /api/admin/lab-combos
pub async fn list_combos(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Query(query): Query<ListCombosQuery>,
) -> Result<Json<Vec<LabComboPage>>, AppError> {
    let pages = svc.list(query).await?;
    Ok(Json(pages))
}

/// POST /api/admin/lab-combos/:id/publish
pub async fn publish_combo(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<LabComboPage>, AppError> {
    let page = svc.publish(id).await?;
    Ok(Json(page))
}

/// POST /api/admin/lab-combos/:id/archive
pub async fn archive_combo(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<LabComboPage>, AppError> {
    let page = svc.archive(id).await?;
    Ok(Json(page))
}

/// DELETE /api/admin/lab-combos/:id
pub async fn delete_combo(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    svc.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/admin/lab-combos/:id
pub async fn update_combo(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateComboRequest>,
) -> Result<Json<LabComboPage>, AppError> {
    let page = svc.update(id, req).await?;
    Ok(Json(page))
}

#[derive(Debug, Deserialize)]
pub struct GeneratePopularRequest {
    pub locale: String,
    pub limit: Option<usize>,
}

#[derive(serde::Serialize)]
pub struct GeneratePopularResponse {
    pub generated: usize,
    pub details: Vec<String>,
}

/// POST /api/admin/lab-combos/generate-popular
pub async fn generate_popular(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Json(req): Json<GeneratePopularRequest>,
) -> Result<Json<GeneratePopularResponse>, AppError> {
    let limit = req.limit.unwrap_or(14);
    let results = svc.generate_popular_combos(&req.locale, limit).await?;
    Ok(Json(GeneratePopularResponse {
        generated: results.len(),
        details: results,
    }))
}

// ── Image upload (presigned URL flow) ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ImageUploadUrlQuery {
    pub content_type: Option<String>,
}

/// GET /api/admin/lab-combos/:id/image-upload-url?content_type=image/webp
pub async fn get_image_upload_url(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Path(id): Path<Uuid>,
    Query(q): Query<ImageUploadUrlQuery>,
) -> Result<Json<crate::application::user::AvatarUploadResponse>, AppError> {
    let content_type = q.content_type.unwrap_or_else(|| "image/webp".to_string());
    let resp = svc.get_image_upload_url(id, &content_type).await?;
    Ok(Json(resp))
}

#[derive(Debug, Deserialize)]
pub struct SaveImageUrlRequest {
    pub image_url: String,
}

/// PUT /api/admin/lab-combos/:id/image-url
pub async fn save_image_url(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Path(id): Path<Uuid>,
    Json(req): Json<SaveImageUrlRequest>,
) -> Result<Json<LabComboPage>, AppError> {
    let page = svc.save_image_url(id, req.image_url).await?;
    Ok(Json(page))
}

/// GET /api/admin/lab-combos/:id/image-upload-url/:kind?content_type=image/webp
/// kind = "hero" | "process" | "detail"
pub async fn get_typed_image_upload_url(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Path((id, kind)): Path<(Uuid, String)>,
    Query(q): Query<ImageUploadUrlQuery>,
) -> Result<Json<crate::application::user::AvatarUploadResponse>, AppError> {
    let content_type = q.content_type.unwrap_or_else(|| "image/webp".to_string());
    let resp = svc.get_typed_image_upload_url(id, &kind, &content_type).await?;
    Ok(Json(resp))
}

/// PUT /api/admin/lab-combos/:id/image-url/:kind
/// kind = "hero" | "process" | "detail"
pub async fn save_typed_image_url(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
    Path((id, kind)): Path<(Uuid, String)>,
    Json(req): Json<SaveImageUrlRequest>,
) -> Result<Json<LabComboPage>, AppError> {
    let page = svc.save_typed_image_url(id, &kind, req.image_url).await?;
    Ok(Json(page))
}

/// POST /api/admin/lab-combos/backfill-ingredients
/// Populate structured_ingredients for all existing records that have [].
pub async fn backfill_ingredients(
    _claims: AdminClaims,
    State(svc): State<Arc<LabComboService>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = svc.backfill_structured_ingredients().await?;
    Ok(Json(serde_json::json!({
        "updated": count,
        "message": format!("Backfilled structured_ingredients for {} records", count)
    })))
}

// ── Dish Classification Preview ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ClassifyDishRequest {
    pub dish_name: String,
}

/// POST /api/admin/lab-combos/classify-dish
/// Returns DishProfile for a given dish name — instant, no DB/AI.
/// Used by the admin UI to show a preview of the dish type classification.
pub async fn classify_dish_handler(
    _claims: AdminClaims,
    Json(req): Json<ClassifyDishRequest>,
) -> Result<Json<DishProfile>, AppError> {
    if req.dish_name.trim().is_empty() {
        return Err(AppError::validation("dish_name must not be empty"));
    }
    let profile = classify_dish(req.dish_name.trim());
    Ok(Json(profile))
}

// ── Pipeline Metrics ─────────────────────────────────────────────────────────

/// GET /api/admin/lab-combos/metrics
/// Returns pipeline observability metrics (generation counts, latency, etc.)
pub async fn pipeline_metrics(
    _claims: AdminClaims,
) -> Json<metrics::MetricsSnapshot> {
    Json(metrics::global_metrics().snapshot())
}
