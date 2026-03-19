use crate::infrastructure::R2Client;
use crate::infrastructure::{DictionaryService, LlmAdapter};
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
    pub(crate) pool: PgPool,
    pub(crate) r2_client: R2Client,
    pub(crate) dictionary: DictionaryService,
    pub(crate) llm_adapter: Arc<LlmAdapter>,
    pub(crate) category_repo: CatalogCategoryRepository,
    pub(crate) ingredient_repo: CatalogIngredientRepository,
    pub(crate) ai_cache: AiCacheRepository,
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

    /// Product type from AI draft (e.g. "seafood", "meat", "dairy")
    /// Used by enforce_category() to validate category ↔ product_type consistency
    pub product_type: Option<String>,

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

// ── 🔒 enforce_category: product_type ↔ category hard rules ─────────
//
// AI cannot be trusted with categories. Backend is the source of truth.
// This function enforces a strict mapping: product_type → correct category name.
// It also rejects "other" — every product MUST have a real type.

/// Map product_type to the canonical category name_en in the DB.
/// Returns None if product_type is unknown (caller should reject or fallback).
fn product_type_to_category(product_type: &str) -> Option<&'static str> {
    match product_type.to_lowercase().as_str() {
        "fish" | "seafood" | "fish_and_seafood" => Some("Fish & Seafood"),
        "meat" | "poultry" | "meat_and_poultry" => Some("Meat & Poultry"),
        "dairy" | "dairy_and_eggs" | "eggs" => Some("Dairy & Eggs"),
        "vegetable" | "vegetables" => Some("Vegetables"),
        "fruit" | "fruits" => Some("Fruits"),
        "grain" | "grains" | "grains_and_pasta" | "pasta" | "cereal" => Some("Grains & Pasta"),
        "legume" | "legumes" => Some("Legumes"),
        "nut" | "nuts" | "seeds" => Some("Nuts & Seeds"),
        "spice" | "spices" | "herb" | "herbs" | "seasoning" => Some("Spices & Herbs"),
        "oil" | "oils" | "fat" | "fats" => Some("Oils & Fats"),
        "beverage" | "beverages" | "drink" => Some("Beverages"),
        "mushroom" | "mushrooms" | "fungi" => Some("Vegetables"), // mushrooms → vegetables
        _ => None, // "other" or anything unknown → rejected
    }
}

