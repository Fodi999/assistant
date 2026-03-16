use crate::infrastructure::R2Client;
use crate::infrastructure::{DictionaryService, LlmAdapter, UnifiedProductResponse};
use crate::infrastructure::persistence::{
    AiCacheRepository,
    CatalogCategoryRepository, CatalogIngredientRepository,
};
use crate::shared::{AppError, AppResult, UnitType};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Admin Catalog Service - manage products with image upload to R2
#[derive(Clone)]
pub struct AdminCatalogService {
    pool: PgPool,
    r2_client: R2Client,
    dictionary: DictionaryService,
    llm_adapter: Arc<LlmAdapter>,
    category_repo: CatalogCategoryRepository,
    ingredient_repo: CatalogIngredientRepository,
    ai_cache: AiCacheRepository,
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

/// Update Product Request — full editing support for admin panel
#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name_en: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub category_id: Option<Uuid>,
    pub unit: Option<UnitType>,
    pub description: Option<String>,
    pub image_url: Option<String>,

    // Multilingual descriptions
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,

    // Nutrition per 100g
    pub calories_per_100g: Option<i32>,
    pub protein_per_100g: Option<f64>,
    pub fat_per_100g: Option<f64>,
    pub carbs_per_100g: Option<f64>,

    // Physical properties
    pub density_g_per_ml: Option<f64>,

    // Seasons & allergens
    pub seasons: Option<Vec<String>>,
    pub allergens: Option<Vec<String>>,

    // Fish availability calendar (12 bools: Jan..Dec)
    pub availability_months: Option<Vec<bool>>,

    // Product type & availability model
    pub product_type: Option<String>,
    pub availability_model: Option<String>,

    // Kitchen calculator fields
    pub shelf_life_days: Option<i32>,
    pub edible_yield_percent: Option<f64>,
    pub typical_portion_g: Option<f64>,
    pub substitution_group: Option<String>,

    // Extra nutrients per 100g
    pub fiber_per_100g: Option<f64>,
    pub sugar_per_100g: Option<f64>,
    pub salt_per_100g: Option<f64>,

    // Fish / seafood attributes
    pub water_type: Option<String>,   // "sea" | "freshwater" | "both"
    pub wild_farmed: Option<String>,  // "wild" | "farmed" | "both"
    pub sushi_grade: Option<bool>,

    // SEO fields
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub seo_h1: Option<String>,
    pub canonical_url: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,

    /// Если true, бекенд автоматически переведёт empty поля (PL/RU/UK)
    /// Использует dictionary cache, затем Groq если нужно
    #[serde(default)]
    pub auto_translate: bool,
}

/// Product Response — full data for admin panel
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductResponse {
    pub id: Uuid,
    pub slug: Option<String>,
    pub name_en: String,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub category_id: Uuid,
    pub unit: UnitType,
    pub description: Option<String>,
    pub image_url: Option<String>,

    // Multilingual descriptions
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,

    // Nutrition per 100g
    pub calories_per_100g: Option<i32>,
    pub protein_per_100g: Option<rust_decimal::Decimal>,
    pub fat_per_100g: Option<rust_decimal::Decimal>,
    pub carbs_per_100g: Option<rust_decimal::Decimal>,

    // Physical properties
    pub density_g_per_ml: Option<rust_decimal::Decimal>,

    // Seasons & allergens as text arrays
    pub seasons: Vec<String>,
    pub allergens: Vec<String>,

    // Fish availability calendar (12 bools: Jan..Dec)
    pub availability_months: Option<Vec<bool>>,

    // Product type & availability model
    pub product_type: String,
    pub availability_model: String,

    // Kitchen calculator fields
    pub shelf_life_days: Option<i32>,
    pub edible_yield_percent: Option<rust_decimal::Decimal>,
    pub typical_portion_g: Option<rust_decimal::Decimal>,
    pub substitution_group: Option<String>,

    // Extra nutrients per 100g
    pub fiber_per_100g: Option<rust_decimal::Decimal>,
    pub sugar_per_100g: Option<rust_decimal::Decimal>,
    pub salt_per_100g: Option<rust_decimal::Decimal>,

    // Fish / seafood attributes
    pub water_type: Option<String>,
    pub wild_farmed: Option<String>,
    pub sushi_grade: Option<bool>,

    // SEO fields
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub seo_h1: Option<String>,
    pub canonical_url: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
}

/// Admin Category Requests
#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name_pl: String,
    pub name_en: String,
    pub name_uk: String,
    pub name_ru: String,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name_pl: Option<String>,
    pub name_en: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name_pl: String,
    pub name_en: String,
    pub name_uk: String,
    pub name_ru: String,
    pub sort_order: i32,
}

/// Row for AI audit — collects completeness data across all tables
#[derive(Debug, sqlx::FromRow)]
struct AuditRow {
    id: Uuid,
    slug: Option<String>,
    name_en: Option<String>,
    name_ru: Option<String>,
    product_type: Option<String>,
    image_url: Option<String>,
    description_en: Option<String>,
    description_ru: Option<String>,
    description_pl: Option<String>,
    description_uk: Option<String>,
    calories_per_100g: Option<i32>,
    protein: Option<f32>,
    fat: Option<f32>,
    carbs: Option<f32>,
    density: Option<f32>,
    shelf_life_days: Option<i32>,
    portion: Option<f32>,
    availability_months: Option<Vec<bool>>,
    // nutrition sub-table existence
    has_macros: Option<bool>,
    has_vitamins: Option<bool>,
    has_minerals: Option<bool>,
    has_fatty_acids: Option<bool>,
    has_diet_flags: Option<bool>,
    has_allergens: Option<bool>,
    has_food_props: Option<bool>,
    has_culinary: Option<bool>,
    // key values for AI validation
    n_calories: Option<f32>,
    n_protein: Option<f32>,
    n_fat: Option<f32>,
    n_carbs: Option<f32>,
    m_calcium: Option<f32>,
    m_iron: Option<f32>,
    m_potassium: Option<f32>,
    m_sodium: Option<f32>,
    v_c: Option<f32>,
    v_d: Option<f32>,
}

impl AdminCatalogService {
    pub fn new(
        pool: PgPool,
        r2_client: R2Client,
        dictionary: DictionaryService,
        llm_adapter: Arc<LlmAdapter>,
    ) -> Self {
        let ai_cache = AiCacheRepository::new(pool.clone());
        Self {
            pool: pool.clone(),
            r2_client,
            dictionary,
            llm_adapter,
            category_repo: CatalogCategoryRepository::new(pool.clone()),
            ingredient_repo: CatalogIngredientRepository::new(pool),
            ai_cache,
        }
    }

    /// Create new product - OPTIMIZED UNIFIED ARCHITECTURE
    ///
    /// Pipeline (OPTIMIZED - one AI call instead of 3):
    /// 1️⃣ Unified AI processing (normalize + translate + classify in ONE call)
    /// 2️⃣ Check for duplicates (case-insensitive on name_en)
    /// 3️⃣ Cache translations to dictionary for future use
    /// 4️⃣ Save to database with all translations
    ///
    /// Performance: 3x faster (~700ms instead of ~1800ms)
    /// Cost: 1/3 of the original ($0.001 instead of $0.003)
    pub async fn create_product(&self, req: CreateProductRequest) -> AppResult<ProductResponse> {
        tracing::info!("🚀 Starting optimized product creation pipeline");

        let name_input = req.name_input.trim();
        if name_input.is_empty() {
            return Err(AppError::validation("name_input cannot be empty"));
        }

        // ==========================================
        // � ШАГ 1: UNIFIED AI PROCESSING (instead of 3 separate calls!)
        // ==========================================
        // If user provided explicit values, use them (don't call AI)
        // Otherwise, call unified processing which returns EVERYTHING at once
        let (name_en, name_pl, name_uk, name_ru, category_slug, unit_str, confidence) = if !req
            .name_en
            .is_empty()
            && !req.name_pl.is_empty()
            && !req.name_ru.is_empty()
            && !req.name_uk.is_empty()
        {
            // All fields provided explicitly - no AI needed
            tracing::info!("All translations provided explicitly, skipping AI");
            (
                req.name_en.trim().to_string(),
                req.name_pl.trim().to_string(),
                req.name_uk.trim().to_string(),
                req.name_ru.trim().to_string(),
                "vegetables".to_string(), // Will be overridden below if provided
                "piece".to_string(),      // Will be overridden below if provided
                1.0,                      // 100% confident since user provided it
            )
        } else {
            // Use unified processing: ONE call returns everything
            tracing::info!("Running unified AI processing for: {}", name_input);

            // 1️⃣ Unified AI processing (Rule Engine -> Cache -> LLM)
            let ai_result = self.llm_adapter.process_unified(name_input).await?;

            (
                ai_result.name_en,
                ai_result.name_pl,
                ai_result.name_uk,
                ai_result.name_ru,
                ai_result.category_slug,
                ai_result.unit,
                ai_result.confidence,
            )
        };

        tracing::info!("Canonical English: {} (Confidence: {:.2})", name_en, confidence);
        
        if confidence < 0.8 {
            tracing::warn!("⚠️ Low AI confidence ({:.2}) for product '{}'. Consider manual review.", confidence, name_en);
            // In a real app, we might return a specific status code or flag to the frontend
            // to show a confirmation dialog. For now, we just log it.
        }

        // ==========================================
        // 🔍 ШАГ 2: ПРОВЕРКА ДУБЛИКАТОВ (case-insensitive on canonical name)
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
            return Err(AppError::conflict(&format!(
                "Product '{}' already exists",
                name_en
            )));
        }

