use crate::application::{
    AdminCatalogService, CategoryResponse, CreateCategoryRequest, CreateProductRequest,
    ProductResponse, UpdateCategoryRequest, UpdateProductRequest,
};
use crate::domain::AdminClaims;
use crate::shared::AppError;
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

/// Image URL Response
#[derive(Debug, Serialize)]
pub struct ImageUrlResponse {
    pub image_url: String,
}

/// List all products
pub async fn list_products(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
) -> Result<Json<Vec<ProductResponse>>, AppError> {
    let products = service.list_products().await?;
    Ok(Json(products))
}

/// Get product by ID
pub async fn get_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = service.get_product_by_id(id).await?;
    Ok(Json(product))
}

/// Create new product
pub async fn create_product(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
    Json(req): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>), AppError> {
    let product = service.create_product(req).await?;
    Ok((StatusCode::CREATED, Json(product)))
}

/// Update product
pub async fn update_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = service.update_product(id, req).await?;
    Ok(Json(product))
}

/// Delete product
pub async fn delete_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<StatusCode, AppError> {
    service.delete_product(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Upload product image (multipart)
pub async fn upload_product_image(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
    mut multipart: Multipart,
) -> Result<Json<ImageUrlResponse>, AppError> {
    // Extract file from multipart
    let mut file_data = None;
    let mut content_type = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(&format!("Invalid multipart data: {}", e)))?
    {
        let field_name = field.name().unwrap_or("");

        if field_name == "file" || field_name == "image" {
            content_type = field.content_type().map(|ct| ct.to_string());
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::validation(&format!("Failed to read file: {}", e)))?,
            );
            break;
        }
    }

    let file_data = file_data.ok_or_else(|| {
        AppError::validation("No file provided. Field name should be 'file' or 'image'")
    })?;
    let content_type =
        content_type.ok_or_else(|| AppError::validation("No content-type provided"))?;

    let image_url = service.upload_image(id, file_data, &content_type).await?;

    Ok(Json(ImageUrlResponse { image_url }))
}

/// GET /api/admin/products/:id/image-url
/// Get presigned URL for direct R2 upload
#[derive(Debug, serde::Deserialize)]
pub struct GetUploadUrlQuery {
    pub content_type: Option<String>,
}

pub async fn get_image_upload_url(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
    axum::extract::Query(query): axum::extract::Query<GetUploadUrlQuery>,
) -> Result<Json<crate::application::user::AvatarUploadResponse>, AppError> {
    let content_type = query
        .content_type
        .unwrap_or_else(|| "image/webp".to_string());
    let response = service.get_image_upload_url(id, &content_type).await?;
    Ok(Json(response))
}

/// PUT /api/admin/products/:id/image
/// Save image URL after frontend upload
#[derive(Debug, serde::Deserialize)]
pub struct SaveImageUrlRequest {
    pub image_url: String,
}

pub async fn save_image_url(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
    Json(req): Json<SaveImageUrlRequest>,
) -> Result<StatusCode, AppError> {
    service.save_image_url(id, req.image_url).await?;
    Ok(StatusCode::OK)
}

/// Delete product image
pub async fn delete_product_image(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<StatusCode, AppError> {
    service.delete_product_image(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ==========================================
// 📢 PUBLISH / UNPUBLISH
// ==========================================

/// POST /api/admin/catalog/products/:id/publish
/// Publishes product to the public blog. Validates minimum data quality.
pub async fn publish_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = service.publish_product(id).await?;
    Ok(Json(product))
}

/// POST /api/admin/catalog/products/:id/unpublish
/// Removes product from the public blog (sets is_published = false).
pub async fn unpublish_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = service.unpublish_product(id).await?;
    Ok(Json(product))
}

// ==========================================
// 🤖 AI AUTOFILL
// ==========================================

/// POST /api/admin/catalog/products/:id/ai-autofill
pub async fn ai_autofill_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.ai_autofill(id).await?;
    Ok(Json(result))
}

// ==========================================
// � AI SEO GENERATION
// ==========================================

/// POST /api/admin/catalog/products/:id/ai-seo
pub async fn ai_generate_seo(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.ai_generate_seo(id).await?;
    Ok(Json(result))
}

// ==========================================
// �🔍 AI AUDIT
// ==========================================

/// GET /api/admin/catalog/audit
/// Scans all products, checks completeness, validates against USDA via AI
pub async fn ai_audit(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let report = service.ai_audit().await?;
    Ok(Json(report))
}

// ==========================================
// 🧬 FOOD PAIRING HANDLERS
// ==========================================

/// GET /api/admin/catalog/products/:id/pairings
pub async fn get_pairings(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.get_pairings(id).await?;
    Ok(Json(result))
}

/// Request body for adding a pairing
#[derive(Debug, serde::Deserialize)]
pub struct AddPairingRequest {
    pub paired_product_id: Uuid,
    pub pairing_type: String,
    pub strength: f32,
}

/// POST /api/admin/catalog/products/:id/pairings
pub async fn add_pairing(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
    Json(req): Json<AddPairingRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service
        .add_pairing(id, req.paired_product_id, &req.pairing_type, req.strength)
        .await?;
    Ok(Json(result))
}

