use crate::application::{
    AdminCatalogService, CreateProductRequest, ProductResponse, UpdateProductRequest,
};
use crate::domain::AdminClaims;
use crate::shared::{AppError, Language};
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

    let file_data = file_data.ok_or_else(|| AppError::validation("No file provided. Field name should be 'file' or 'image'"))?;
    let content_type = content_type.ok_or_else(|| AppError::validation("No content-type provided"))?;

    let image_url = service
        .upload_product_image(id, file_data, &content_type)
        .await?;

    Ok(Json(ImageUrlResponse { image_url }))
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
