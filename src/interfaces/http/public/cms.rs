use crate::application::cms_service::{ArticleQuery, CmsService};
use crate::shared::AppError;
use axum::{
    extract::{Path, Query, State},
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

// ── KNOWLEDGE ARTICLES (paginated + search) ───────────────────────────────────

pub async fn list_articles(
    Query(q): Query<ArticleQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = svc.list_articles_paged(&q).await?;
    Ok(Json(serde_json::to_value(result).unwrap()))
}

pub async fn get_article(
    Path(slug): Path<String>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = svc.get_article_by_slug(&slug).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

// ── SITEMAP ───────────────────────────────────────────────────────────────────

pub async fn articles_sitemap(
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.articles_sitemap().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

// ── STATS ─────────────────────────────────────────────────────────────────────

pub async fn public_stats(
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = svc.public_stats().await?;
    Ok(Json(serde_json::to_value(stats).unwrap()))
}

// ── ARTICLE CATEGORIES ────────────────────────────────────────────────────────

pub async fn list_article_categories(
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_categories().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}
