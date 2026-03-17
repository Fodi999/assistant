pub mod generate_states;
pub mod nutrition_transform;
pub mod scan_products;
pub mod storage_rules;
pub mod translation_rules;

// 🆕 AI Use Cases — Clean Architecture (replaces flat files)
// Each use-case: cache check → AiClient trait → parse → cache store
pub mod use_cases;

use crate::shared::AppResult;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// AI Sous Chef Service — orchestrates state generation, rules, and scanning
#[derive(Clone)]
pub struct AiSousChefService {
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

/// Admin request to update a single ingredient state
#[derive(Debug, Deserialize)]
pub struct UpdateStateRequest {
    pub calories_per_100g: Option<f64>,
    pub protein_per_100g: Option<f64>,
    pub fat_per_100g: Option<f64>,
    pub carbs_per_100g: Option<f64>,
    pub fiber_per_100g: Option<f64>,
    pub water_percent: Option<f64>,
    pub shelf_life_hours: Option<i32>,
    pub storage_temp_c: Option<i32>,
    pub texture: Option<String>,
    pub weight_change_percent: Option<f64>,
    pub state_type: Option<String>,
    pub oil_absorption_g: Option<f64>,
    pub water_loss_percent: Option<f64>,
    pub glycemic_index: Option<i16>,
    pub cooking_method: Option<String>,
    pub notes_en: Option<String>,
    pub notes_pl: Option<String>,
    pub notes_ru: Option<String>,
    pub notes_uk: Option<String>,
    pub name_suffix_en: Option<String>,
    pub name_suffix_pl: Option<String>,
    pub name_suffix_ru: Option<String>,
    pub name_suffix_uk: Option<String>,
}

impl AiSousChefService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Access the pool (for direct queries from handlers)
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Update a single state field-by-field (admin manual edit)
    pub async fn update_state(
        &self,
        ingredient_id: Uuid,
        state_name: &str,
        req: UpdateStateRequest,
    ) -> AppResult<IngredientStateRow> {
        // Verify the state row exists
        let row_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ingredient_states WHERE ingredient_id = $1 AND state = $2::processing_state",
        )
        .bind(ingredient_id)
        .bind(state_name)
        .fetch_optional(&self.pool)
        .await?;

        let row_id = row_id.ok_or_else(|| {
            crate::shared::AppError::not_found("State not found for this ingredient")
        })?;

        // Build dynamic UPDATE — only set provided fields
        sqlx::query(
            r#"UPDATE ingredient_states SET
                calories_per_100g   = COALESCE($2, calories_per_100g),
                protein_per_100g    = COALESCE($3, protein_per_100g),
                fat_per_100g        = COALESCE($4, fat_per_100g),
                carbs_per_100g      = COALESCE($5, carbs_per_100g),
                fiber_per_100g      = COALESCE($6, fiber_per_100g),
                water_percent       = COALESCE($7, water_percent),
                shelf_life_hours    = COALESCE($8, shelf_life_hours),
                storage_temp_c      = COALESCE($9, storage_temp_c),
                texture             = COALESCE($10, texture),
                weight_change_percent = COALESCE($11, weight_change_percent),
                state_type          = COALESCE($12, state_type),
                oil_absorption_g    = COALESCE($13, oil_absorption_g),
                water_loss_percent  = COALESCE($14, water_loss_percent),
                glycemic_index      = COALESCE($15, glycemic_index),
                cooking_method      = COALESCE($16::cooking_method_enum, cooking_method),
                notes_en            = COALESCE($17, notes_en),
                notes_pl            = COALESCE($18, notes_pl),
                notes_ru            = COALESCE($19, notes_ru),
                notes_uk            = COALESCE($20, notes_uk),
                name_suffix_en      = COALESCE($21, name_suffix_en),
                name_suffix_pl      = COALESCE($22, name_suffix_pl),
                name_suffix_ru      = COALESCE($23, name_suffix_ru),
                name_suffix_uk      = COALESCE($24, name_suffix_uk),
                generated_by        = 'admin_edit',
                updated_at          = now()
            WHERE id = $1"#,
        )
        .bind(row_id)
        .bind(req.calories_per_100g)
        .bind(req.protein_per_100g)
        .bind(req.fat_per_100g)
        .bind(req.carbs_per_100g)
        .bind(req.fiber_per_100g)
        .bind(req.water_percent)
        .bind(req.shelf_life_hours)
        .bind(req.storage_temp_c)
        .bind(&req.texture)
        .bind(req.weight_change_percent)
        .bind(&req.state_type)
        .bind(req.oil_absorption_g)
        .bind(req.water_loss_percent)
        .bind(req.glycemic_index)
        .bind(&req.cooking_method)
        .bind(&req.notes_en)
        .bind(&req.notes_pl)
        .bind(&req.notes_ru)
        .bind(&req.notes_uk)
        .bind(&req.name_suffix_en)
        .bind(&req.name_suffix_pl)
        .bind(&req.name_suffix_ru)
        .bind(&req.name_suffix_uk)
        .execute(&self.pool)
        .await?;

        // Re-fetch and return updated row
        let updated = sqlx::query_as::<_, IngredientStateRow>(
            r#"SELECT
                id, ingredient_id, state::text as state,
                calories_per_100g::float8, protein_per_100g::float8, fat_per_100g::float8,
                carbs_per_100g::float8, fiber_per_100g::float8, water_percent::float8,
                shelf_life_hours, storage_temp_c, texture,
                weight_change_percent::float8, state_type, oil_absorption_g::float8, water_loss_percent::float8,
                glycemic_index, cooking_method::text as cooking_method,
                name_suffix_en, name_suffix_pl, name_suffix_ru, name_suffix_uk,
                notes_en, notes_pl, notes_ru, notes_uk,
                notes, generated_by, data_score::float8,
                to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as updated_at
            FROM ingredient_states
            WHERE id = $1"#,
        )
        .bind(row_id)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!("✏️ Admin edited state {} / {} → generated_by=admin_edit", ingredient_id, state_name);

        Ok(updated)
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
                calories_per_100g::float8, protein_per_100g::float8, fat_per_100g::float8,
                carbs_per_100g::float8, fiber_per_100g::float8, water_percent::float8,
                shelf_life_hours, storage_temp_c, texture,
                weight_change_percent::float8, state_type, oil_absorption_g::float8, water_loss_percent::float8,
                glycemic_index, cooking_method::text as cooking_method,
                name_suffix_en, name_suffix_pl, name_suffix_ru, name_suffix_uk,
                notes_en, notes_pl, notes_ru, notes_uk,
                notes, generated_by, data_score::float8,
                to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as updated_at
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
    pub weight_change_percent: Option<f64>,
    pub state_type: Option<String>,
    pub oil_absorption_g: Option<f64>,
    pub water_loss_percent: Option<f64>,
    pub glycemic_index: Option<i16>,
    pub cooking_method: Option<String>,
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
    pub created_at: String,
    pub updated_at: String,
}