        // ==========================================
        // 💾 ШАГ 3: CACHE translations to dictionary for future use
        // ==========================================
        // Save to dictionary so next time we need these translations, they're free
        if let Err(e) = self
            .dictionary
            .insert(&name_en, &name_pl, &name_ru, &name_uk)
            .await
        {
            tracing::warn!("Failed to cache translations to dictionary: {}", e);
            // Not critical - continue anyway
        }

        // ==========================================
        // 🤖 ШАГ 4: RESOLVE CATEGORY & UNIT (override AI if provided)
        // ==========================================
        let (final_category_id, final_unit) = if req.category_id.is_some() && req.unit.is_some() {
            // User provided explicit overrides
            (req.category_id.unwrap(), req.unit.unwrap())
        } else {
            // Use AI results
            let cat_id = match self.find_category_by_slug(&category_slug).await {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!(
                        "Category '{}' not found, rejecting product creation",
                        category_slug
                    );
                    return Err(AppError::validation(&format!(
                        "Invalid category from AI: {}. Please provide explicit category_id",
                        category_slug
                    )));
                }
            };

            let unit_resolved = match UnitType::from_string(&unit_str) {
                Ok(u) => u,
                Err(_) => {
                    tracing::warn!(
                        "Unit '{}' not recognized, rejecting product creation",
                        unit_str
                    );
                    return Err(AppError::validation(&format!(
                        "Invalid unit from AI: {}. Please provide explicit unit",
                        unit_str
                    )));
                }
            };

