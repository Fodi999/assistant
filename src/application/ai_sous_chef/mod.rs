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

/// Data quality score for a product (backend = source of truth)
#[derive(Debug, Serialize, Clone)]
pub struct DataQualityRow {
    pub id: Uuid,
    pub name_en: String,
    pub product_type: String,
    pub score: f64,
    pub filled: i64,
    pub total: i64,
    pub missing_fields: Vec<MissingField>,
    pub status: &'static str, // "complete" | "optional_missing" | "critical_missing"
}

/// A single missing field with metadata
#[derive(Debug, Serialize, Clone)]
pub struct MissingField {
    pub field: &'static str,
    pub label_ru: &'static str,
    pub group: &'static str, // "basic" | "nutrition" | "seo" | "relations"
    pub severity: &'static str, // "critical" | "recommended" | "optional"
}

/// Raw DB row for data quality query
#[derive(Debug, sqlx::FromRow)]
struct DataQualityRaw {
    pub id: Uuid,
    pub name_en: String,
    pub product_type: String,
    // basic
    pub has_image: bool,
    pub has_name_ru: bool,
    pub has_name_pl: bool,
    pub has_name_uk: bool,
    pub has_description_en: bool,
    pub has_description_ru: bool,
    pub has_slug: bool,
    // nutrition
    pub has_calories: bool,
    pub has_protein: bool,
    pub has_fat: bool,
    pub has_carbs: bool,
    pub has_fiber: bool,
    pub has_density: bool,
    pub has_shelf_life: bool,
    pub has_typical_portion: bool,
    // seo
    pub has_seo_title: bool,
    // relations
    pub has_macros: bool,
    pub has_vitamins: bool,
    pub has_minerals: bool,
    pub has_allergens: bool,
    pub has_diet_flags: bool,
    pub has_culinary: bool,
    pub has_food_props: bool,
    pub has_states: bool,
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

    /// Get data quality scores for all products (backend = source of truth)
    pub async fn data_quality(&self) -> AppResult<Vec<DataQualityRow>> {
        let raw_rows = Self::fetch_quality_raw(&self.pool, None).await?;
        Ok(raw_rows.into_iter().map(Self::build_quality_row).collect())
    }

    /// Get data quality for a SINGLE product (for re-validation after save)
    pub async fn data_quality_single(&self, product_id: Uuid) -> AppResult<DataQualityRow> {
        let raw_rows = Self::fetch_quality_raw(&self.pool, Some(product_id)).await?;
        raw_rows
            .into_iter()
            .next()
            .map(Self::build_quality_row)
            .ok_or_else(|| crate::shared::AppError::not_found("Product not found"))
    }

    /// Fetch raw quality booleans from DB
    async fn fetch_quality_raw(pool: &PgPool, product_id: Option<Uuid>) -> AppResult<Vec<DataQualityRaw>> {
        let rows = sqlx::query_as::<_, DataQualityRaw>(
            r#"
            SELECT
                ci.id,
                ci.name_en,
                COALESCE(ci.product_type, 'other') as product_type,
                -- basic
                (ci.image_url IS NOT NULL AND ci.image_url != '') as has_image,
                (ci.name_ru IS NOT NULL AND ci.name_ru != '') as has_name_ru,
                (ci.name_pl IS NOT NULL AND ci.name_pl != '') as has_name_pl,
                (ci.name_uk IS NOT NULL AND ci.name_uk != '') as has_name_uk,
                (ci.description_en IS NOT NULL AND ci.description_en != '') as has_description_en,
                (ci.description_ru IS NOT NULL AND ci.description_ru != '') as has_description_ru,
                (ci.slug IS NOT NULL AND ci.slug != '') as has_slug,
                -- nutrition
                (ci.calories_per_100g IS NOT NULL) as has_calories,
                (ci.protein_per_100g IS NOT NULL) as has_protein,
                (ci.fat_per_100g IS NOT NULL) as has_fat,
                (ci.carbs_per_100g IS NOT NULL) as has_carbs,
                (ci.fiber_per_100g IS NOT NULL) as has_fiber,
                (ci.density_g_per_ml IS NOT NULL) as has_density,
                (ci.shelf_life_days IS NOT NULL) as has_shelf_life,
                (ci.typical_portion_g IS NOT NULL) as has_typical_portion,
                -- seo
                (ci.seo_title IS NOT NULL AND ci.seo_title != '') as has_seo_title,
                -- relations
                EXISTS(SELECT 1 FROM nutrition_macros WHERE product_id = ci.id) as has_macros,
                EXISTS(SELECT 1 FROM nutrition_vitamins WHERE product_id = ci.id) as has_vitamins,
                EXISTS(SELECT 1 FROM nutrition_minerals WHERE product_id = ci.id) as has_minerals,
                EXISTS(SELECT 1 FROM product_allergens WHERE product_id = ci.id) as has_allergens,
                EXISTS(SELECT 1 FROM diet_flags WHERE product_id = ci.id) as has_diet_flags,
                EXISTS(SELECT 1 FROM food_culinary_properties WHERE product_id = ci.id) as has_culinary,
                EXISTS(SELECT 1 FROM food_properties WHERE product_id = ci.id) as has_food_props,
                ((SELECT COUNT(*) FROM ingredient_states WHERE ingredient_id = ci.id) >= 10) as has_states
            FROM catalog_ingredients ci
            WHERE ci.is_active = true
              AND ($1::uuid IS NULL OR ci.id = $1)
            ORDER BY ci.name_en ASC
            "#,
        )
        .bind(product_id)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// Categories where fiber is naturally 0 (animal products)
    const FIBER_NOT_APPLICABLE: &'static [&'static str] = &[
        "fish", "seafood", "meat", "poultry", "eggs", "dairy",
        "fish_and_seafood", "meat_and_poultry", "dairy_and_eggs",
    ];

