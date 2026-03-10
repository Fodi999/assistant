use crate::application::cms_service::CmsService;
use crate::shared::AppError;
use axum::{
    extract::{Path, State},
    Json,
};

// ── ABOUT PAGE ────────────────────────────────────────────────────────────────

pub async fn get_about(
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.get_about().await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

// ── EXPERTISE ─────────────────────────────────────────────────────────────────

pub async fn list_expertise(
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_expertise().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

// ── EXPERIENCE ────────────────────────────────────────────────────────────────

pub async fn list_experience(
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_experience().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

// ── GALLERY ───────────────────────────────────────────────────────────────────

pub async fn list_gallery(
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_gallery().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

// ── KNOWLEDGE ARTICLES ────────────────────────────────────────────────────────

pub async fn list_articles(
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_articles_public().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn get_article(
    Path(slug): Path<String>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.get_article_by_slug(&slug).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}
