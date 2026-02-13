use crate::infrastructure::R2Client;
use crate::shared::{AppError, AppResult, UnitType};
use bytes::Bytes;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Admin Catalog Service - manage products with image upload to R2
#[derive(Clone)]
pub struct AdminCatalogService {
    pool: PgPool,
    r2_client: R2Client,
}

/// Create Product Request
#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name_en: String,
    #[serde(default = "default_empty_string")]
    pub name_pl: String,
    #[serde(default = "default_empty_string")]
    pub name_uk: String,
    #[serde(default = "default_empty_string")]
    pub name_ru: String,
    pub category_id: Uuid,
    pub price: Decimal,
    pub unit: UnitType,
    pub description: Option<String>,
}

fn default_empty_string() -> String {
    String::new()
}

/// Update Product Request
#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name_en: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub category_id: Option<Uuid>,
    pub price: Option<Decimal>,
    pub unit: Option<UnitType>,
    pub description: Option<String>,
}

/// Product Response
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductResponse {
    pub id: Uuid,
    pub name_en: String,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub category_id: Uuid,
    pub price: Decimal,
    pub unit: UnitType,
    pub description: Option<String>,
    pub image_url: Option<String>,
}

impl AdminCatalogService {
    pub fn new(pool: PgPool, r2_client: R2Client) -> Self {
        Self { pool, r2_client }
    }

    /// Create new product
    pub async fn create_product(&self, req: CreateProductRequest) -> AppResult<ProductResponse> {
        let id = Uuid::new_v4();

        let product = sqlx::query_as::<_, ProductResponse>(
            r#"
            INSERT INTO catalog_ingredients (
                id, name_en, name_pl, name_uk, name_ru,
                category_id, price, default_unit, description, is_active
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true)
            RETURNING
                id, name_en, name_pl, name_uk, name_ru,
                category_id, 
                price,
                default_unit as unit,
                description,
                image_url
            "#
        )
        .bind(id)
        .bind(&req.name_en)
        .bind(&req.name_pl)
        .bind(&req.name_uk)
        .bind(&req.name_ru)
        .bind(req.category_id)
        .bind(req.price)
        .bind(&req.unit)
        .bind(&req.description)
        .fetch_one(&self.pool)
        .await?;

        Ok(product)
    }

    /// Get product by ID
    pub async fn get_product_by_id(&self, id: Uuid) -> AppResult<ProductResponse> {
        let product = sqlx::query_as::<_, ProductResponse>(
            r#"
            SELECT
                id, name_en, name_pl, name_uk, name_ru,
                category_id,
                price,
                default_unit as unit,
                description,
                image_url
            FROM catalog_ingredients
            WHERE id = $1 AND COALESCE(is_active, true) = true
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Product not found"))?;

        Ok(product)
    }

    /// List all products
    pub async fn list_products(&self) -> AppResult<Vec<ProductResponse>> {
        let products = sqlx::query_as::<_, ProductResponse>(
            r#"
            SELECT
                id, name_en, name_pl, name_uk, name_ru,
                category_id,
                price,
                default_unit as unit,
                description,
                image_url
            FROM catalog_ingredients
            WHERE COALESCE(is_active, true) = true
            ORDER BY name_en ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(products)
    }

    /// Update product
    pub async fn update_product(
        &self,
        id: Uuid,
        req: UpdateProductRequest,
    ) -> AppResult<ProductResponse> {
        // Check if product exists
        self.get_product_by_id(id).await?;

        let product = sqlx::query_as::<_, ProductResponse>(
            r#"
            UPDATE catalog_ingredients
            SET
                name_en = COALESCE($2, name_en),
                name_pl = COALESCE($3, name_pl),
                name_uk = COALESCE($4, name_uk),
                name_ru = COALESCE($5, name_ru),
                category_id = COALESCE($6, category_id),
                price = COALESCE($7, price),
                default_unit = COALESCE($8, default_unit),
                description = COALESCE($9, description)
            WHERE id = $1 AND COALESCE(is_active, true) = true
            RETURNING
                id, name_en, name_pl, name_uk, name_ru,
                category_id,
                price,
                default_unit as unit,
                description,
                image_url
            "#
        )
        .bind(id)
        .bind(&req.name_en)
        .bind(&req.name_pl)
        .bind(&req.name_uk)
        .bind(&req.name_ru)
        .bind(req.category_id)
        .bind(req.price)
        .bind(&req.unit)
        .bind(&req.description)
        .fetch_one(&self.pool)
        .await?;

        Ok(product)
    }

    /// Delete product
    pub async fn delete_product(&self, id: Uuid) -> AppResult<()> {
        // Soft delete - mark as inactive instead of deleting
        // This preserves relationships with inventory and other tables
        let result = sqlx::query(
            "UPDATE catalog_ingredients SET is_active = false WHERE id = $1 AND is_active = true"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Product not found or already deleted"));
        }

        Ok(())
    }

    /// Upload/replace product image
    /// Key format: products/{product_id}.{ext}
    pub async fn upload_product_image(
        &self,
        product_id: Uuid,
        file_data: Bytes,
        content_type: &str,
    ) -> AppResult<String> {
        // Check if product exists
        self.get_product_by_id(product_id).await?;

        // Validate content type
        let extension = match content_type {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => return Err(AppError::validation("Invalid image type. Allowed: jpg, png, webp")),
        };

        // Validate file size (max 5MB)
        const MAX_SIZE: usize = 5 * 1024 * 1024; // 5MB
        if file_data.len() > MAX_SIZE {
            return Err(AppError::validation("File too large. Max size: 5MB"));
        }

        // Generate consistent key: products/{uuid}.{ext}
        let key = format!("products/{}.{}", product_id, extension);

        // Upload to R2
        let image_url = self.r2_client.upload_image(&key, file_data, content_type).await?;

        // Update database
        sqlx::query!(
            "UPDATE catalog_ingredients SET image_url = $1 WHERE id = $2",
            image_url,
            product_id
        )
        .execute(&self.pool)
        .await?;

        Ok(image_url)
    }

    /// Delete product image
    pub async fn delete_product_image(&self, product_id: Uuid) -> AppResult<()> {
        // Get product
        let product = self.get_product_by_id(product_id).await?;

        if product.image_url.is_none() {
            return Err(AppError::not_found("Product has no image"));
        }

        // Try to delete all possible extensions (in case of inconsistency)
        for ext in ["jpg", "png", "webp"] {
            let key = format!("products/{}.{}", product_id, ext);
            let _ = self.r2_client.delete_image(&key).await;
        }

        // Update database
        sqlx::query!(
            "UPDATE catalog_ingredients SET image_url = NULL WHERE id = $1",
            product_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
