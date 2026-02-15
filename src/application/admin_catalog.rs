use crate::infrastructure::R2Client;
use crate::infrastructure::{DictionaryService, GroqService};
use crate::shared::{AppError, AppResult, UnitType};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper function to normalize translations - fallback to English if empty
fn normalize_translation(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.trim().to_string()
    }
}

/// Admin Catalog Service - manage products with image upload to R2
#[derive(Clone)]
pub struct AdminCatalogService {
    pool: PgPool,
    r2_client: R2Client,
    dictionary: DictionaryService,
    groq: GroqService,
}

/// Create Product Request - NEW ARCHITECTURE
/// 
/// Admin can input in ANY language (RU, PL, UK, EN)
/// Backend normalizes to English (canonical) automatically
#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    /// 🧠 Universal input field - can be in ANY language
    /// Backend will detect language and normalize to English
    pub name_input: String,
    
    /// 🌍 Optional manual overrides (if not provided, will be auto-generated)
    #[serde(default = "default_empty_string")]
    pub name_en: String,
    #[serde(default = "default_empty_string")]
    pub name_pl: String,
    #[serde(default = "default_empty_string")]
    pub name_uk: String,
    #[serde(default = "default_empty_string")]
    pub name_ru: String,
    
    /// 🤖 Category & Unit can be AI-classified (optional override)
    pub category_id: Option<Uuid>,
    pub unit: Option<UnitType>,
    
    pub description: Option<String>,
    
    /// Если true, бекенд автоматически переведёт на все языки и классифицирует
    /// Использует dictionary cache, затем Groq если нужно
    #[serde(default = "default_true")]
    pub auto_translate: bool,
}

fn default_true() -> bool {
    true
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
    pub unit: Option<UnitType>,
    pub description: Option<String>,
    /// Если true, бекенд автоматически переведёт empty поля (PL/RU/UK)
    /// Использует dictionary cache, затем Groq если нужно
    #[serde(default)]
    pub auto_translate: bool,
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
    pub unit: UnitType,
    pub description: Option<String>,
    pub image_url: Option<String>,
}

impl AdminCatalogService {
    pub fn new(
        pool: PgPool,
        r2_client: R2Client,
        dictionary: DictionaryService,
        groq: GroqService,
    ) -> Self {
        Self {
            pool,
            r2_client,
            dictionary,
            groq,
        }
    }

