pub mod generate_states;
pub mod nutrition_transform;
pub mod scan_products;
pub mod storage_rules;
pub mod translation_rules;

use crate::shared::AppResult;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Catalog Rule Bot Service — orchestrates state generation, rules, and scanning
#[derive(Clone)]
pub struct CatalogRuleBotService {
    pool: PgPool,
}

/// Result of state generation for a single ingredient
#[derive(Debug, Serialize)]
pub struct GenerateStatesResult {
    pub ingredient_id: Uuid,
    pub name_en: String,
    pub states_created: Vec<String>,
    pub states_total: usize,
}

/// Result of bulk state generation
#[derive(Debug, Serialize)]
pub struct BulkGenerateResult {
    pub total_ingredients: usize,
    pub ingredients_processed: usize,
    pub states_created: usize,
    pub errors: Vec<String>,
}

/// Data quality score for a product
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DataQualityRow {
    pub id: Uuid,
    pub name_en: String,
    pub product_type: String,
    pub score: f64,
    pub filled: i64,
    pub total: i64,
}

impl CatalogRuleBotService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Access the pool (for direct queries from handlers)
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Generate all missing states for ONE ingredient
    pub async fn generate_states_for(
        &self,
        ingredient_id: Uuid,
    ) -> AppResult<GenerateStatesResult> {
        // Get name
        let name: String = sqlx::query_scalar(
            "SELECT name_en FROM catalog_ingredients WHERE id = $1 AND is_active = true",
        )
        .bind(ingredient_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| crate::shared::AppError::not_found("Ingredient not found"))?;

        let created = generate_states::generate_states_for_ingredient(&self.pool, ingredient_id).await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ingredient_states WHERE ingredient_id = $1",
        )
        .bind(ingredient_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(GenerateStatesResult {
            ingredient_id,
            name_en: name,
            states_created: created.iter().map(|s| s.as_str().to_string()).collect(),
            states_total: total as usize,
        })
    }

    /// Bulk generate states for ALL ingredients missing any state
    pub async fn generate_all_states(&self) -> AppResult<BulkGenerateResult> {
        let ids = scan_products::get_ingredients_missing_states(&self.pool).await?;
        let total = ids.len();

        let mut processed = 0usize;
        let mut total_created = 0usize;
        let mut errors = Vec::new();

        for id in ids {
            match generate_states::generate_states_for_ingredient(&self.pool, id).await {
                Ok(created) => {
                    total_created += created.len();
                    processed += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", id, e));
                }
            }
        }

        tracing::info!(
            "✅ Bulk generation complete: {}/{} processed, {} states created, {} errors",
            processed, total, total_created, errors.len()
        );

        Ok(BulkGenerateResult {
            total_ingredients: total,
            ingredients_processed: processed,
            states_created: total_created,
            errors,
        })
    }

    /// Get state audit report
    pub async fn state_audit(&self) -> AppResult<scan_products::CatalogStateAudit> {
        scan_products::scan_catalog_states(&self.pool).await
    }

    /// Get data quality scores for all products
    pub async fn data_quality(&self) -> AppResult<Vec<DataQualityRow>> {
        let rows = sqlx::query_as::<_, DataQualityRow>(
            r#"
            SELECT
                ci.id,
                ci.name_en,
                COALESCE(ci.product_type, 'other') as product_type,
                -- Count filled fields out of key fields
                (
                    CASE WHEN ci.calories_per_100g IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN ci.protein_per_100g IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN ci.fat_per_100g IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN ci.carbs_per_100g IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN ci.image_url IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN ci.description_en IS NOT NULL AND ci.description_en != '' THEN 1 ELSE 0 END +
                    CASE WHEN ci.description_ru IS NOT NULL AND ci.description_ru != '' THEN 1 ELSE 0 END +
                    CASE WHEN ci.description_pl IS NOT NULL AND ci.description_pl != '' THEN 1 ELSE 0 END +
                    CASE WHEN ci.description_uk IS NOT NULL AND ci.description_uk != '' THEN 1 ELSE 0 END +
                    CASE WHEN ci.density_g_per_ml IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN ci.shelf_life_days IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN ci.typical_portion_g IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN ci.seo_title IS NOT NULL AND ci.seo_title != '' THEN 1 ELSE 0 END +
                    CASE WHEN EXISTS(SELECT 1 FROM nutrition_macros WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                    CASE WHEN EXISTS(SELECT 1 FROM nutrition_vitamins WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                    CASE WHEN EXISTS(SELECT 1 FROM nutrition_minerals WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                    CASE WHEN EXISTS(SELECT 1 FROM product_allergens WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                    CASE WHEN EXISTS(SELECT 1 FROM diet_flags WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                    CASE WHEN EXISTS(SELECT 1 FROM food_culinary_properties WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                    CASE WHEN EXISTS(SELECT 1 FROM food_properties WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                    CASE WHEN (SELECT COUNT(*) FROM ingredient_states WHERE ingredient_id = ci.id) >= 10 THEN 1 ELSE 0 END
                )::float8 as filled,
                21::bigint as total,
                ROUND(
                    (
                        CASE WHEN ci.calories_per_100g IS NOT NULL THEN 1 ELSE 0 END +
                        CASE WHEN ci.protein_per_100g IS NOT NULL THEN 1 ELSE 0 END +
                        CASE WHEN ci.fat_per_100g IS NOT NULL THEN 1 ELSE 0 END +
                        CASE WHEN ci.carbs_per_100g IS NOT NULL THEN 1 ELSE 0 END +
                        CASE WHEN ci.image_url IS NOT NULL THEN 1 ELSE 0 END +
                        CASE WHEN ci.description_en IS NOT NULL AND ci.description_en != '' THEN 1 ELSE 0 END +
                        CASE WHEN ci.description_ru IS NOT NULL AND ci.description_ru != '' THEN 1 ELSE 0 END +
                        CASE WHEN ci.description_pl IS NOT NULL AND ci.description_pl != '' THEN 1 ELSE 0 END +
                        CASE WHEN ci.description_uk IS NOT NULL AND ci.description_uk != '' THEN 1 ELSE 0 END +
                        CASE WHEN ci.density_g_per_ml IS NOT NULL THEN 1 ELSE 0 END +
                        CASE WHEN ci.shelf_life_days IS NOT NULL THEN 1 ELSE 0 END +
                        CASE WHEN ci.typical_portion_g IS NOT NULL THEN 1 ELSE 0 END +
                        CASE WHEN ci.seo_title IS NOT NULL AND ci.seo_title != '' THEN 1 ELSE 0 END +
                        CASE WHEN EXISTS(SELECT 1 FROM nutrition_macros WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                        CASE WHEN EXISTS(SELECT 1 FROM nutrition_vitamins WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                        CASE WHEN EXISTS(SELECT 1 FROM nutrition_minerals WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                        CASE WHEN EXISTS(SELECT 1 FROM product_allergens WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                        CASE WHEN EXISTS(SELECT 1 FROM diet_flags WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                        CASE WHEN EXISTS(SELECT 1 FROM food_culinary_properties WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                        CASE WHEN EXISTS(SELECT 1 FROM food_properties WHERE product_id = ci.id) THEN 1 ELSE 0 END +
                        CASE WHEN (SELECT COUNT(*) FROM ingredient_states WHERE ingredient_id = ci.id) >= 10 THEN 1 ELSE 0 END
                    )::numeric / 21.0 * 100, 1
                )::float8 as score
            FROM catalog_ingredients ci
            WHERE ci.is_active = true
            ORDER BY score ASC, ci.name_en ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get states for a specific ingredient
    pub async fn get_states(
        &self,
        ingredient_id: Uuid,
    ) -> AppResult<Vec<IngredientStateRow>> {
        let rows = sqlx::query_as::<_, IngredientStateRow>(
            r#"SELECT
                id, ingredient_id, state::text as state,
                calories_per_100g, protein_per_100g, fat_per_100g,
                carbs_per_100g, fiber_per_100g, water_percent,
                shelf_life_hours, storage_temp_c, texture,
                name_suffix_en, name_suffix_pl, name_suffix_ru, name_suffix_uk,
                notes_en, notes_pl, notes_ru, notes_uk,
                notes, generated_by, data_score,
                created_at, updated_at
            FROM ingredient_states
            WHERE ingredient_id = $1
            ORDER BY
                CASE state
                    WHEN 'raw' THEN 0
                    WHEN 'boiled' THEN 1
                    WHEN 'steamed' THEN 2
                    WHEN 'baked' THEN 3
                    WHEN 'grilled' THEN 4
                    WHEN 'fried' THEN 5
                    WHEN 'smoked' THEN 6
                    WHEN 'frozen' THEN 7
                    WHEN 'dried' THEN 8
                    WHEN 'pickled' THEN 9
                END"#,
        )
        .bind(ingredient_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

/// Full state row for API response
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct IngredientStateRow {
    pub id: Uuid,
    pub ingredient_id: Uuid,
    pub state: String,
    pub calories_per_100g: Option<f64>,
    pub protein_per_100g: Option<f64>,
    pub fat_per_100g: Option<f64>,
    pub carbs_per_100g: Option<f64>,
    pub fiber_per_100g: Option<f64>,
    pub water_percent: Option<f64>,
    pub shelf_life_hours: Option<i32>,
    pub storage_temp_c: Option<i32>,
    pub texture: Option<String>,
    pub name_suffix_en: Option<String>,
    pub name_suffix_pl: Option<String>,
    pub name_suffix_ru: Option<String>,
    pub name_suffix_uk: Option<String>,
    pub notes_en: Option<String>,
    pub notes_pl: Option<String>,
    pub notes_ru: Option<String>,
    pub notes_uk: Option<String>,
    pub notes: Option<String>,
    pub generated_by: Option<String>,
    pub data_score: Option<f64>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}
