use crate::application::cms_service::{
    CmsService, CreateAiArticleDraftRequest, CreateAiShopProductDraftRequest, CreateArticleRequest,
    CreateExperienceRequest, CreateExpertiseRequest, CreateGalleryRequest,
    CreateShopProductRequest, GenerateAiArticleImagesRequest, UpdateAboutRequest,
    UpdateArticleRequest, UpdateExperienceRequest, UpdateExpertiseRequest, UpdateGalleryRequest,
    UpdateShopProductStatusRequest,
};
use crate::domain::AdminClaims;
use crate::shared::AppError;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use super::site_context::{resolve_site_id, SiteQuery, KITCHEN_SITE_ID};

// ── QUERY PARAMS ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ArticleFilterQuery {
    pub category: Option<String>,
    pub site_id: Option<Uuid>,
    pub site: Option<String>,
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
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(row).unwrap()),
    ))
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
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(row).unwrap()),
    ))
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

pub async fn list_gallery_categories(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_gallery_categories().await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn list_gallery(
    _claims: AdminClaims,
    Query(q): Query<GalleryFilterQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = svc.list_gallery(q.category.as_deref(), false).await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn create_gallery(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<CreateGalleryRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let row = svc.create_gallery(req).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(row).unwrap()),
    ))
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

pub async fn create_ai_article_draft(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<CreateAiArticleDraftRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let draft = svc
        .create_ai_article_draft(&req.topic, req.target_chars, req.image_count)
        .await?;
    Ok(Json(serde_json::to_value(draft).unwrap()))
}

pub async fn generate_ai_article_image(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<GenerateAiArticleImagesRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let image = svc
        .generate_ai_article_image(
            &req.title,
            req.prompt.as_deref(),
            req.index,
            req.enhanced,
            &req.reference_urls,
            req.model_preset.as_deref(),
            req.scene_preset.as_deref(),
            req.width_cm,
            req.height_cm,
            req.depth_cm,
            req.weight_kg,
            &req.photo_scenarios,
            req.scale_reference.as_deref(),
            req.custom_scale_reference.as_deref(),
        )
        .await?;
    Ok(Json(serde_json::to_value(image).unwrap()))
}

pub async fn create_ai_shop_product_draft(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    Json(req): Json<CreateAiShopProductDraftRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let draft = svc
        .create_ai_shop_product_draft(&req.product, req.image_count)
        .await?;
    Ok(Json(serde_json::to_value(draft).unwrap()))
}

pub async fn list_shop_products(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let rows = svc.list_shop_products_for_site(site_id).await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn create_shop_product(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(svc): State<CmsService>,
    Json(req): Json<CreateShopProductRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let row = svc.create_shop_product_for_site(req, site_id).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(row).unwrap()),
    ))
}

pub async fn update_shop_product_status(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(svc): State<CmsService>,
    Json(req): Json<UpdateShopProductStatusRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let row = svc
        .update_shop_product_status_for_site(id, site_id, &req.status)
        .await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn delete_shop_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(svc): State<CmsService>,
) -> Result<StatusCode, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    svc.delete_shop_product_for_site(id, site_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_articles(
    _claims: AdminClaims,
    Query(q): Query<ArticleFilterQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let site_query = SiteQuery {
        site_id: q.site_id,
        site: q.site,
    };
    let site_id = resolve_site_id(&site_query, KITCHEN_SITE_ID);
    let rows = svc
        .list_articles_admin_for_site(site_id, q.category.as_deref())
        .await?;
    Ok(Json(serde_json::to_value(rows).unwrap()))
}

pub async fn get_article(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let row = svc.get_article_by_id_for_site(id, site_id).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn create_article(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(svc): State<CmsService>,
    Json(req): Json<CreateArticleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let row = svc.create_article_for_site(req, site_id).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(row).unwrap()),
    ))
}

pub async fn update_article(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(svc): State<CmsService>,
    Json(req): Json<UpdateArticleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let row = svc.update_article_for_site(id, site_id, req).await?;
    Ok(Json(serde_json::to_value(row).unwrap()))
}

pub async fn delete_article(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(svc): State<CmsService>,
) -> Result<StatusCode, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    svc.delete_article_for_site(id, site_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── IMAGE UPLOAD ──────────────────────────────────────────────────────────────

pub async fn upload_article_reference(
    _claims: AdminClaims,
    State(svc): State<CmsService>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(format!("Invalid multipart data: {}", e)))?
        .ok_or_else(|| AppError::validation("No reference image provided"))?;
    let content_type = field
        .content_type()
        .map(str::to_string)
        .ok_or_else(|| AppError::validation("Reference content type is missing"))?;
    let bytes = field
        .bytes()
        .await
        .map_err(|e| {
            tracing::warn!("Failed to read CMS reference image multipart field: {e}");
            AppError::validation(
                "Failed to read reference image. Make sure the file is a valid image smaller than 10 MB",
            )
        })?;
    let url = svc.upload_article_reference(bytes, &content_type).await?;
    Ok(Json(serde_json::json!({ "url": url })))
}

#[derive(Deserialize)]
pub struct UploadQuery {
    pub folder: Option<String>,
    pub content_type: Option<String>,
}

/// GET /api/admin/cms/upload-url?folder=gallery&content_type=image/webp
/// Returns presigned R2 upload URL + final public URL
pub async fn get_upload_url(
    _claims: AdminClaims,
    Query(q): Query<UploadQuery>,
    State(svc): State<CmsService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let folder = q.folder.unwrap_or_else(|| "general".to_string());
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