/// DELETE /api/admin/catalog/products/:id/pairings/:pairing_id
pub async fn delete_pairing(
    _claims: AdminClaims,
    Path((product_id, pairing_id)): Path<(Uuid, Uuid)>,
    State(service): State<AdminCatalogService>,
) -> Result<StatusCode, AppError> {
    service.delete_pairing(product_id, pairing_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/admin/catalog/products/:id/ai-pairings
pub async fn ai_generate_pairings(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = service.ai_generate_pairings(id).await?;
    Ok(Json(result))
}

/// GET /api/admin/catalog/products/search?q=...
#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub async fn search_products(
    _claims: AdminClaims,
    axum::extract::Query(query): axum::extract::Query<SearchQuery>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let results = service.search_products(&query.q).await?;
    Ok(Json(results))
}

// ==========================================
// 📂 CATEGORY HANDLERS
// ==========================================

/// GET /api/admin/categories
pub async fn list_categories(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
) -> Result<Json<Vec<CategoryResponse>>, AppError> {
    let categories = service.list_categories().await?;
    Ok(Json(categories))
}

/// POST /api/admin/categories
pub async fn create_category(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<CategoryResponse>), AppError> {
    let category = service.create_category(req).await?;
    Ok((StatusCode::CREATED, Json(category)))
}

/// PUT /api/admin/categories/:id
pub async fn update_category(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
    Json(req): Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    let category = service.update_category(id, req).await?;
    Ok(Json(category))
}

/// DELETE /api/admin/categories/:id
pub async fn delete_category(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<StatusCode, AppError> {
    service.delete_category(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── AI Draft Endpoint ────────────────────────────────────────────────

use crate::application::ai_sous_chef::use_cases::create_product_draft::{
    CreateDraftRequest, CreateDraftResponse,
};

/// POST /api/admin/catalog/ai/create-product-draft
pub async fn ai_create_product_draft(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
    Json(req): Json<CreateDraftRequest>,
) -> Result<Json<CreateDraftResponse>, AppError> {
    let response = service.ai_create_product_draft(req).await?;
    Ok(Json(response))
}

// ── AI Suggest Products Endpoint ─────────────────────────────────────

use crate::application::ai_sous_chef::use_cases::suggest_products::{
    SuggestProductsRequest, SuggestProductsResponse,
};

/// POST /api/admin/catalog/ai/suggest-products
///
/// AI suggests 5 products that match the query, for the admin to pick from.
pub async fn ai_suggest_products(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
    Json(req): Json<SuggestProductsRequest>,
) -> Result<Json<SuggestProductsResponse>, AppError> {
    let response = service.ai_suggest_products(req).await?;
    Ok(Json(response))
}

// ==========================================
// 📖 DICTIONARY ADMIN ENDPOINTS
// ==========================================

use crate::infrastructure::DictionaryEntryFull;

/// GET /api/admin/catalog/dictionary
/// List ALL dictionary entries (active, pending, rejected) for admin page
pub async fn list_dictionary(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
) -> Result<Json<Vec<DictionaryEntryFull>>, AppError> {
    let entries = service.dictionary.list_all().await?;
    Ok(Json(entries))
}

/// GET /api/admin/catalog/dictionary/pending
/// List only PENDING entries awaiting admin review
pub async fn list_pending_dictionary(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
) -> Result<Json<Vec<DictionaryEntryFull>>, AppError> {
    let entries = service.dictionary.list_pending().await?;
    Ok(Json(entries))
}

/// POST /api/admin/catalog/dictionary/:id/approve
/// Approve a pending AI translation → becomes active
pub async fn approve_dictionary_entry(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<Json<DictionaryEntryFull>, AppError> {
    let entry = service.dictionary.approve(id).await?;
    Ok(Json(entry))
}

/// Request body for approving with edits
#[derive(Debug, serde::Deserialize)]
pub struct ApproveWithEditsRequest {
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
}

/// PUT /api/admin/catalog/dictionary/:id/approve
/// Approve with corrections — admin fixes translations before activating
pub async fn approve_dictionary_with_edits(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
    Json(req): Json<ApproveWithEditsRequest>,
) -> Result<Json<DictionaryEntryFull>, AppError> {
    let entry = service
        .dictionary
        .approve_with_edits(id, &req.name_ru, &req.name_pl, &req.name_uk)
        .await?;
    Ok(Json(entry))
}

/// POST /api/admin/catalog/dictionary/:id/reject
/// Reject a pending AI translation → marked as rejected
pub async fn reject_dictionary_entry(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(service): State<AdminCatalogService>,
) -> Result<StatusCode, AppError> {
    service.dictionary.reject(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Request body for manual dictionary entry
#[derive(Debug, serde::Deserialize)]
pub struct CreateDictionaryEntryRequest {
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
}

/// POST /api/admin/catalog/dictionary
/// Manually add a new active dictionary entry (admin-curated, source=manual)
pub async fn create_dictionary_entry(
    _claims: AdminClaims,
    State(service): State<AdminCatalogService>,
    Json(req): Json<CreateDictionaryEntryRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let entry = service
        .dictionary
        .insert(&req.name_en, &req.name_pl, &req.name_ru, &req.name_uk)
        .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(entry))))
}