/// Normalize product_type to canonical singular form.
/// Rejects "other" — returns error.
fn normalize_product_type(raw: &str) -> AppResult<String> {
    let normalized = match raw.to_lowercase().as_str() {
        "fish" | "seafood" | "fish_and_seafood" => "seafood",
        "meat" | "poultry" | "meat_and_poultry" => "meat",
        "dairy" | "dairy_and_eggs" | "eggs" => "dairy",
        "vegetable" | "vegetables" => "vegetable",
        "fruit" | "fruits" => "fruit",
        "grain" | "grains" | "grains_and_pasta" | "pasta" | "cereal" => "grain",
        "legume" | "legumes" => "legume",
        "nut" | "nuts" | "seeds" => "nut",
        "spice" | "spices" | "herb" | "herbs" | "seasoning" => "spice",
        "oil" | "oils" | "fat" | "fats" => "oil",
        "beverage" | "beverages" | "drink" => "beverage",
        "mushroom" | "mushrooms" | "fungi" => "vegetable",
        "other" => {
            return Err(AppError::validation(
                "product_type 'other' is not allowed. Every product must have a specific type \
                 (e.g. seafood, meat, dairy, vegetable, fruit, grain, legume, nut, spice, oil, beverage)."
            ));
        }
        unknown => {
            return Err(AppError::validation(&format!(
                "Unknown product_type '{}'. Allowed: seafood, meat, dairy, vegetable, fruit, \
                 grain, legume, nut, spice, oil, beverage.",
                unknown
            )));
        }
    };
    Ok(normalized.to_string())
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

    // Publication status
    pub is_published: bool,
    pub published_at: Option<String>,
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
        // 🔒 enforce_category: product_type is the source of truth for category
        let (final_category_id, final_unit, final_product_type) = if req.category_id.is_some()
            && req.unit.is_some()
        {
            // User provided explicit overrides
            let pt = if let Some(ref pt_raw) = req.product_type {
                normalize_product_type(pt_raw)?
            } else {
                // No product_type but category_id provided — allow but log warning
                tracing::warn!("Product created with explicit category_id but no product_type");
                "other".to_string() // Legacy path — will be fixed by data quality engine
            };
            (req.category_id.unwrap(), req.unit.unwrap(), pt)
        } else if let Some(ref pt_raw) = req.product_type {
            // 🔒 product_type provided (from AI draft) → enforce category mapping
            let pt = normalize_product_type(pt_raw)?;

            let category_name = product_type_to_category(&pt).ok_or_else(|| {
                AppError::validation(&format!(
                    "Cannot map product_type '{}' to a category",
                    pt
                ))
            })?;

            let cat_id = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM catalog_categories WHERE name_en = $1 LIMIT 1",
            )
            .bind(category_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("DB error resolving category for product_type '{}': {}", pt, e);
                AppError::internal("Failed to resolve category")
            })?
            .ok_or_else(|| {
                tracing::error!(
                    "Category '{}' not found in DB for product_type '{}'",
                    category_name,
                    pt
                );
                AppError::internal(&format!(
                    "Category '{}' not found in database. Please create it first.",
                    category_name
                ))
            })?;

            tracing::info!(
                "🔒 enforce_category: product_type '{}' → category '{}' ({})",
                pt,
                category_name,
                cat_id
            );

            // Unit from request or smart default
            let unit_resolved = if let Some(u) = req.unit {
                u
            } else {
                let default_unit = match pt.as_str() {
                    "seafood" | "meat" | "vegetable" | "fruit" | "grain" | "legume" | "nut"
                    | "spice" => "kilogram",
                    "dairy" => "liter",
                    "oil" | "beverage" => "liter",
                    _ => "kilogram",
                };
                UnitType::from_string(default_unit).unwrap_or(UnitType::from_string("kilogram").unwrap())
            };

            (cat_id, unit_resolved, pt)
        } else {
            // Legacy path: no product_type, no explicit category — use AI slug
            tracing::info!("Running unified AI processing for: {}", name_input);

            let cat_id = match self.find_category_by_slug(&category_slug).await {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!(
                        "Category '{}' not found, rejecting product creation",
                        category_slug
                    );
                    return Err(AppError::validation(&format!(
                        "Invalid category from AI: {}. Please provide explicit category_id or product_type",
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

            // Try to infer product_type from category_slug
            let inferred_pt = match category_slug.to_lowercase().as_str() {
                "seafood" | "fish_and_seafood" => "seafood",
                "meat" | "meat_and_poultry" => "meat",
                "dairy_and_eggs" => "dairy",
                "vegetables" => "vegetable",
                "fruits" => "fruit",
                "grains" | "grains_and_pasta" => "grain",
                "beverages" => "beverage",
                _ => "other", // Legacy — will be caught by data quality engine
            };

            (cat_id, unit_resolved, inferred_pt.to_string())
        };

        // ==========================================
        // 💾 ШАГ 5: СОХРАНЕНИЕ В БД
        // ==========================================
        let id = Uuid::new_v4();

        let product = sqlx::query_as::<_, ProductResponse>(
            r#"
            INSERT INTO catalog_ingredients (
                id, name_en, name_pl, name_uk, name_ru,
                category_id, default_unit, description, product_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image,
                      COALESCE(is_published, false) as is_published, published_at::text
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
        .bind(&final_product_type)
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
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(id)
        .bind(&product.slug)
        .bind(&name_en)
        .bind(&name_pl)
        .bind(&name_ru)
        .bind(&name_uk)
        .bind(final_category_id)
        .bind(&final_product_type)
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

        // ==========================================
        // 🗓️ ШАГ 8: AUTO-GENERATE FISH SEASONALITY CALENDAR
        // ==========================================
        // If product_type is fish/seafood → auto-generate 12 months × 4 regions
        tokio::spawn({
            let pool = self.pool.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                match crate::application::ai_sous_chef::fish_seasonality::generate_seasonality_for_product(&pool, id).await {
                    Ok(count) if count > 0 => {
                        tracing::info!("🗓️ Auto-generated {} seasonality rows for product {}", count, id);
                    }
                    Ok(_) => {} // Not fish/seafood — skipped silently
                    Err(e) => {
                        tracing::warn!("⚠️ Failed to auto-generate seasonality for {}: {}", id, e);
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
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image,
                      COALESCE(is_published, false) as is_published, published_at::text
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
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image,
                      COALESCE(is_published, false) as is_published, published_at::text
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
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image,
                      COALESCE(is_published, false) as is_published, published_at::text
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
        let old_product_type = product.product_type.clone();
        let prod_type = {
            let raw = req.product_type.unwrap_or(product.product_type);
            // 🔒 enforce_category on update: reject "other", normalize product_type
            if raw == "other" {
                tracing::warn!(
                    "⚠️ Product {} has product_type 'other' — allowing update but flagging",
                    id
                );
                // Don't block update of existing "other" products, but keep the value
                // Data Quality Engine will flag it
                raw
            } else {
                // Validate and normalize
                match normalize_product_type(&raw) {
                    Ok(normalized) => normalized,
                    Err(_) => {
                        tracing::warn!("Unknown product_type '{}' for product {}, keeping as-is", raw, id);
                        raw
                    }
                }
            }
        };
        let avail_model = req.availability_model.unwrap_or(product.availability_model);

        // 🔒 enforce_category: if product_type changed, auto-fix category to match
        let final_category_id = if let Some(explicit_cat) = req.category_id {
            explicit_cat
        } else if prod_type != "other" {
            // product_type is set and not "other" — enforce correct category
            if let Some(correct_cat_name) = product_type_to_category(&prod_type) {
                match sqlx::query_scalar::<_, Uuid>(
                    "SELECT id FROM catalog_categories WHERE name_en = $1 LIMIT 1",
                )
                .bind(correct_cat_name)
                .fetch_optional(&self.pool)
                .await
                {
                    Ok(Some(cat_id)) => {
                        if cat_id != product.category_id {
                            tracing::info!(
                                "🔒 enforce_category on update: product_type '{}' → category '{}' (was {})",
                                prod_type,
                                correct_cat_name,
                                product.category_id
                            );
                        }
                        cat_id
                    }
                    _ => product.category_id, // Can't resolve — keep existing
                }
            } else {
                product.category_id
            }
        } else {
            product.category_id
        };

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
                      seo_title, seo_description, seo_h1, canonical_url, og_title, og_description, og_image,
                      COALESCE(is_published, false) as is_published, published_at::text
            "#
        )
        .bind(&name_en)
        .bind(&name_pl)
        .bind(&name_uk)
        .bind(&name_ru)
        .bind(final_category_id)
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

        // 🗓️ Auto-generate seasonality if product_type changed TO fish/seafood
        if prod_type != old_product_type
            && (prod_type == "fish" || prod_type == "seafood")
        {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                match crate::application::ai_sous_chef::fish_seasonality::generate_seasonality_for_product(&pool, id).await {
                    Ok(count) if count > 0 => {
                        tracing::info!("🗓️ Reclassified → auto-generated {} seasonality rows for {}", count, id);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("⚠️ Failed to generate seasonality on reclassify for {}: {}", id, e);
                    }
                }
            });
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

    // ── Publication Flow ──────────────────────────────────────────────

    /// Publish a product — makes it visible in the public blog/catalog.
    ///
    /// Rules:
    /// - Product must have calories_per_100g (at minimum)
    /// - Product must have product_type != 'other'
    /// - Product must have at least name_en + name_ru
    pub async fn publish_product(&self, id: Uuid) -> AppResult<ProductResponse> {
        // 1. Fetch product to validate readiness
        let product = self.get_product_by_id(id).await?;

        // 2. Validate minimum requirements for publication
        let mut errors = Vec::new();
        if product.product_type == "other" {
            errors.push("product_type is 'other' — must be a specific type");
        }
        if product.calories_per_100g.is_none() {
            errors.push("calories_per_100g is missing");
        }
        if product.name_en.trim().is_empty() {
            errors.push("name_en is empty");
        }
        if product.name_ru.as_deref().unwrap_or("").trim().is_empty() {
            errors.push("name_ru is empty");
        }

        if !errors.is_empty() {
            return Err(AppError::validation(&format!(
                "Cannot publish: {}",
                errors.join(", ")
            )));
        }

        // 3. Set is_published = true, published_at = now()
        sqlx::query(
            "UPDATE catalog_ingredients SET is_published = true, published_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("DB error publishing product {}: {}", id, e);
            AppError::internal("Failed to publish product")
        })?;

        tracing::info!("✅ Product {} ('{}') published to blog", id, product.name_en);

        // Ping blog to revalidate sitemap + product pages immediately
        let slug = product.slug.clone();
        tokio::spawn(revalidate_blog(slug));

        // Return updated product
        self.get_product_by_id(id).await
    }

    /// Unpublish a product — removes it from the public blog/catalog.
    pub async fn unpublish_product(&self, id: Uuid) -> AppResult<ProductResponse> {
        // Verify exists
        let product = self.get_product_by_id(id).await?;

        sqlx::query(
            "UPDATE catalog_ingredients SET is_published = false WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("DB error unpublishing product {}: {}", id, e);
            AppError::internal("Failed to unpublish product")
        })?;

        tracing::info!("Product {} unpublished from blog", id);

        // Ping blog to revalidate sitemap (product removed)
        let slug = product.slug.clone();
        tokio::spawn(revalidate_blog(slug));

        self.get_product_by_id(id).await
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
            "legumes" => "Legumes",
            "nuts" | "nuts_and_seeds" => "Nuts & Seeds",
            "spices" | "spices_and_herbs" => "Spices & Herbs",
            "oils" | "oils_and_fats" => "Oils & Fats",
            _ => {
                // 🔒 No more defaulting to Vegetables! Return error.
                tracing::error!(
                    "❌ Unknown category slug '{}' — refusing to default to Vegetables",
                    slug
                );
                return Err(AppError::validation(&format!(
                    "Unknown category '{}'. Please provide a valid product_type or category_id.",
                    slug
                )));
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
}

// ── Blog revalidation ─────────────────────────────────────────────────────────

/// Ping Next.js ISR revalidation endpoint so the sitemap + ingredient pages
/// refresh immediately after publish/unpublish — no waiting for revalidate timer.
///
/// Uses env vars:
///   BLOG_URL                (default: https://dima-fomin.pl)
///   BLOG_REVALIDATE_SECRET  (default: fodi-revalidate-2025-secret)
pub async fn revalidate_blog(slug: Option<String>) {
    let blog_url = std::env::var("BLOG_URL")
        .unwrap_or_else(|_| "https://dima-fomin.pl".to_string());
    let secret = std::env::var("BLOG_REVALIDATE_SECRET")
        .unwrap_or_else(|_| "fodi-revalidate-2025-secret".to_string());

    let url = format!("{}/api/revalidate", blog_url);

    // Build tag-based payload: always invalidate "ingredients" tag + specific product
    let mut tags = vec!["ingredients".to_string()];
    let mut paths = vec!["/chef-tools/ingredients".to_string()];

    if let Some(ref s) = slug {
        tags.push(format!("ingredient-{}", s));
        paths.push(format!("/chef-tools/nutrition/{}", s));
        paths.push(format!("/chef-tools/ingredients/{}", s));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    // Try tag-based first (preferred, faster)
    let body = serde_json::json!({ "tags": tags, "paths": paths });
    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", secret))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => tracing::info!(
            "🔄 Blog revalidated (tags={:?}, paths={:?}) → {}",
            tags, paths, resp.status()
        ),
        Err(e) => tracing::warn!("⚠️ Blog revalidate failed: {}", e),
    }
}
