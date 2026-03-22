//! Admin Intent Pages handlers
//!
//! POST   /api/admin/intent-pages/generate         → generate single
//! POST   /api/admin/intent-pages/generate-batch    → batch generate
//! GET    /api/admin/intent-pages                   → list (filter)
//! GET    /api/admin/intent-pages/stats             → counts per status
//! GET    /api/admin/intent-pages/settings          → get publish limit
//! PUT    /api/admin/intent-pages/settings          → set publish limit
//! POST   /api/admin/intent-pages/scheduler/run     → trigger scheduler
//! GET    /api/admin/intent-pages/:id               → get by id
//! PUT    /api/admin/intent-pages/:id               → update content
//! POST   /api/admin/intent-pages/:id/publish       → publish now
//! POST   /api/admin/intent-pages/:id/unpublish     → unpublish
//! POST   /api/admin/intent-pages/:id/enqueue       → draft → queued
//! POST   /api/admin/intent-pages/:id/archive       → any → archived
//! POST   /api/admin/intent-pages/enqueue-bulk      → bulk enqueue
//! DELETE /api/admin/intent-pages/:id               → delete

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::intent_pages::{
    BatchGenerateRequest, BatchResult, EnqueueBulkRequest, GenerateRequest,
    IntentPage, IntentPagesService, ListQuery, UpdateIntentPageRequest,
    UpdateSettingsRequest,
};
use crate::domain::AdminClaims;
use crate::shared::AppError;

pub type IntentPagesState = Arc<IntentPagesService>;

/// POST /api/admin/intent-pages/generate
pub async fn generate_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Json(req): Json<GenerateRequest>,
) -> Result<(StatusCode, Json<IntentPage>), AppError> {
    let page = service.generate(&req).await?;
    Ok((StatusCode::CREATED, Json(page)))
}

/// POST /api/admin/intent-pages/generate-batch
pub async fn generate_batch(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Json(req): Json<BatchGenerateRequest>,
) -> Result<Json<BatchResult>, AppError> {
    let result = service.generate_batch(&req).await?;
    Ok(Json(result))
}

/// GET /api/admin/intent-pages
pub async fn list_intent_pages(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<IntentPage>>, AppError> {
    let pages = service.list(&q).await?;
    Ok(Json(pages))
}

/// GET /api/admin/intent-pages/stats
pub async fn intent_pages_stats(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = service.stats().await?;
    Ok(Json(stats))
}

/// GET /api/admin/intent-pages/settings
pub async fn get_settings(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = service.get_settings().await?;
    Ok(Json(settings))
}

/// PUT /api/admin/intent-pages/settings
pub async fn update_settings(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = service.update_settings(req.publish_limit_per_day).await?;
    Ok(Json(settings))
}

/// POST /api/admin/intent-pages/scheduler/run
pub async fn run_scheduler(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.run_scheduled_publish().await?;
    Ok(Json(result))
}

/// GET /api/admin/intent-pages/:id
pub async fn get_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IntentPage>, AppError> {
    let page = service.get_by_id(id).await?;
    Ok(Json(page))
}

/// PUT /api/admin/intent-pages/:id
pub async fn update_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateIntentPageRequest>,
) -> Result<Json<IntentPage>, AppError> {
    let page = service.update(id, &req).await?;
    Ok(Json(page))
}

/// POST /api/admin/intent-pages/:id/publish
pub async fn publish_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IntentPage>, AppError> {
    let page = service.publish(id).await?;
    Ok(Json(page))
}

/// POST /api/admin/intent-pages/:id/unpublish
pub async fn unpublish_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IntentPage>, AppError> {
    let page = service.unpublish(id).await?;
    Ok(Json(page))
}

/// POST /api/admin/intent-pages/:id/enqueue
pub async fn enqueue_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IntentPage>, AppError> {
    let page = service.enqueue(id).await?;
    Ok(Json(page))
}

/// POST /api/admin/intent-pages/:id/archive
pub async fn archive_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IntentPage>, AppError> {
    let page = service.archive(id).await?;
    Ok(Json(page))
}

/// POST /api/admin/intent-pages/enqueue-bulk
pub async fn enqueue_bulk(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Json(req): Json<EnqueueBulkRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let priority = req.priority.unwrap_or(1).clamp(0, 2);
    let result = service.enqueue_bulk(&req.ids, priority).await?;
    Ok(Json(result))
}

/// DELETE /api/admin/intent-pages/:id
pub async fn delete_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    service.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
