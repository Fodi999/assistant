use crate::application::cms_service::{
    CmsService, CreateArticleRequest, CreateExperienceRequest, CreateExpertiseRequest,
    CreateGalleryRequest, UpdateAboutRequest, UpdateArticleRequest, UpdateExperienceRequest,
    UpdateExpertiseRequest, UpdateGalleryRequest,
};
use crate::domain::AdminClaims;
use crate::shared::AppError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

// ── QUERY PARAMS ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ArticleFilterQuery {
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct GalleryFilterQuery {
    pub category: Option<String>,
}

// ── ABOUT PAGE ────────────────────────────────────────────────────────────────

pub async fn get_about(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.get_about().await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn update_about(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<UpdateAboutRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.update_about(req).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

// ── EXPERTISE ─────────────────────────────────────────────────────────────────

pub async fn list_expertise(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_expertise().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn create_expertise(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<CreateExpertiseRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let row = svc.create_expertise(req).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(row).unwrap())))
}

pub async fn update_expertise(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
    Json(req): Json<UpdateExpertiseRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.update_expertise(id, req).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn delete_expertise(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
) -> Result<StatusCode, AppError> {
    svc.delete_expertise(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── EXPERIENCE ────────────────────────────────────────────────────────────────

pub async fn list_experience(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_experience().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn create_experience(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<CreateExperienceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let row = svc.create_experience(req).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(row).unwrap())))
}

pub async fn update_experience(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
    Json(req): Json<UpdateExperienceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.update_experience(id, req).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn delete_experience(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
) -> Result<StatusCode, AppError> {
    svc.delete_experience(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── GALLERY ───────────────────────────────────────────────────────────────────

pub async fn list_gallery(
    _claims: AdminClaims,
    Query(q): Query<GalleryFilterQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_gallery(q.category.as_deref()).await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn create_gallery(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<CreateGalleryRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let row = svc.create_gallery(req).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(row).unwrap())))
}

pub async fn update_gallery(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
    Json(req): Json<UpdateGalleryRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.update_gallery(id, req).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn delete_gallery(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
) -> Result<StatusCode, AppError> {
    svc.delete_gallery(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── KNOWLEDGE ARTICLES ────────────────────────────────────────────────────────

pub async fn list_articles(
    _claims: AdminClaims,
    Query(q): Query<ArticleFilterQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_articles_admin(q.category.as_deref()).await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn get_article(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.get_article_by_id(id).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn create_article(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<CreateArticleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let row = svc.create_article(req).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(row).unwrap())))
}

pub async fn update_article(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
    Json(req): Json<UpdateArticleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.update_article(id, req).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn delete_article(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(svc): State<CmsService>,
) -> Result<StatusCode, AppError> {
    svc.delete_article(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── IMAGE UPLOAD ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UploadQuery {
    pub folder:       Option<String>,
    pub content_type: Option<String>,
}

/// GET /api/admin/cms/upload-url?folder=gallery&content_type=image/webp
/// Returns presigned R2 upload URL + final public URL
pub async fn get_upload_url(
    _claims: AdminClaims,
    Query(q): Query<UploadQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let folder       = q.folder.unwrap_or_else(|| "general".to_string());
    let content_type = q.content_type.unwrap_or_else(|| "image/webp".to_string());
    let resp = svc.get_image_upload_url(&folder, &content_type).await?;
    Ok(Json(serde_json::json!({
        "upload_url": resp.upload_url,
        "url":        resp.public_url,
    })))
}

// ── CATEGORIES (admin read) ───────────────────────────────────────────────────

pub async fn list_article_categories(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_categories().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}