    /// Create new product - NEW ARCHITECTURE
    /// 
    /// Pipeline:
    /// 1. Нормализация входа в английский (определение языка + перевод)
    /// 2. Проверка дубликатов
    /// 3. Hybrid перевод (dictionary cache + Groq)
    /// 4. AI классификация (категория + unit, если не предоставлены)
    /// 5. Валидация и сохранение
    pub async fn create_product(&self, req: CreateProductRequest) -> AppResult<ProductResponse> {
        tracing::info!("🚀 Starting product creation pipeline");

        // ==========================================
        // 🧠 ШАГ 1: НОРМАЛИЗАЦИЯ В АНГЛИЙСКИЙ
        // ==========================================
        let name_input = req.name_input.trim();
        if name_input.is_empty() {
            return Err(AppError::validation("name_input cannot be empty"));
        }

        // Если явно предоставлен name_en, используем его
        // Иначе, определяем язык и переводим
        let name_en = if !req.name_en.is_empty() {
            req.name_en.trim().to_string()
        } else {
            tracing::info!("Determining language for input: {}", name_input);
            self.groq.normalize_to_english(name_input).await?
        };

        tracing::info!("Canonical English: {}", name_en);

        // ==========================================
        // 🔍 ШАГ 2: ПРОВЕРКА ДУБЛИКАТОВ
        // ==========================================
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM catalog_ingredients WHERE LOWER(name_en) = LOWER($1) AND COALESCE(is_active, true) = true)"
        )
        .bind(&name_en)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking duplicate: {}", e);
            AppError::internal("Failed to check for duplicate")
        })?;

        if exists {
            return Err(AppError::conflict(&format!("Product '{}' already exists", name_en)));
        }

        // ==========================================
        // 🌍 ШАГ 3: HYBRID ПЕРЕВОД
        // ==========================================
        let mut name_pl = normalize_translation(&req.name_pl, &name_en);
        let mut name_uk = normalize_translation(&req.name_uk, &name_en);
        let mut name_ru = normalize_translation(&req.name_ru, &name_en);

        if req.auto_translate && req.name_pl.trim().is_empty() && req.name_uk.trim().is_empty() && req.name_ru.trim().is_empty() {
            tracing::info!("Auto-translate enabled, checking dictionary cache");

            // 1️⃣ Dictionary cache (0$)
            if let Some(dict_entry) = self.dictionary.find_by_en(&name_en).await? {
                tracing::info!("✅ Cache hit: {}", name_en);
                name_pl = dict_entry.name_pl;
                name_uk = dict_entry.name_uk;
                name_ru = dict_entry.name_ru;
            } else {
                // 2️⃣ Groq AI ($0.01)
                tracing::info!("❌ Cache miss, calling Groq");
                match self.groq.translate(&name_en).await {
                    Ok(translation) => {
                        // Save to dictionary for future
                        if let Err(e) = self.dictionary
                            .insert(&name_en, &translation.pl, &translation.ru, &translation.uk)
                            .await {
                            tracing::warn!("Failed to cache translation: {}", e);
                        }
                        name_pl = translation.pl;
                        name_uk = translation.uk;
                        name_ru = translation.ru;
                    }
                    Err(e) => {
                        tracing::warn!("Translation failed, fallback to English: {}", e);
                        // 3️⃣ Fallback
                        name_pl = name_en.clone();
                        name_uk = name_en.clone();
                        name_ru = name_en.clone();
                    }
                }
            }
        }

        // ==========================================
        // 🤖 ШАГ 4: AI КЛАССИФИКАЦИЯ (если не предоставлены)
        // ==========================================
        let (category_id, unit) = if req.category_id.is_some() && req.unit.is_some() {
            // Используем предоставленные значения
            (req.category_id.unwrap(), req.unit.unwrap())
        } else {
            tracing::info!("Running AI classification for: {}", name_en);
            
            // Graceful degradation: AI не должен ломать CRUD
            let classification = match self.groq.classify_product(&name_en).await {
                Ok(c) => {
                    tracing::info!("✅ AI classification: category={}, unit={}", 
                        c.category_slug, c.unit);
                    c
                }
                Err(e) => {
                    tracing::warn!("⚠️ AI classification failed (using defaults): {}", e);
                    // Graceful fallback: овощи + штука (самые универсальные)
                    crate::infrastructure::groq_service::AiClassification {
                        category_slug: "vegetables".to_string(),
                        unit: "piece".to_string(),
                    }
                }
            };
            
            // Конвертируем AI результаты в BD типы
            let cat_id = match self.find_category_by_slug(&classification.category_slug).await {
                Ok(id) => id,
                Err(_) => {
                    // Если категория не найдена, используем дефолт "Vegetables"
                    tracing::warn!("Category '{}' not found, using Vegetables fallback", classification.category_slug);
                    self.find_category_by_slug("vegetables").await?
                }
            };
            
            let unit = match UnitType::from_string(&classification.unit) {
                Ok(u) => u,
                Err(_) => {
                    // Если unit не парсится, используем piece (самый безопасный)
                    tracing::warn!("Unit '{}' not recognized, using piece fallback", classification.unit);
                    UnitType::Piece
                }
            };
            
            (cat_id, unit)
        };

        // ==========================================
        // 💾 ШАГ 5: СОХРАНЕНИЕ В БД
        // ==========================================
        let id = Uuid::new_v4();

        let product = sqlx::query_as::<_, ProductResponse>(
            r#"
            INSERT INTO catalog_ingredients (
                id, name_en, name_pl, name_uk, name_ru,
                category_id, default_unit, description
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, name_en, name_pl, name_uk, name_ru,
                category_id, 
                default_unit as unit,
                description,
                image_url
            "#
        )
        .bind(id)
        .bind(&name_en)
        .bind(&name_pl)
        .bind(&name_uk)
        .bind(&name_ru)
        .bind(req.category_id)
        .bind(&req.unit)
        .bind(&req.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error creating product '{}': {}", name_en, e);
            AppError::internal("Failed to create product")
        })?;

        Ok(product)
    }

    /// Get product by ID
    pub async fn get_product_by_id(&self, id: Uuid) -> AppResult<ProductResponse> {
        let product = sqlx::query_as::<_, ProductResponse>(
            r#"
            SELECT
                id, name_en, name_pl, name_uk, name_ru,
                category_id,
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

    /// Update product with optional auto-translation via Groq
    pub async fn update_product(
        &self,
        id: Uuid,
        req: UpdateProductRequest,
    ) -> AppResult<ProductResponse> {
        // Check if product exists
        let existing = self.get_product_by_id(id).await?;

        // If name_en is being updated, validate it
        if let Some(ref new_name_en) = req.name_en {
            let name_en = new_name_en.trim();
            if name_en.is_empty() {
                return Err(AppError::validation("name_en cannot be empty"));
            }

            // Check for duplicate (excluding current product)
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM catalog_ingredients WHERE LOWER(name_en) = LOWER($1) AND id != $2 AND COALESCE(is_active, true) = true)"
            )
            .bind(name_en)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error checking duplicate product '{}': {}", name_en, e);
                AppError::internal("Failed to check for duplicate product")
            })?;

            if exists {
                return Err(AppError::conflict(&format!("Product '{}' already exists", name_en)));
            }
        }

        // Determine final English name
        let name_en = req.name_en.as_deref().map(|s| s.trim().to_string());
        let final_name_en = name_en.as_deref().unwrap_or(&existing.name_en);

        // 🧠 HYBRID TRANSLATION LOGIC
        // Check if we need to auto-translate empty fields
        let mut name_pl = req.name_pl.as_deref()
            .map(|s| normalize_translation(s, final_name_en));
        let mut name_uk = req.name_uk.as_deref()
            .map(|s| normalize_translation(s, final_name_en));
        let mut name_ru = req.name_ru.as_deref()
            .map(|s| normalize_translation(s, final_name_en));

        // If auto_translate is enabled and we have empty translations, use cache/Groq
        if req.auto_translate && (name_pl.is_none() && name_uk.is_none() && name_ru.is_none()) {
            tracing::info!("Auto-translation enabled for: {}", final_name_en);

            // 1️⃣ Check dictionary cache first (0$ cost)
            if let Some(dict_entry) = self.dictionary.find_by_en(final_name_en).await? {
                tracing::info!("Found in dictionary cache: {}", final_name_en);
                name_pl = Some(dict_entry.name_pl);
                name_uk = Some(dict_entry.name_uk);
                name_ru = Some(dict_entry.name_ru);
            } else {
                // 2️⃣ Dictionary miss - call Groq (minimal cost)
                tracing::info!("Dictionary miss for: {}, calling Groq", final_name_en);
                
                match self.groq.translate(final_name_en).await {
                    Ok(translation) => {
                        // 3️⃣ Save to dictionary for future use (кеш навсегда)
                        if let Err(e) = self.dictionary
                            .insert(final_name_en, &translation.pl, &translation.ru, &translation.uk)
                            .await {
                            tracing::warn!("Failed to save translation to dictionary: {}", e);
                            // Don't fail the update if dictionary insert fails
                        }

                        name_pl = Some(translation.pl);
                        name_uk = Some(translation.uk);
                        name_ru = Some(translation.ru);
                    }
                    Err(e) => {
                        tracing::warn!("Groq translation failed, falling back to English: {}", e);
                        // Fallback: use English for all languages
                        name_pl = Some(final_name_en.to_string());
                        name_uk = Some(final_name_en.to_string());
                        name_ru = Some(final_name_en.to_string());
                    }
                }
            }
        }

        // 4️⃣ Update database with translations
        let product = sqlx::query_as::<_, ProductResponse>(
            r#"
            UPDATE catalog_ingredients
            SET
                name_en = COALESCE($2, name_en),
                name_pl = COALESCE($3, name_pl),
                name_uk = COALESCE($4, name_uk),
                name_ru = COALESCE($5, name_ru),
                category_id = COALESCE($6, category_id),
                default_unit = COALESCE($7, default_unit),
                description = COALESCE($8, description)
            WHERE id = $1 AND COALESCE(is_active, true) = true
            RETURNING
                id, name_en, name_pl, name_uk, name_ru,
                category_id,
                default_unit as unit,
                description,
                image_url
            "#
        )
        .bind(id)
        .bind(&name_en)
        .bind(&name_pl)
        .bind(&name_uk)
        .bind(&name_ru)
        .bind(req.category_id)
        .bind(&req.unit)
        .bind(&req.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error updating product {}: {}", id, e);
            AppError::internal("Failed to update product")
        })?;

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
        .await
        .map_err(|e| {
            tracing::error!("Database error deleting product {}: {}", id, e);
            AppError::internal("Failed to delete product")
        })?;

        if result.rows_affected() == 0 {
            tracing::warn!("Attempted to delete non-existent product: {}", id);
            return Err(AppError::not_found("Product not found or already deleted"));
        }

        tracing::info!("Product {} soft-deleted successfully", id);
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
        let image_url = self.r2_client.upload_image(&key, file_data, content_type).await
            .map_err(|e| {
                tracing::error!("R2 upload error for product {}: {}", product_id, e);
                AppError::internal("Failed to upload image")
            })?;

        // Update database
        sqlx::query!(
            "UPDATE catalog_ingredients SET image_url = $1 WHERE id = $2",
            image_url,
            product_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error updating image_url for product {}: {}", product_id, e);
            AppError::internal("Failed to update image URL")
        })?;

        tracing::info!("Image uploaded for product {}: {}", product_id, image_url);
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
            if let Err(e) = self.r2_client.delete_image(&key).await {
                tracing::warn!("Failed to delete image {} from R2: {}", key, e);
            }
        }

        // Update database
        sqlx::query!(
            "UPDATE catalog_ingredients SET image_url = NULL WHERE id = $1",
            product_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error clearing image_url for product {}: {}", product_id, e);
            AppError::internal("Failed to clear image URL")
        })?;

        tracing::info!("Image deleted for product {}", product_id);
        Ok(())
    }

    /// 🔍 Найти категорию по AI slug
    /// 
    /// Маппинг AI классификации на реальные категории базы данных
    async fn find_category_by_slug(&self, slug: &str) -> AppResult<Uuid> {
        let category_name_en = match slug.to_lowercase().as_str() {
            "dairy_and_eggs" => "Dairy & Eggs",
            "fruits" => "Fruits",
            "vegetables" => "Vegetables",
            "meat" | "meat_and_poultry" => "Meat & Poultry",
            "seafood" | "fish_and_seafood" => "Fish & Seafood",
            "grains" | "grains_and_pasta" => "Grains & Pasta",
            "beverages" => "Beverages",
            _ => {
                tracing::warn!("Unknown category slug: {}, defaulting to Vegetables", slug);
                "Vegetables"
            }
        };

        let category_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM catalog_categories WHERE name_en = $1 LIMIT 1"
        )
        .bind(category_name_en)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding category: {}", e);
            AppError::internal("Failed to find category")
        })?
        .ok_or_else(|| {
            tracing::error!("Category not found: {}", category_name_en);
            AppError::not_found(&format!("Category '{}' not found", category_name_en))
        })?;

        tracing::info!("Found category {} -> {}", slug, category_id);
        Ok(category_id)
    }
}

