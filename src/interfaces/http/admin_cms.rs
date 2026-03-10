use crate::application::cms_service::{
    CmsService, CreateArticleRequest, CreateExperienceRequest, CreateExpertiseRequest,
    CreateGalleryRequest, UpdateAboutRequest, UpdateArticleRequest, UpdateExperienceRequest,
    UpdateExpertiseRequest, UpdateGalleryRequest,
};
use crate::domain::AdminClaims;
use crate::shared::AppError;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

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
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_gallery().await?;
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
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_articles_admin().await?;
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