    /// Build DataQualityRow from raw booleans — defines ALL field checks in one place
    fn build_quality_row(r: DataQualityRaw) -> DataQualityRow {
        // Rule: fiber is OK for animal products even if null (naturally 0)
        let fiber_ok = r.has_fiber
            || Self::FIBER_NOT_APPLICABLE.iter().any(|&cat| {
                r.product_type.eq_ignore_ascii_case(cat)
            });

        // Define all checks: (ok, field, label_ru, group, severity)
        let checks: Vec<(bool, &'static str, &'static str, &'static str, &'static str)> = vec![
            // ── basic (critical) ──
            (r.has_image,          "image_url",       "Фото",          "basic",     "critical"),
            (r.has_name_ru,        "name_ru",         "Название RU",   "basic",     "critical"),
            (r.has_name_pl,        "name_pl",         "Название PL",   "basic",     "critical"),
            (r.has_name_uk,        "name_uk",         "Название UK",   "basic",     "critical"),
            (r.has_description_en, "description_en",  "Описание EN",   "basic",     "critical"),
            (r.has_description_ru, "description_ru",  "Описание RU",   "basic",     "critical"),
            (r.has_slug,           "slug",            "Slug",          "basic",     "critical"),
            // ── nutrition (critical) ──
            (r.has_calories,       "calories_per_100g", "Калории",     "nutrition", "critical"),
            (r.has_protein,        "protein_per_100g",  "Белки",       "nutrition", "critical"),
            (r.has_fat,            "fat_per_100g",      "Жиры",        "nutrition", "critical"),
            (r.has_carbs,          "carbs_per_100g",    "Углеводы",    "nutrition", "critical"),
            // ── nutrition (recommended) — fiber uses category-aware rule ──
            (fiber_ok,             "fiber_per_100g",    "Клетчатка",   "nutrition", "recommended"),
            (r.has_density,        "density_g_per_ml",  "Плотность",   "nutrition", "recommended"),
            (r.has_shelf_life,     "shelf_life_days",   "Срок годности", "nutrition", "recommended"),
            (r.has_typical_portion,"typical_portion_g", "Типичная порция","nutrition","recommended"),
            // ── seo (recommended) ──
            (r.has_seo_title,      "seo_title",       "SEO Title",    "seo",       "recommended"),
            // ── relations (optional) ──
            (r.has_macros,         "nutrition_macros",  "Макронутриенты (таблица)", "relations", "optional"),
            (r.has_vitamins,       "nutrition_vitamins","Витамины",     "relations", "optional"),
            (r.has_minerals,       "nutrition_minerals","Минералы",     "relations", "optional"),
            (r.has_allergens,      "product_allergens", "Аллергены",    "relations", "optional"),
            (r.has_diet_flags,     "diet_flags",        "Диет-флаги",   "relations", "optional"),
            (r.has_culinary,       "food_culinary",     "Кулинарные свойства", "relations", "optional"),
            (r.has_food_props,     "food_properties",   "Свойства продукта",   "relations", "optional"),
            (r.has_states,         "ingredient_states", "Состояния (≥10)", "relations", "optional"),
        ];

        let total = checks.len() as i64;
        let filled = checks.iter().filter(|(ok, ..)| *ok).count() as i64;
        let score = if total > 0 {
            ((filled as f64 / total as f64) * 1000.0).round() / 10.0
        } else {
            0.0
        };

        let missing_fields: Vec<MissingField> = checks
            .iter()
            .filter(|(ok, ..)| !ok)
            .map(|(_, field, label_ru, group, severity)| MissingField {
                field,
                label_ru,
                group,
                severity,
            })
            .collect();

        let has_critical_missing = missing_fields.iter().any(|f| f.severity == "critical");
        let status = if missing_fields.is_empty() {
            "complete"
        } else if has_critical_missing {
            "critical_missing"
        } else {
            "optional_missing"
        };

        DataQualityRow {
            id: r.id,
            name_en: r.name_en,
            product_type: r.product_type,
            score,
            filled,
            total,
            missing_fields,
            status,
        }
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
