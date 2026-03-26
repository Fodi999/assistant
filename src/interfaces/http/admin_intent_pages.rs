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
//! POST   /api/admin/intent-pages/publish-bulk      → bulk publish
//! POST   /api/admin/intent-pages/archive-bulk      → bulk archive
//! POST   /api/admin/intent-pages/delete-bulk       → bulk delete
//! GET    /api/admin/intent-pages/duplicates        → find duplicate slugs
//! POST   /api/admin/intent-pages/cleanup-slugs     → fix & remove duplicate slugs
//! DELETE /api/admin/intent-pages/:id               → delete

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::intent_pages::{
    BatchGenerateRequest, BatchResult, BulkActionRequest, EnqueueBulkRequest,
    GenerateRequest, ImageUploadResponse, IntentPage, IntentPagesService, ListQuery,
    SeoAuditResult, SitemapEntry,
    UpdateIntentPageRequest, UpdateSettingsRequest,
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

/// POST /api/admin/intent-pages/:id/regenerate
/// Clears AI cache + deletes page + generates fresh with new prompt
pub async fn regenerate_intent_page(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IntentPage>, AppError> {
    let page = service.regenerate(id).await?;
    Ok(Json(page))
}

/// POST /api/admin/intent-pages/publish-bulk
pub async fn publish_bulk(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Json(req): Json<BulkActionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.bulk_publish(&req.ids).await?;
    Ok(Json(result))
}

/// POST /api/admin/intent-pages/archive-bulk
pub async fn archive_bulk(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Json(req): Json<BulkActionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.bulk_archive(&req.ids).await?;
    Ok(Json(result))
}

/// POST /api/admin/intent-pages/delete-bulk
pub async fn delete_bulk(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Json(req): Json<BulkActionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.bulk_delete(&req.ids).await?;
    Ok(Json(result))
}

/// GET /api/admin/intent-pages/duplicates
pub async fn find_duplicates(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
) -> Result<Json<Vec<crate::application::intent_pages::DuplicateGroup>>, AppError> {
    let groups = service.find_duplicates().await?;
    Ok(Json(groups))
}

/// POST /api/admin/intent-pages/cleanup-slugs
pub async fn cleanup_slugs(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.cleanup_duplicate_slugs().await?;
    Ok(Json(result))
}

/// PUT /api/admin/intent-pages/google-discovered
/// Body: { "count": 7192 }
/// Updates the Google Search Console baseline used for indexing gap KPI.
pub async fn set_google_discovered(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = body
        .get("count")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AppError::validation("Body must be { \"count\": <number> }"))?;
    let result = service.set_google_discovered(count).await?;
    Ok(Json(result))
}

// ── Image upload for intent-page content blocks ──────────────────────────────

#[derive(serde::Deserialize)]
pub struct ImageUploadQuery {
    pub content_type: String,
}

/// GET /api/admin/intent-pages/:id/images/:key/upload-url?content_type=image/webp
/// Returns { upload_url, public_url } for direct R2 upload.
pub async fn get_image_upload_url(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path((id, key)): Path<(Uuid, String)>,
    Query(q): Query<ImageUploadQuery>,
) -> Result<Json<ImageUploadResponse>, AppError> {
    let resp = service.get_image_upload_url(id, &key, &q.content_type).await?;
    Ok(Json(resp))
}

#[derive(serde::Deserialize)]
pub struct SaveImageRequest {
    pub image_url: String,
}

/// POST /api/admin/intent-pages/:id/images/:key
/// Body: { "image_url": "https://..." }
/// Patches content_blocks to set src on the matching image block.
pub async fn save_image_url(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
    Path((id, key)): Path<(Uuid, String)>,
    Json(req): Json<SaveImageRequest>,
) -> Result<Json<IntentPage>, AppError> {
    let page = service.save_image_url(id, &key, req.image_url).await?;
    Ok(Json(page))
}

// ── SEO Quality Control ──────────────────────────────────────────────────────

/// GET /api/admin/intent-pages/seo-audit
/// Returns SEO quality audit for ALL published pages.
pub async fn seo_audit(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
) -> Result<Json<Vec<SeoAuditResult>>, AppError> {
    let results = service.seo_audit().await?;
    Ok(Json(results))
}

/// POST /api/admin/intent-pages/cleanup-quality
/// Unpublish all pages with SEO score < 3/5. Returns cleanup report.
pub async fn cleanup_low_quality(
    _claims: AdminClaims,
    State(service): State<IntentPagesState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.cleanup_low_quality().await?;
    Ok(Json(result))
}

// ── Sitemap (lightweight, for Next.js) ───────────────────────────────────────

/// GET /public/intent-pages/sitemap
/// Returns slug + locale + published_at for ALL published intent pages.
/// No auth required — used by Next.js sitemap.xml generator.
pub async fn intent_pages_sitemap(
    State(service): State<IntentPagesState>,
) -> Result<Json<Vec<SitemapEntry>>, AppError> {
    let entries = service.sitemap().await?;
    Ok(Json(entries))
}