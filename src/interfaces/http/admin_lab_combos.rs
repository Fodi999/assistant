//! Admin HTTP handlers for Lab Combo SEO pages.

use crate::application::lab_combos::{
    GenerateComboRequest, LabComboPage, LabComboService, ListCombosQuery, UpdateComboRequest,
};
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