            (cat_id, unit_resolved)
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
                id, slug, name_en, name_pl, name_uk, name_ru,
                category_id, default_unit as unit, description, image_url,
                description_en, description_pl, description_ru, description_uk,
                calories_per_100g,
                protein_per_100g, fat_per_100g, carbs_per_100g,
                density_g_per_ml,
                seasons::text[] as seasons, allergens::text[] as allergens,
                availability_months,
                COALESCE(product_type, 'other') as product_type,
                COALESCE(availability_model, 'all_year') as availability_model,
                shelf_life_days, edible_yield_percent, typical_portion_g, substitution_group,
                fiber_per_100g, sugar_per_100g, salt_per_100g,
                      water_type, wild_farmed, sushi_grade,
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image
            "#,
        )
        .bind(id)
        .bind(&name_en)
        .bind(&name_pl)
        .bind(&name_uk)
        .bind(&name_ru)
        .bind(final_category_id)
        .bind(&final_unit)
        .bind(&req.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error creating product '{}': {}", name_en, e);
            AppError::internal("Failed to create product")
        })?;

        // ==========================================
        // 🔄 ШАГ 6: SYNC TO PRODUCTS TABLE (for food_pairing FK + nutrition)
        // ==========================================
        let _ = sqlx::query(
            r#"INSERT INTO products (id, slug, name_en, name_pl, name_ru, name_uk,
                                     category_id, product_type, unit, image_url)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'other', $8, $9)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(&product.slug)
        .bind(&name_en)
        .bind(&name_pl)
        .bind(&name_ru)
        .bind(&name_uk)
        .bind(final_category_id)
        .bind(final_unit.to_string())
        .bind(&product.image_url)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::warn!("Failed to sync product to products table: {}", e);
            e
        })
        .ok(); // Non-critical — log and continue

        // ==========================================
        // 🤖 ШАГ 7: AUTO-GENERATE PROCESSING STATES (ai_sous_chef)
        // ==========================================
        // Generates all 10 states (raw, boiled, fried, etc.) deterministically
        tokio::spawn({
            let pool = self.pool.clone();
            async move {
                // Small delay to ensure the INSERT is committed
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                match crate::application::ai_sous_chef::generate_states::generate_states_for_ingredient(&pool, id).await {
                    Ok(states) => {
                        tracing::info!("✅ Auto-generated {} processing states for product {}", states.len(), id);
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ Failed to auto-generate states for {}: {}", id, e);
                    }
                }
            }
        });

        Ok(product)
    }

    /// Get product by ID
    pub async fn get_product_by_id(&self, id: Uuid) -> AppResult<ProductResponse> {
        let product = sqlx::query_as::<_, ProductResponse>(
            r#"SELECT id, slug, name_en, name_pl, name_uk, name_ru,
                      category_id, default_unit as unit, description, image_url,
                      description_en, description_pl, description_ru, description_uk,
                      calories_per_100g,
                      protein_per_100g, fat_per_100g, carbs_per_100g,
                      density_g_per_ml,
                      seasons::text[] as seasons, allergens::text[] as allergens,
                      availability_months,
                      COALESCE(product_type, 'other') as product_type,
                      COALESCE(availability_model, 'all_year') as availability_model,
                      shelf_life_days, edible_yield_percent, typical_portion_g, substitution_group,
                      fiber_per_100g, sugar_per_100g, salt_per_100g,
                      water_type, wild_farmed, sushi_grade,
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image
               FROM catalog_ingredients
               WHERE id = $1 AND is_active = true"#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Product not found OR deleted"))?;

        Ok(product)
    }

    /// List all products in the catalog
    pub async fn list_products(&self) -> AppResult<Vec<ProductResponse>> {
        let products = sqlx::query_as::<_, ProductResponse>(
            r#"SELECT id, slug, name_en, name_pl, name_uk, name_ru,
                      category_id, default_unit as unit, description, image_url,
                      description_en, description_pl, description_ru, description_uk,
                      calories_per_100g,
                      protein_per_100g, fat_per_100g, carbs_per_100g,
                      density_g_per_ml,
                      seasons::text[] as seasons, allergens::text[] as allergens,
                      availability_months,
                      COALESCE(product_type, 'other') as product_type,
                      COALESCE(availability_model, 'all_year') as availability_model,
                      shelf_life_days, edible_yield_percent, typical_portion_g, substitution_group,
                      fiber_per_100g, sugar_per_100g, salt_per_100g,
                      water_type, wild_farmed, sushi_grade,
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image
               FROM catalog_ingredients
               WHERE is_active = true
               ORDER BY name_en ASC"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(products)
    }

    /// Update product in the catalog (all fields)
    pub async fn update_product(
        &self,
        id: Uuid,
        req: UpdateProductRequest,
    ) -> AppResult<ProductResponse> {
        let mut tx = self.pool.begin().await?;

        // 1. Get existing product
        let product = sqlx::query_as::<_, ProductResponse>(
            r#"SELECT id, slug, name_en, name_pl, name_uk, name_ru,
                      category_id, default_unit as unit, description, image_url,
                      description_en, description_pl, description_ru, description_uk,
                      calories_per_100g,
                      protein_per_100g, fat_per_100g, carbs_per_100g,
                      density_g_per_ml,
                      seasons::text[] as seasons, allergens::text[] as allergens,
                      availability_months,
                      COALESCE(product_type, 'other') as product_type,
                      COALESCE(availability_model, 'all_year') as availability_model,
                      shelf_life_days, edible_yield_percent, typical_portion_g, substitution_group,
                      fiber_per_100g, sugar_per_100g, salt_per_100g,
                      water_type, wild_farmed, sushi_grade,
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image
               FROM catalog_ingredients
               WHERE id = $1 AND is_active = true FOR UPDATE"#
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("Product not found"))?;

        // 2. Prepare name values
        let old_name_en = product.name_en.clone();
        let name_en = req.name_en.unwrap_or(product.name_en);
        let mut name_pl = req.name_pl.or(product.name_pl);
        let mut name_uk = req.name_uk.or(product.name_uk);
        let mut name_ru = req.name_ru.or(product.name_ru);

        if req.auto_translate {
            // First check dictionary cache
            let cached = self.dictionary.find_by_en(&name_en).await.unwrap_or(None);

            let translations = if let Some(entry) = cached {
                crate::infrastructure::groq_service::GroqTranslationResponse {
                    pl: entry.name_pl,
                    ru: entry.name_ru,
                    uk: entry.name_uk,
                }
            } else {
                // If not in cache, call AI via Adapter (Cache)
                match self.llm_adapter.translate(&name_en).await {
                    Ok(t) => {
                        // Cache it for future
                        let _ = self.dictionary.insert(&name_en, &t.pl, &t.ru, &t.uk).await;
                        t
                    }
                    Err(_) => crate::infrastructure::groq_service::GroqTranslationResponse {
                        pl: "".to_string(),
                        ru: "".to_string(),
                        uk: "".to_string(),
                    },
                }
            };

            if name_pl.is_none() || name_pl.as_deref() == Some("") {
                name_pl = Some(translations.pl);
            }
            if name_uk.is_none() || name_uk.as_deref() == Some("") {
                name_uk = Some(translations.uk);
            }
            if name_ru.is_none() || name_ru.as_deref() == Some("") {
                name_ru = Some(translations.ru);
            }
        }

        // 3. Prepare nutrition & description values
        let mut description_en = req.description_en.or(product.description_en);
        let mut description_pl = req.description_pl.or(product.description_pl);
        let mut description_ru = req.description_ru.or(product.description_ru);
        let mut description_uk = req.description_uk.or(product.description_uk);

        // Guard: if description_en contains Cyrillic, move it to description_ru and translate to EN
        if let Some(ref en_text) = description_en {
            let has_cyrillic = en_text.chars().any(|c| matches!(c, '\u{0400}'..='\u{04FF}'));
            if has_cyrillic {
                let trimmed = en_text.trim().to_string();
                tracing::warn!("description_en contains Cyrillic for product, auto-fixing: {:?}", &trimmed);
                // Move to description_ru if RU is empty
                if description_ru.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                    description_ru = Some(trimmed.clone());
                }
                // Translate to English
                match self.llm_adapter.translate_to_language(&trimmed, "en").await {
                    Ok(t) if !t.trim().is_empty() => {
                        description_en = Some(t);
                    }
                    _ => {
                        // Couldn't translate — clear the field so AI autofill can fill it later
                        description_en = Some(String::new());
                    }
                }
            }
        }

        // Auto-translate descriptions when description_en exists and target lang is empty
        if req.auto_translate {
            if let Some(ref en_text) = description_en {
                let en_trimmed = en_text.trim();
                if !en_trimmed.is_empty() {
                    let needs_pl = description_pl.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true);
                    let needs_ru = description_ru.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true);
                    let needs_uk = description_uk.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true);

                    if needs_pl {
                        match self.llm_adapter.translate_to_language(en_trimmed, "pl").await {
                            Ok(t) if !t.trim().is_empty() => { description_pl = Some(t); }
                            _ => {}
                        }
                    }
                    if needs_ru {
                        match self.llm_adapter.translate_to_language(en_trimmed, "ru").await {
                            Ok(t) if !t.trim().is_empty() => { description_ru = Some(t); }
                            _ => {}
                        }
                    }
                    if needs_uk {
                        match self.llm_adapter.translate_to_language(en_trimmed, "uk").await {
                            Ok(t) if !t.trim().is_empty() => { description_uk = Some(t); }
                            _ => {}
                        }
                    }
                }
            }
        }

        let calories = req.calories_per_100g.or(product.calories_per_100g);
        let protein: Option<rust_decimal::Decimal> = req.protein_per_100g
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default())
            .or(product.protein_per_100g);
        let fat: Option<rust_decimal::Decimal> = req.fat_per_100g
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default())
            .or(product.fat_per_100g);
        let carbs: Option<rust_decimal::Decimal> = req.carbs_per_100g
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default())
            .or(product.carbs_per_100g);
        let density: Option<rust_decimal::Decimal> = req.density_g_per_ml
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default())
            .or(product.density_g_per_ml);

        // 4. Update record with ALL fields
        let seasons_val: Option<Vec<String>> = req.seasons;
        let allergens_val: Option<Vec<String>> = req.allergens;
        let availability_months_val: Option<Vec<bool>> = req.availability_months;
        let shelf_life = req.shelf_life_days.or(product.shelf_life_days);
        let edible_yield: Option<rust_decimal::Decimal> = req.edible_yield_percent
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default())
            .or(product.edible_yield_percent);
        let typical_portion: Option<rust_decimal::Decimal> = req.typical_portion_g
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default())
            .or(product.typical_portion_g);
        let subst_group = req.substitution_group.or(product.substitution_group);
        let prod_type = req.product_type.unwrap_or(product.product_type);
        let avail_model = req.availability_model.unwrap_or(product.availability_model);

        // New nutrition fields — use COALESCE in SQL so None keeps existing value
        let fiber: Option<rust_decimal::Decimal> = req.fiber_per_100g
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default());
        let sugar: Option<rust_decimal::Decimal> = req.sugar_per_100g
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default());
        let salt: Option<rust_decimal::Decimal> = req.salt_per_100g
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default());
        let water_type_val = req.water_type;
        let wild_farmed_val = req.wild_farmed;
        let sushi_grade_val = req.sushi_grade;

        let updated_product = sqlx::query_as::<_, ProductResponse>(
            r#"
            UPDATE catalog_ingredients
            SET name_en = $1, name_pl = $2, name_uk = $3, name_ru = $4,
                category_id = $5, default_unit = $6, description = $7,
                image_url = COALESCE($8, image_url),
                description_en = $9, description_pl = $10,
                description_ru = $11, description_uk = $12,
                calories_per_100g = $13, protein_per_100g = $14,
                fat_per_100g = $15, carbs_per_100g = $16,
                density_g_per_ml = $17,
                seasons = CASE WHEN $18::text[] IS NOT NULL THEN $18::text[]::season_type[] ELSE seasons END,
                allergens = CASE WHEN $19::text[] IS NOT NULL THEN $19::text[]::allergen_type[] ELSE allergens END,
                availability_months = COALESCE($20, availability_months),
                product_type = $21,
                availability_model = $22,
                shelf_life_days = $23,
                edible_yield_percent = $24,
                typical_portion_g = $25,
                substitution_group = $26,
                fiber_per_100g = COALESCE($28, fiber_per_100g),
                sugar_per_100g = COALESCE($29, sugar_per_100g),
                salt_per_100g = COALESCE($30, salt_per_100g),
                water_type = COALESCE($31, water_type),
                wild_farmed = COALESCE($32, wild_farmed),
                sushi_grade = COALESCE($33, sushi_grade),
                seo_title = COALESCE($34, seo_title),
                seo_description = COALESCE($35, seo_description),
                seo_h1 = COALESCE($36, seo_h1),
                canonical_url = COALESCE($37, canonical_url),
                og_title = COALESCE($38, og_title),
                og_description = COALESCE($39, og_description),
                og_image = COALESCE($40, og_image)
            WHERE id = $27
            RETURNING id, slug, name_en, name_pl, name_uk, name_ru,
                      category_id, default_unit as unit, description, image_url,
                      description_en, description_pl, description_ru, description_uk,
                      calories_per_100g,
                      protein_per_100g, fat_per_100g, carbs_per_100g,
                      density_g_per_ml,
                      seasons::text[] as seasons, allergens::text[] as allergens,
                      availability_months,
                      COALESCE(product_type, 'other') as product_type,
                      COALESCE(availability_model, 'all_year') as availability_model,
                      shelf_life_days, edible_yield_percent, typical_portion_g, substitution_group,
                      fiber_per_100g, sugar_per_100g, salt_per_100g,
                      water_type, wild_farmed, sushi_grade,
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image
            "#
        )
        .bind(&name_en)
        .bind(&name_pl)
        .bind(&name_uk)
        .bind(&name_ru)
        .bind(req.category_id.unwrap_or(product.category_id))
        .bind(req.unit.unwrap_or(product.unit))
        .bind(req.description.or(product.description))
        .bind(&req.image_url)
        .bind(&description_en)
        .bind(&description_pl)
        .bind(&description_ru)
        .bind(&description_uk)
        .bind(calories)
        .bind(protein)
        .bind(fat)
        .bind(carbs)
        .bind(density)
        .bind(&seasons_val)
        .bind(&allergens_val)
        .bind(&availability_months_val)
        .bind(&prod_type)
        .bind(&avail_model)
        .bind(shelf_life)
        .bind(edible_yield)
        .bind(typical_portion)
        .bind(&subst_group)
        .bind(id)              // $27
        .bind(fiber)           // $28
        .bind(sugar)           // $29
        .bind(salt)            // $30
        .bind(&water_type_val) // $31
        .bind(&wild_farmed_val)// $32
        .bind(sushi_grade_val) // $33
        .bind(&req.seo_title)       // $34
        .bind(&req.seo_description) // $35
        .bind(&req.seo_h1)         // $36
        .bind(&req.canonical_url)   // $37
        .bind(&req.og_title)        // $38
        .bind(&req.og_description)  // $39
        .bind(&req.og_image)        // $40
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        // 🧹 Invalidate AI cache for this product so frontend gets fresh data
        let name_lower = name_en.to_lowercase();
        let prefixes = [
            format!("translate_all:{}", name_lower),
            format!("product_unified:{}", name_lower),
        ];
        for prefix in &prefixes {
            if let Err(e) = self.ai_cache.delete(prefix).await {
                tracing::warn!("Failed to invalidate cache key '{}': {}", prefix, e);
            }
        }
        // Also invalidate by old name if it changed
        let old_name_lower = old_name_en.to_lowercase();
        if old_name_lower != name_lower {
            let old_prefixes = [
                format!("translate_all:{}", old_name_lower),
                format!("product_unified:{}", old_name_lower),
            ];
            for prefix in &old_prefixes {
                if let Err(e) = self.ai_cache.delete(prefix).await {
                    tracing::warn!("Failed to invalidate old cache key '{}': {}", prefix, e);
                }
            }
        }
        tracing::info!("🧹 Cache invalidated for product: {}", name_en);

        // Log slug change if name_en changed (DB trigger auto-updates slug + saves alias)
        let old_slug = product.slug.as_deref().unwrap_or("");
        let new_slug = updated_product.slug.as_deref().unwrap_or("");
        if !old_slug.is_empty() && !new_slug.is_empty() && old_slug != new_slug {
            tracing::info!(
                "🔀 Slug auto-updated: '{}' → '{}' (old slug saved as alias for 301 redirect)",
                old_slug, new_slug
            );
        }

        Ok(updated_product)
    }

    /// Generate presigned URL for catalog image upload
    /// 🎯 SaaS 2026: Frontend uploads directly to R2
    pub async fn get_image_upload_url(
        &self,
        product_id: Uuid,
        content_type: &str,
    ) -> AppResult<crate::application::user::AvatarUploadResponse> {
        // 1. Verify product exists
        let _ = self.get_product_by_id(product_id).await?;

        // 2. Determine file extension from content_type
        let ext = if content_type.contains("jpeg") || content_type.contains("jpg") {
            "jpg"
        } else if content_type.contains("png") {
            "png"
        } else {
            "webp"
        };

        // 3. Generate key: assets/catalog/{product_id}.{ext}
        let key = format!("assets/catalog/{}.{}", product_id, ext);

        // 4. Generate presigned URL (valid for 5 mins)
        let upload_url = self
            .r2_client
            .generate_presigned_upload_url(&key, content_type)
            .await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(crate::application::user::AvatarUploadResponse {
            upload_url,
            public_url,
        })
    }

    /// Save image URL after frontend uploaded to R2
    pub async fn save_image_url(&self, product_id: Uuid, image_url: String) -> AppResult<()> {
        sqlx::query("UPDATE catalog_ingredients SET image_url = $1 WHERE id = $2")
            .bind(image_url)
            .bind(product_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete product image
    pub async fn delete_product_image(&self, id: Uuid) -> AppResult<()> {
        // 1. Get product to find image key
        let product = self.get_product_by_id(id).await?;

        if let Some(image_url) = product.image_url {
            // Extract key from URL (assuming format: base_url/key)
            // usually key is assets/catalog/{id}.webp or products/{id}.{ext}

            // Delete from R2 (optional, as we at least null it in DB)
            if let Some(key_part) = image_url.split("/").last() {
                // Determine folder based on URL content or use a default
                let folder = if image_url.contains("catalog") {
                    "assets/catalog"
                } else {
                    "products"
                };
                let _ = self
                    .r2_client
                    .delete_image(&format!("{}/{}", folder, key_part))
                    .await;
            }
        }

        sqlx::query("UPDATE catalog_ingredients SET image_url = NULL WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Upload image to R2 (DEPRECATED - used for direct backend upload)
    pub async fn upload_image(
        &self,
        id: Uuid,
        file_data: Bytes,
        content_type: &str,
    ) -> AppResult<String> {
        // Check if product exists
        self.get_product_by_id(id).await?;

        // Validate content type
        let extension = match content_type {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => {
                return Err(AppError::validation(
                    "Invalid image type. Allowed: jpg, png, webp",
                ))
            }
        };

        // Validate file size (max 5MB)
        const MAX_SIZE: usize = 5 * 1024 * 1024; // 5MB
        if file_data.len() > MAX_SIZE {
            return Err(AppError::validation("File too large. Max size: 5MB"));
        }

        // Generate consistent key: products/{uuid}.{ext}
        let key = format!("products/{}.{}", id, extension);

        // Upload to R2
        let image_url = self
            .r2_client
            .upload_image(&key, file_data, content_type)
            .await
            .map_err(|e| {
                tracing::error!("R2 upload error for product {}: {}", id, e);
                AppError::internal("Failed to upload image")
            })?;

        // Update database
        sqlx::query("UPDATE catalog_ingredients SET image_url = $1 WHERE id = $2")
            .bind(&image_url)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Database error updating image_url for product {}: {}",
                    id,
                    e
                );
                AppError::internal("Failed to update image URL")
            })?;

        tracing::info!("Image uploaded for product {}: {}", id, image_url);
        Ok(image_url)
    }

    /// Delete product
    pub async fn delete_product(&self, id: Uuid) -> AppResult<()> {
        // Soft delete - mark as inactive instead of deleting
        // This preserves relationships with inventory and other tables
        let result = sqlx::query(
            "UPDATE catalog_ingredients SET is_active = false WHERE id = $1 AND is_active = true",
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

    // ==========================================
    // 🤖 AI AUTOFILL
    // ==========================================

    /// AI autofill — asks Groq to fill all empty nutrition/description/culinary
    /// fields for a given product by its slug/name. Returns a JSON suggestion
    /// that the admin reviews before saving.
    pub async fn ai_autofill(&self, id: Uuid) -> AppResult<serde_json::Value> {
        // Load current product to know what's already filled
        let product = self.get_product_by_id(id).await?;

        let name_en = product.name_en.clone();
        let name_ru = product.name_ru.clone().unwrap_or_default();
        let product_type = product.product_type.clone();

        // Mark which fields are already filled so AI skips them
        let has_description_en = product.description_en.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_description_ru = product.description_ru.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_description_pl = product.description_pl.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_description_uk = product.description_uk.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_calories = product.calories_per_100g.is_some();
        let has_protein = product.protein_per_100g.is_some();
        let has_fat = product.fat_per_100g.is_some();
        let has_carbs = product.carbs_per_100g.is_some();

        let prompt = format!(
            r#"You are a professional food database expert. Fill in the missing data for this ingredient.

Product: "{name_en}" (Russian: "{name_ru}", type: "{product_type}")

Return ONLY a valid JSON object. Rules:
- If a field says "FILLED=true" below → return null (do NOT overwrite).
- If a field says "FILLED=false" below → you MUST provide a real value (do NOT return null).
- For all other fields not listed below → always provide a value.

Field status (true=already has data, false=EMPTY and needs your data):
- description_en: FILLED={has_desc_en}
- description_ru: FILLED={has_desc_ru}
- description_pl: FILLED={has_desc_pl}
- description_uk: FILLED={has_desc_uk}
- calories_per_100g: FILLED={has_cal}
- protein_per_100g: FILLED={has_prot}
- fat_per_100g: FILLED={has_fat}
- carbs_per_100g: FILLED={has_carbs}

IMPORTANT: If FILLED=false, you MUST provide the value, NOT null!
All nutritional values should be for RAW product per 100g.

Return this JSON:
{{
  "description_en": "<2-3 sentence culinary description in English, or null if FILLED=true>",
  "description_ru": "<2-3 предложения кулинарное описание на русском, or null if FILLED=true>",
  "description_pl": "<2-3 zdania opis kulinarny po polsku, or null if FILLED=true>",
  "description_uk": "<2-3 речення кулінарний опис українською, or null if FILLED=true>",
  "calories_per_100g": <integer kcal per 100g, or null>,
  "protein_per_100g": <float grams protein per 100g, or null>,
  "fat_per_100g": <float grams fat per 100g, or null>,
  "carbs_per_100g": <float grams carbs per 100g, or null>,
  "fiber_per_100g": <float or null>,
  "sugar_per_100g": <float or null>,
  "density_g_per_ml": <float density g/ml, or null>,
  "typical_portion_g": <float typical serving size in grams, or null>,
  "shelf_life_days": <integer days shelf life, or null>,
  "product_type": "<one of: fish, seafood, meat, vegetable, fruit, dairy, grain, spice, oil, beverage, nut, legume, other — or null if already set>",
  "seasons": <["Spring","Summer","Autumn","Winter"] subset or ["AllYear"], based on typical availability>,
  "macros": {{
    "calories_kcal": <integer or null>,
    "protein_g": <float or null>,
    "fat_g": <float or null>,
    "carbs_g": <float or null>,
    "fiber_g": <float or null>,
    "sugar_g": <float or null>,
    "starch_g": <float or null>,
    "water_g": <float or null>
  }},
  "vitamins": {{
    "vitamin_a": <float mg per 100g or null>,
    "vitamin_c": <float mg per 100g or null>,
    "vitamin_d": <float mg per 100g or null>,
    "vitamin_e": <float mg per 100g or null>,
    "vitamin_k": <float mcg per 100g or null>,
    "vitamin_b1": <float or null>,
    "vitamin_b2": <float or null>,
    "vitamin_b3": <float or null>,
    "vitamin_b5": <float or null>,
    "vitamin_b6": <float or null>,
    "vitamin_b9": <float or null>,
    "vitamin_b12": <float or null>
  }},
  "minerals": {{
    "calcium": <float mg per 100g or null>,
    "iron": <float or null>,
    "magnesium": <float or null>,
    "phosphorus": <float or null>,
    "potassium": <float or null>,
    "sodium": <float or null>,
    "zinc": <float or null>,
    "selenium": <float or null>
  }},
  "fatty_acids": {{
    "saturated_fat": <float g per 100g or null>,
    "monounsaturated_fat": <float or null>,
    "polyunsaturated_fat": <float or null>,
    "omega3": <float or null>,
    "omega6": <float or null>,
    "epa": <float or null>,
    "dha": <float or null>
  }},
  "diet_flags": {{
    "vegan": <true/false>,
    "vegetarian": <true/false>,
    "gluten_free": <true/false>,
    "keto": <true/false>,
    "paleo": <true/false>,
    "mediterranean": <true/false>,
    "low_carb": <true/false>
  }},
  "allergens": {{
    "milk": <true/false>,
    "eggs": <true/false>,
    "fish": <true/false>,
    "shellfish": <true/false>,
    "nuts": <true/false>,
    "peanuts": <true/false>,
    "gluten": <true/false>,
    "soy": <true/false>,
    "sesame": <true/false>,
    "celery": <true/false>,
    "mustard": <true/false>,
    "sulfites": <true/false>,
    "lupin": <true/false>,
    "molluscs": <true/false>
  }},
  "culinary": {{
    "sweetness": <1-10 integer or null>,
    "acidity": <1-10 integer or null>,
    "bitterness": <1-10 integer or null>,
    "umami": <1-10 integer or null>,
    "aroma": <1-10 integer or null>,
    "texture": "<string like crispy/tender/creamy or null>"
  }},
  "food_properties": {{
    "glycemic_index": <integer or null>,
    "glycemic_load": <float or null>,
    "ph": <float 0-14 or null>,
    "smoke_point": <integer celsius or null>,
    "water_activity": <float 0-1 or null>
  }}
}}

Use USDA FoodData Central (raw/uncooked values per 100g) as reference. Be precise.
REMEMBER: FILLED=false means the field is EMPTY — you MUST provide a real value, NOT null!"#,
            name_en = name_en,
            name_ru = name_ru,
            product_type = product_type,
            has_desc_en = has_description_en,
            has_desc_ru = has_description_ru,
            has_desc_pl = has_description_pl,
            has_desc_uk = has_description_uk,
            has_cal = has_calories,
            has_prot = has_protein,
            has_fat = has_fat,
            has_carbs = has_carbs,
        );

        let raw = self.llm_adapter.groq_raw_request(&prompt, 3000).await?;

        // Parse JSON with fallback extraction
        let result: serde_json::Value = serde_json::from_str(&raw)
            .or_else(|_| {
                if let Some(start) = raw.find('{') {
                    if let Some(end) = raw.rfind('}') {
                        return serde_json::from_str(&raw[start..=end]);
                    }
                }
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No JSON found",
                )))
            })
            .map_err(|e| {
                tracing::error!("Failed to parse AI autofill response: {}", e);
                AppError::internal("AI returned invalid JSON")
            })?;

        tracing::info!("✅ AI autofill complete for product {}", id);
        Ok(result)
    }

    // ==========================================
    // � AI SEO GENERATION
    // ==========================================

    /// Generate SEO metadata for a product using AI.
    /// Takes product name, category, nutrition highlights and generates
    /// title, description, h1, og_title, og_description.
    pub async fn ai_generate_seo(&self, id: Uuid) -> AppResult<serde_json::Value> {
        let product = self.get_product_by_id(id).await?;

        let name_en = &product.name_en;
        let slug = product.slug.as_deref().unwrap_or("unknown");
        let product_type = &product.product_type;
        let name_ru = product.name_ru.as_deref().unwrap_or("");

        // Collect nutrition highlights for better SEO
        let mut highlights = Vec::new();
        if let Some(cal) = product.calories_per_100g {
            highlights.push(format!("{}kcal", cal));
        }
        if let Some(p) = &product.protein_per_100g {
            highlights.push(format!("{}g protein", p));
        }
        if let Some(f) = &product.fat_per_100g {
            highlights.push(format!("{}g fat", f));
        }
        if let Some(c) = &product.carbs_per_100g {
            highlights.push(format!("{}g carbs", c));
        }
        let nutrition_str = if highlights.is_empty() {
            "nutrition data available".to_string()
        } else {
            highlights.join(", ")
        };

        let prompt = format!(
            r#"You are an SEO expert for a food/nutrition website "dima-fomin.pl".

Generate SEO metadata for this ingredient page:

Product: "{name_en}" (Russian: "{name_ru}")
Category: {product_type}
Slug: {slug}
Nutrition highlights per 100g: {nutrition_str}
Page URL: https://dima-fomin.pl/ingredients/{slug}

Return ONLY a valid JSON object:
{{
  "seo_title": "<60 chars max, format: {{Name}} — Nutrition, Vitamins, Culinary Uses | dima-fomin.pl>",
  "seo_description": "<155 chars max, compelling meta description with nutrition keywords>",
  "seo_h1": "<H1 heading, format: {{Name}} — Nutrition & Culinary Profile>",
  "canonical_url": "https://dima-fomin.pl/ingredients/{slug}",
  "og_title": "<65 chars max, engaging Open Graph title for social sharing>",
  "og_description": "<200 chars max, social-friendly description with key nutrition facts>"
}}

Rules:
- seo_title: Include product name + "Nutrition" + one key benefit. Max 60 chars.
- seo_description: Include calories, protein, key vitamin/mineral. Max 155 chars. Make it click-worthy.
- seo_h1: Clean, keyword-rich H1. Slightly different from seo_title.
- og_title: Social-friendly, can be more casual. Include emoji if appropriate.
- og_description: Focus on most interesting nutrition fact or culinary use.
- All in English.
- Return ONLY the JSON, no other text."#,
            name_en = name_en,
            name_ru = name_ru,
            product_type = product_type,
            slug = slug,
            nutrition_str = nutrition_str,
        );

        let raw = self.llm_adapter.groq_raw_request(&prompt, 800).await?;

        let result: serde_json::Value = serde_json::from_str(&raw)
            .or_else(|_| {
                if let Some(start) = raw.find('{') {
                    if let Some(end) = raw.rfind('}') {
                        return serde_json::from_str(&raw[start..=end]);
                    }
                }
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No JSON found",
                )))
            })
            .map_err(|e| {
                tracing::error!("Failed to parse AI SEO response: {}", e);
                AppError::internal("AI returned invalid JSON")
            })?;

        tracing::info!("✅ AI SEO generated for product {} ({})", id, name_en);
        Ok(result)
    }

    // ==========================================
    // �🔍 AI AUDIT — Catalog completeness & accuracy checker
    // ==========================================

    /// AI Audit — scans all products, checks completeness of all fields,
    /// then asks AI to validate nutrition data against USDA reference.
    /// Returns structured report for admin dashboard.
    pub async fn ai_audit(&self) -> AppResult<serde_json::Value> {
        tracing::info!("🔍 Starting AI catalog audit...");

        // ── Phase 1: Collect all products with nutrition data ──
        let rows = sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT
                ci.id,
                ci.slug,
                ci.name_en,
                ci.name_ru,
                COALESCE(ci.product_type, 'other') as product_type,
                ci.image_url,
                ci.description_en,
                ci.description_ru,
                ci.description_pl,
                ci.description_uk,
                ci.calories_per_100g,
                ci.protein_per_100g::float4  as protein,
                ci.fat_per_100g::float4      as fat,
                ci.carbs_per_100g::float4    as carbs,
                ci.density_g_per_ml::float4  as density,
                ci.shelf_life_days,
                ci.typical_portion_g::float4 as portion,
                ci.availability_months,
                -- nutrition sub-tables existence flags
                (SELECT COUNT(*) > 0 FROM nutrition_macros     WHERE product_id = ci.id) as has_macros,
                (SELECT COUNT(*) > 0 FROM nutrition_vitamins   WHERE product_id = ci.id) as has_vitamins,
                (SELECT COUNT(*) > 0 FROM nutrition_minerals   WHERE product_id = ci.id) as has_minerals,
                (SELECT COUNT(*) > 0 FROM nutrition_fatty_acids WHERE product_id = ci.id) as has_fatty_acids,
                (SELECT COUNT(*) > 0 FROM diet_flags           WHERE product_id = ci.id) as has_diet_flags,
                (SELECT COUNT(*) > 0 FROM product_allergens    WHERE product_id = ci.id) as has_allergens,
                (SELECT COUNT(*) > 0 FROM food_properties      WHERE product_id = ci.id) as has_food_props,
                (SELECT COUNT(*) > 0 FROM food_culinary_properties WHERE product_id = ci.id) as has_culinary,
                -- key macros from nutrition table
                nm.calories_kcal as n_calories,
                nm.protein_g     as n_protein,
                nm.fat_g         as n_fat,
                nm.carbs_g       as n_carbs,
                -- key minerals
                nmi.calcium      as m_calcium,
                nmi.iron         as m_iron,
                nmi.potassium    as m_potassium,
                nmi.sodium       as m_sodium,
                -- key vitamins
                nv.vitamin_c     as v_c,
                nv.vitamin_d     as v_d
            FROM catalog_ingredients ci
            LEFT JOIN nutrition_macros nm ON nm.product_id = ci.id
            LEFT JOIN nutrition_minerals nmi ON nmi.product_id = ci.id
            LEFT JOIN nutrition_vitamins nv ON nv.product_id = ci.id
            WHERE ci.is_active = true
            ORDER BY ci.name_en ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Audit query failed: {}", e);
            AppError::internal("Failed to query products for audit")
        })?;

        let total = rows.len();
        tracing::info!("🔍 Auditing {} products...", total);

        // ── Phase 2: Build completeness report for each product ──
        let mut issues: Vec<serde_json::Value> = Vec::new();
        let mut products_for_ai: Vec<String> = Vec::new();

        for row in &rows {
            let name = row.name_en.as_deref().unwrap_or("?");
            let slug = row.slug.as_deref().unwrap_or("?");
            let mut missing: Vec<String> = Vec::new();
            let mut warnings: Vec<String> = Vec::new();

            // ── Check basic fields ──
            if row.image_url.is_none() {
                missing.push("🖼️ нет фото".into());
            }
            if row.description_en.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push("📝 description_en".into());
            }
            if row.description_ru.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push("📝 description_ru".into());
            }
            if row.description_pl.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push("📝 description_pl".into());
            }
            if row.description_uk.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push("📝 description_uk".into());
            }

            // ── Check catalog macros ──
            if row.calories_per_100g.is_none() { missing.push("🔢 calories".into()); }
            if row.protein.is_none() { missing.push("🔢 protein".into()); }
            if row.fat.is_none() { missing.push("🔢 fat".into()); }
            if row.carbs.is_none() { missing.push("🔢 carbs".into()); }

            // ── Check physical ──
            if row.density.is_none() { missing.push("⚙️ density".into()); }
            if row.shelf_life_days.is_none() { missing.push("⚙️ shelf_life_days".into()); }
            if row.portion.is_none() { missing.push("⚙️ typical_portion".into()); }

            // ── Check nutrition sub-tables ──
            if !row.has_macros.unwrap_or(false) { missing.push("📊 macros table".into()); }
            if !row.has_vitamins.unwrap_or(false) { missing.push("⚗️ vitamins table".into()); }
            if !row.has_minerals.unwrap_or(false) { missing.push("🪨 minerals table".into()); }
            if !row.has_fatty_acids.unwrap_or(false) { missing.push("🧈 fatty_acids table".into()); }
            if !row.has_diet_flags.unwrap_or(false) { missing.push("🥗 diet_flags table".into()); }
            if !row.has_allergens.unwrap_or(false) { missing.push("⚠️ allergens table".into()); }
            if !row.has_food_props.unwrap_or(false) { missing.push("🔬 food_properties table".into()); }
            if !row.has_culinary.unwrap_or(false) { missing.push("🍳 culinary table".into()); }

            // ── Check availability_months ──
            if row.availability_months.is_none() {
                missing.push("📅 availability_months".into());
            }

            // ── Quick sanity checks (no AI needed) ──
            if let Some(cal) = row.calories_per_100g {
                if cal < 0 || cal > 900 {
                    warnings.push(format!("⚠️ calories={} (expected 0-900)", cal));
                }
            }
            if let Some(p) = row.protein {
                if p < 0.0 || p > 100.0 {
                    warnings.push(format!("⚠️ protein={:.1} (expected 0-100)", p));
                }
            }
            if let Some(f) = row.fat {
                if f < 0.0 || f > 100.0 {
                    warnings.push(format!("⚠️ fat={:.1} (expected 0-100)", f));
                }
            }
            if let Some(c) = row.carbs {
                if c < 0.0 || c > 100.0 {
                    warnings.push(format!("⚠️ carbs={:.1} (expected 0-100)", c));
                }
            }
            // Macros sum > 100g? (protein + fat + carbs should not exceed ~100g per 100g)
            if let (Some(p), Some(f), Some(c)) = (row.protein, row.fat, row.carbs) {
                let sum = p + f + c;
                if sum > 105.0 {
                    warnings.push(format!("⚠️ P+F+C={:.1}g > 100g (impossible)", sum));
                }
            }

            // Collect products that HAVE macros data for AI validation
            if row.has_macros.unwrap_or(false) && row.n_calories.is_some() {
                products_for_ai.push(format!(
                    "- {name}: cal={cal}, prot={prot}, fat={fat}, carbs={carbs}, vit_c={vc}, vit_d={vd}, calcium={ca}, iron={fe}, potassium={k}, sodium={na}",
                    name = name,
                    cal = row.n_calories.map(|v| format!("{:.0}", v)).unwrap_or("?".into()),
                    prot = row.n_protein.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    fat = row.n_fat.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    carbs = row.n_carbs.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    vc = row.v_c.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    vd = row.v_d.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    ca = row.m_calcium.map(|v| format!("{:.0}", v)).unwrap_or("?".into()),
                    fe = row.m_iron.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    k = row.m_potassium.map(|v| format!("{:.0}", v)).unwrap_or("?".into()),
                    na = row.m_sodium.map(|v| format!("{:.0}", v)).unwrap_or("?".into()),
                ));
            }

            let completeness = if missing.is_empty() && warnings.is_empty() {
                continue; // Skip fully complete products without warnings
            } else {
                let total_checks: u32 = 20; // approximate total fields checked
                let filled = total_checks.saturating_sub(missing.len() as u32);
                ((filled as f64 / total_checks as f64) * 100.0).round() as u32
            };

            issues.push(serde_json::json!({
                "id": row.id,
                "name_en": name,
                "slug": slug,
                "product_type": row.product_type,
                "completeness_percent": completeness,
                "missing": missing,
                "warnings": warnings,
            }));
        }

        // ── Phase 3: AI validation of filled products against USDA ──
        let mut ai_warnings: Vec<serde_json::Value> = Vec::new();

        if !products_for_ai.is_empty() && products_for_ai.len() <= 30 {
            let products_list = products_for_ai.join("\n");
            let ai_prompt = format!(
                r#"You are a food database QA expert. Below is nutrition data (per 100g raw) from our database for several products.

Compare each product's values against USDA FoodData Central reference and report ONLY significant errors (>40% deviation from USDA).

Products:
{products_list}

Return ONLY a JSON array of issues found. Each issue:
{{
  "product": "<product name>",
  "field": "<field name>",
  "our_value": <our value>,
  "usda_value": <USDA reference value>,
  "deviation_percent": <percentage deviation>,
  "severity": "<high|medium|low>",
  "comment": "<brief explanation in Russian>"
}}

If ALL values are accurate, return an empty array: []
Be strict — only flag errors >40% deviation. Return ONLY the JSON array, no other text."#,
                products_list = products_list,
            );

            match self.llm_adapter.groq_raw_request(&ai_prompt, 2000).await {
                Ok(raw) => {
                    // Try to parse JSON array
                    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&raw)
                        .or_else(|_| {
                            if let Some(start) = raw.find('[') {
                                if let Some(end) = raw.rfind(']') {
                                    return serde_json::from_str(&raw[start..=end]);
                                }
                            }
                            Ok(vec![])
                        });
                    if let Ok(items) = parsed {
                        ai_warnings = items;
                    }
                }
                Err(e) => {
                    tracing::warn!("AI validation failed (non-critical): {}", e);
                }
            }
        }

        // ── Phase 4: Build final report ──
        // Sort issues by completeness (worst first)
        issues.sort_by(|a, b| {
            let ca = a["completeness_percent"].as_u64().unwrap_or(100);
            let cb = b["completeness_percent"].as_u64().unwrap_or(100);
            ca.cmp(&cb)
        });

        let complete_count = total - issues.len();
        let report = serde_json::json!({
            "summary": {
                "total_products": total,
                "fully_complete": complete_count,
                "needs_attention": issues.len(),
                "ai_data_warnings": ai_warnings.len(),
                "audit_date": chrono::Utc::now().to_rfc3339(),
            },
            "products_needing_attention": issues,
            "ai_usda_warnings": ai_warnings,
        });

        tracing::info!(
            "✅ Audit complete: {}/{} products OK, {} need attention, {} AI warnings",
            complete_count, total, issues.len(), ai_warnings.len()
        );

        Ok(report)
    }

    /// List all categories
    pub async fn list_categories(&self) -> AppResult<Vec<CategoryResponse>> {
        let categories = sqlx::query_as::<_, CategoryResponse>(
            "SELECT id, name_pl, name_en, name_uk, name_ru, sort_order FROM catalog_categories ORDER BY sort_order ASC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(categories)
    }

    /// Create new category
    pub async fn create_category(&self, req: CreateCategoryRequest) -> AppResult<CategoryResponse> {
        let category = sqlx::query_as::<_, CategoryResponse>(
            r#"
            INSERT INTO catalog_categories (name_pl, name_en, name_uk, name_ru, sort_order)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name_pl, name_en, name_uk, name_ru, sort_order
            "#,
        )
        .bind(req.name_pl)
        .bind(req.name_en)
        .bind(req.name_uk)
        .bind(req.name_ru)
        .bind(req.sort_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(category)
    }

    /// Update category
    pub async fn update_category(
        &self,
        id: Uuid,
        req: UpdateCategoryRequest,
    ) -> AppResult<CategoryResponse> {
        let current = sqlx::query_as::<_, CategoryResponse>(
            "SELECT id, name_pl, name_en, name_uk, name_ru, sort_order FROM catalog_categories WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Category not found"))?;

        let category = sqlx::query_as::<_, CategoryResponse>(
            r#"
            UPDATE catalog_categories 
            SET name_pl = $1, name_en = $2, name_uk = $3, name_ru = $4, sort_order = $5
            WHERE id = $6
            RETURNING id, name_pl, name_en, name_uk, name_ru, sort_order
            "#,
        )
        .bind(req.name_pl.unwrap_or(current.name_pl))
        .bind(req.name_en.unwrap_or(current.name_en))
        .bind(req.name_uk.unwrap_or(current.name_uk))
        .bind(req.name_ru.unwrap_or(current.name_ru))
        .bind(req.sort_order.unwrap_or(current.sort_order))
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(category)
    }

    /// Delete category (fails if used by products)
    pub async fn delete_category(&self, id: Uuid) -> AppResult<()> {
        // Check if referenced by active products only (is_active = true)
        let in_use: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM catalog_ingredients WHERE category_id = $1 AND is_active = true)",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if in_use {
            return Err(AppError::conflict(
                "Cannot delete category: it is used by products",
            ));
        }

        // Hard-delete any soft-deleted (is_active = false) products in this category first
        sqlx::query("DELETE FROM catalog_ingredients WHERE category_id = $1 AND is_active = false")
            .bind(id)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM catalog_categories WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
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
            "SELECT id FROM catalog_categories WHERE name_en = $1 LIMIT 1",
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

    // ==========================================
    // 🧬 FOOD PAIRING — CRUD + AI
    // ==========================================

    /// Response struct for a single pairing row
    #[allow(dead_code)]
    pub async fn get_pairings(&self, product_id: Uuid) -> AppResult<serde_json::Value> {
        #[derive(Debug, sqlx::FromRow, Serialize)]
        struct PairingRow {
            id: Option<Uuid>,
            paired_slug: Option<String>,
            paired_name_en: Option<String>,
            paired_name_ru: Option<String>,
            paired_image: Option<String>,
            paired_product_id: Option<Uuid>,
            pairing_type: Option<String>,
            pair_score: Option<f32>,
            flavor_score: Option<f32>,
            nutrition_score: Option<f32>,
            culinary_score: Option<f32>,
        }

        let rows: Vec<PairingRow> = sqlx::query_as(
            r#"SELECT fp.id,
                      b.slug    AS paired_slug,
                      b.name_en AS paired_name_en,
                      b.name_ru AS paired_name_ru,
                      b.image_url AS paired_image,
                      b.id      AS paired_product_id,
                      fp.pairing_type,
                      fp.pair_score,
                      fp.flavor_score,
                      fp.nutrition_score,
                      fp.culinary_score
               FROM food_pairing fp
               JOIN catalog_ingredients b ON b.id = fp.ingredient_b
               WHERE fp.ingredient_a = $1
               ORDER BY fp.pair_score DESC NULLS LAST"#,
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get pairings: {}", e);
            AppError::internal("Failed to get pairings")
        })?;

        // Group by pairing_type
        let mut primary = Vec::new();
        let mut secondary = Vec::new();
        let mut experimental = Vec::new();
        let mut avoid = Vec::new();

        for r in &rows {
            let item = serde_json::json!({
                "id": r.id,
                "paired_product_id": r.paired_product_id,
                "slug": r.paired_slug,
                "name_en": r.paired_name_en,
                "name_ru": r.paired_name_ru,
                "image_url": r.paired_image,
                "pair_score": r.pair_score,
                "flavor_score": r.flavor_score,
                "nutrition_score": r.nutrition_score,
                "culinary_score": r.culinary_score,
            });
            match r.pairing_type.as_deref().unwrap_or("primary") {
                "secondary" => secondary.push(item),
                "experimental" => experimental.push(item),
                "avoid" => avoid.push(item),
                _ => primary.push(item),
            }
        }

        Ok(serde_json::json!({
            "product_id": product_id,
            "total": rows.len(),
            "primary": primary,
            "secondary": secondary,
            "experimental": experimental,
            "avoid": avoid,
        }))
    }

    /// Add a single pairing
    pub async fn add_pairing(
        &self,
        product_id: Uuid,
        paired_product_id: Uuid,
        pairing_type: &str,
        strength: f32,
    ) -> AppResult<serde_json::Value> {
        // Validate pairing_type
        let valid_types = ["primary", "secondary", "experimental", "avoid"];
        if !valid_types.contains(&pairing_type) {
            return Err(AppError::validation("Invalid pairing_type. Must be: primary, secondary, experimental, avoid"));
        }

        // Insert or update (ON CONFLICT updates)
        sqlx::query(
            r#"INSERT INTO food_pairing (ingredient_a, ingredient_b, pairing_type, pair_score, id)
               VALUES ($1, $2, $3, $4, gen_random_uuid())
               ON CONFLICT (ingredient_a, ingredient_b)
               DO UPDATE SET pairing_type = $3, pair_score = $4"#,
        )
        .bind(product_id)
        .bind(paired_product_id)
        .bind(pairing_type)
        .bind(strength)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add pairing: {}", e);
            AppError::internal("Failed to add pairing")
        })?;

        tracing::info!("✅ Added pairing {} -> {} ({})", product_id, paired_product_id, pairing_type);

        // Return updated pairings
        self.get_pairings(product_id).await
    }

    /// Delete a pairing by its id
    pub async fn delete_pairing(&self, product_id: Uuid, pairing_id: Uuid) -> AppResult<()> {
        let result = sqlx::query(
            "DELETE FROM food_pairing WHERE id = $1 AND ingredient_a = $2",
        )
        .bind(pairing_id)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete pairing: {}", e);
            AppError::internal("Failed to delete pairing")
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Pairing not found"));
        }

        tracing::info!("✅ Deleted pairing {} from product {}", pairing_id, product_id);
        Ok(())
    }

    /// Search products by name (for pairing ingredient search)
    pub async fn search_products(&self, query: &str) -> AppResult<Vec<serde_json::Value>> {
        #[derive(Debug, sqlx::FromRow)]
        struct SearchRow {
            id: Uuid,
            slug: Option<String>,
            name_en: String,
            name_ru: Option<String>,
            image_url: Option<String>,
            product_type: Option<String>,
        }

        let pattern = format!("%{}%", query.to_lowercase());

        let rows: Vec<SearchRow> = sqlx::query_as(
            r#"SELECT ci.id, ci.slug, ci.name_en, ci.name_ru, ci.image_url, ci.product_type
               FROM catalog_ingredients ci
               WHERE ci.is_active = true
                 AND (LOWER(ci.name_en) LIKE $1
                      OR LOWER(COALESCE(ci.name_ru, '')) LIKE $1
                      OR LOWER(COALESCE(ci.name_pl, '')) LIKE $1
                      OR LOWER(COALESCE(ci.slug, '')) LIKE $1)
               ORDER BY ci.name_en
               LIMIT 15"#,
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Product search failed: {}", e);
            AppError::internal("Search failed")
        })?;

        let results: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "slug": r.slug,
                    "name_en": r.name_en,
                    "name_ru": r.name_ru,
                    "image_url": r.image_url,
                    "product_type": r.product_type,
                })
            })
            .collect();

        Ok(results)
    }

    /// AI Generate pairings for a product
    pub async fn ai_generate_pairings(&self, product_id: Uuid) -> AppResult<serde_json::Value> {
        let product = self.get_product_by_id(product_id).await?;
        let name_en = &product.name_en;
        let product_type = &product.product_type;

        // Get existing catalog slugs for matching
        #[derive(Debug, sqlx::FromRow)]
        struct SlugRow {
            id: Uuid,
            slug: Option<String>,
            name_en: String,
        }

        let catalog: Vec<SlugRow> = sqlx::query_as(
            "SELECT id, slug, name_en FROM catalog_ingredients WHERE is_active = true ORDER BY name_en",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let catalog_names: Vec<String> = catalog.iter().map(|r| r.name_en.to_lowercase()).collect();

        let prompt = format!(
            r#"You are a culinary expert and food scientist.

For the ingredient "{name_en}" (category: {product_type}), suggest food pairings.

AVAILABLE INGREDIENTS in our catalog:
{catalog_list}

Return ONLY a valid JSON object:
{{
  "primary": ["ingredient1", "ingredient2", ...],
  "secondary": ["ingredient3", "ingredient4", ...],
  "experimental": ["ingredient5"],
  "avoid": ["ingredient6"]
}}

Rules:
- primary: classic, well-known pairings (3-8 items)
- secondary: good but less common pairings (2-5 items)
- experimental: surprising/creative pairings (0-3 items)
- avoid: ingredients that clash (0-2 items)
- ONLY use ingredient names from the available list above
- Use lowercase English names exactly as shown
- Return ONLY the JSON, no other text"#,
            name_en = name_en,
            product_type = product_type,
            catalog_list = catalog_names.join(", "),
        );

        let raw = self.llm_adapter.groq_raw_request(&prompt, 1000).await?;

        let ai_result: serde_json::Value = serde_json::from_str(&raw)
            .or_else(|_| {
                if let Some(start) = raw.find('{') {
                    if let Some(end) = raw.rfind('}') {
                        return serde_json::from_str(&raw[start..=end]);
                    }
                }
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No JSON found",
                )))
            })
            .map_err(|e| {
                tracing::error!("Failed to parse AI pairings: {}", e);
                AppError::internal("AI returned invalid JSON")
            })?;

        // Match AI suggestions to catalog products and insert
        let mut inserted = 0u32;
        let mut not_found = Vec::new();

        for (ptype, score_base) in &[
            ("primary", 9.0f32),
            ("secondary", 6.5),
            ("experimental", 4.0),
            ("avoid", 1.0),
        ] {
            if let Some(arr) = ai_result.get(ptype).and_then(|v| v.as_array()) {
                for (i, item) in arr.iter().enumerate() {
                    if let Some(name) = item.as_str() {
                        let name_lower = name.to_lowercase();
                        // Find matching product
                        let matched = catalog.iter().find(|r| {
                            r.name_en.to_lowercase() == name_lower
                                || r.slug.as_deref().unwrap_or("") == name_lower
                        });

                        if let Some(matched_product) = matched {
                            if matched_product.id == product_id {
                                continue; // skip self-pairing
                            }
                            let score = score_base - (i as f32 * 0.3);
                            match sqlx::query(
                                r#"INSERT INTO food_pairing (ingredient_a, ingredient_b, pairing_type, pair_score, id)
                                   VALUES ($1, $2, $3, $4, gen_random_uuid())
                                   ON CONFLICT (ingredient_a, ingredient_b)
                                   DO UPDATE SET pairing_type = $3, pair_score = $4"#,
                            )
                            .bind(product_id)
                            .bind(matched_product.id)
                            .bind(*ptype)
                            .bind(score)
                            .execute(&self.pool)
                            .await
                            {
                                Ok(_) => inserted += 1,
                                Err(e) => {
                                    tracing::error!("Failed to insert pairing {} -> {}: {}", product_id, matched_product.name_en, e);
                                    not_found.push(format!("{}(DB error)", name));
                                }
                            }
                        } else {
                            not_found.push(name.to_string());
                        }
                    }
                }
            }
        }

        tracing::info!(
            "✅ AI pairings for {}: {} inserted, {} not found in catalog",
            name_en, inserted, not_found.len()
        );

        // Return updated pairings + AI metadata
        let pairings = self.get_pairings(product_id).await?;
        Ok(serde_json::json!({
            "ai_suggestions": ai_result,
            "inserted": inserted,
            "not_found_in_catalog": not_found,
            "pairings": pairings,
        }))
    }
}
