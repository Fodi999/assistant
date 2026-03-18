pub mod fish_seasonality;
pub mod generate_states;
pub mod nutrition_transform;
pub mod product_dictionary;
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

/// Field requirement level — determines how a field is treated per product type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    Required,
    Recommended,
    Optional,
    NotApplicable,
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
    /// Weighted score: Required=1.0, Recommended=0.6, Optional=0.3
    pub weighted_score: f64,
    pub weight_filled: f64,
    pub weight_total: f64,
    pub missing_fields: Vec<MissingField>,
    pub not_applicable_fields: Vec<NotApplicableField>,
    pub next_actions: Vec<NextAction>,
    pub status: &'static str, // "complete" | "optional_missing" | "critical_missing"
}

/// A prioritised next action for improving product data quality
#[derive(Debug, Serialize, Clone)]
pub struct NextAction {
    pub field: &'static str,
    pub label_ru: &'static str,
    pub priority: &'static str,    // "high" | "medium" | "low"
    pub reason: &'static str,
    pub fix_hint: &'static str,    // auto-fix suggestion
}

/// A single missing field with metadata
#[derive(Debug, Serialize, Clone)]
pub struct MissingField {
    pub field: &'static str,
    pub label_ru: &'static str,
    pub group: &'static str,      // "basic" | "nutrition" | "seo" | "relations"
    pub severity: &'static str,   // "critical" | "recommended" | "optional"
}

/// A field that is not applicable to this product type (excluded from score)
#[derive(Debug, Serialize, Clone)]
pub struct NotApplicableField {
    pub field: &'static str,
    pub label_ru: &'static str,
    pub reason: &'static str,
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
    // fish/seafood-specific
    pub has_water_type: bool,
    pub has_wild_farmed: bool,
    pub has_sushi_grade: bool,
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
                -- fish/seafood-specific
                (ci.water_type IS NOT NULL AND ci.water_type != '') as has_water_type,
                (ci.wild_farmed IS NOT NULL AND ci.wild_farmed != '') as has_wild_farmed,
                (ci.sushi_grade IS NOT NULL) as has_sushi_grade,
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

    // ── Product type category sets for Field Requirement rules ──────────

    /// Animal products — fiber is not applicable
    const ANIMAL_TYPES: &'static [&'static str] = &[
        "fish", "seafood", "meat", "poultry", "eggs", "dairy",
        "fish_and_seafood", "meat_and_poultry", "dairy_and_eggs",
    ];

    /// Aquatic products — water_type / wild_farmed / sushi_grade are applicable
    const AQUATIC_TYPES: &'static [&'static str] = &[
        "fish", "seafood", "fish_and_seafood",
    ];

    /// Check if product_type matches any of the given categories
    fn type_matches(product_type: &str, categories: &[&str]) -> bool {
        categories.iter().any(|&cat| product_type.eq_ignore_ascii_case(cat))
    }

    /// ─── Field Requirement Rules Engine ─────────────────────────────────
    ///
    /// Returns Some(requirement) if there's a category-specific override,
    /// or None to use the field's default severity from the checks table.
    fn field_requirement(field: &str, product_type: &str) -> Option<Requirement> {
        match field {
            // ── Fiber: required for plants, not applicable for animals ──
            "fiber_per_100g" => {
                if Self::type_matches(product_type, Self::ANIMAL_TYPES) {
                    Some(Requirement::NotApplicable)
                } else if Self::type_matches(product_type, &[
                    // singular (DB values) + plural (legacy/AI slugs)
                    "vegetable", "vegetables",
                    "fruit", "fruits",
                    "grain", "grains", "grains_and_pasta",
                    "legume", "legumes",
                    "nut", "nuts",
                ]) {
                    Some(Requirement::Required)
                } else {
                    None // use default (recommended)
                }
            }

            // ── Fish/seafood-specific fields: only for aquatic ──
            "water_type" | "wild_farmed" | "sushi_grade" => {
                if Self::type_matches(product_type, Self::AQUATIC_TYPES) {
                    None // use default (recommended)
                } else {
                    Some(Requirement::NotApplicable)
                }
            }

            // ── No override — use the default severity from the table ──
            _ => None,
        }
    }

    /// Build DataQualityRow from raw booleans — Field Requirement Level system
    ///
    /// Each field is evaluated:
    ///   NotApplicable → excluded from total & filled, added to not_applicable_fields
    ///   Required/Recommended/Optional → counted; if missing → added to missing_fields
    ///
    /// Weighted scoring: Required=1.0, Recommended=0.6, Optional=0.3
    fn build_quality_row(r: DataQualityRaw) -> DataQualityRow {
        let pt = &r.product_type;

        // Define all fields: (has_value, field, label_ru, group, default_severity)
        // default_severity is used when field_requirement() returns Required (placeholder)
        let all_fields: Vec<(bool, &'static str, &'static str, &'static str, &'static str, &'static str)> = vec![
            // (has_value, field_key, label_ru, group, default_severity, na_reason)
            // ── basic (critical) ──
            (r.has_image,          "image_url",       "Фото",          "basic",     "critical",    ""),
            (r.has_name_ru,        "name_ru",         "Название RU",   "basic",     "critical",    ""),
            (r.has_name_pl,        "name_pl",         "Название PL",   "basic",     "critical",    ""),
            (r.has_name_uk,        "name_uk",         "Название UK",   "basic",     "critical",    ""),
            (r.has_description_en, "description_en",  "Описание EN",   "basic",     "critical",    ""),
            (r.has_description_ru, "description_ru",  "Описание RU",   "basic",     "critical",    ""),
            (r.has_slug,           "slug",            "Slug",          "basic",     "critical",    ""),
            // ── nutrition (critical) ──
            (r.has_calories,       "calories_per_100g", "Калории",     "nutrition", "critical",    ""),
            (r.has_protein,        "protein_per_100g",  "Белки",       "nutrition", "critical",    ""),
            (r.has_fat,            "fat_per_100g",      "Жиры",        "nutrition", "critical",    ""),
            (r.has_carbs,          "carbs_per_100g",    "Углеводы",    "nutrition", "critical",    ""),
            // ── nutrition (context-aware) ──
            (r.has_fiber,          "fiber_per_100g",    "Клетчатка",   "nutrition", "recommended", "Животный продукт — клетчатка = 0"),
            (r.has_density,        "density_g_per_ml",  "Плотность",   "nutrition", "recommended", ""),
            (r.has_shelf_life,     "shelf_life_days",   "Срок годности", "nutrition", "recommended", ""),
            (r.has_typical_portion,"typical_portion_g", "Типичная порция","nutrition","recommended", ""),
            // ── fish/seafood-specific (context-aware) ──
            (r.has_water_type,     "water_type",        "Тип воды",      "nutrition", "recommended", "Только для рыбы/морепродуктов"),
            (r.has_wild_farmed,    "wild_farmed",       "Дикий/фермерский","nutrition","recommended", "Только для рыбы/морепродуктов"),
            (r.has_sushi_grade,    "sushi_grade",       "Сашими-качество","nutrition", "optional",    "Только для рыбы/морепродуктов"),
            // ── seo (recommended) ──
            (r.has_seo_title,      "seo_title",       "SEO Title",    "seo",       "recommended", ""),
            // ── relations (optional) ──
            (r.has_macros,         "nutrition_macros",  "Макронутриенты (таблица)", "relations", "optional", ""),
            (r.has_vitamins,       "nutrition_vitamins","Витамины",     "relations", "optional", ""),
            (r.has_minerals,       "nutrition_minerals","Минералы",     "relations", "optional", ""),
            (r.has_allergens,      "product_allergens", "Аллергены",    "relations", "optional", ""),
            (r.has_diet_flags,     "diet_flags",        "Диет-флаги",   "relations", "optional", ""),
            (r.has_culinary,       "food_culinary",     "Кулинарные свойства", "relations", "optional", ""),
            (r.has_food_props,     "food_properties",   "Свойства продукта",   "relations", "optional", ""),
            (r.has_states,         "ingredient_states", "Состояния (≥10)", "relations", "optional", ""),
        ];

        let mut total: i64 = 0;
        let mut filled: i64 = 0;
        let mut weight_total: f64 = 0.0;
        let mut weight_filled: f64 = 0.0;
        let mut missing_fields: Vec<MissingField> = Vec::new();
        let mut not_applicable_fields: Vec<NotApplicableField> = Vec::new();

        for (has_value, field, label_ru, group, default_severity, na_reason) in &all_fields {
            let override_req = Self::field_requirement(field, pt);

            match override_req {
                Some(Requirement::NotApplicable) => {
                    // Completely excluded from score — not counted at all
                    let reason = if !na_reason.is_empty() {
                        na_reason
                    } else {
                        "Не применимо для данного типа продукта"
                    };
                    not_applicable_fields.push(NotApplicableField {
                        field,
                        label_ru,
                        reason,
                    });
                }
                Some(Requirement::Required) => {
                    // Override to critical
                    let w = Self::severity_weight("critical");
                    total += 1;
                    weight_total += w;
                    if *has_value {
                        filled += 1;
                        weight_filled += w;
                    } else {
                        missing_fields.push(MissingField {
                            field,
                            label_ru,
                            group,
                            severity: "critical",
                        });
                    }
                }
                _ => {
                    // None or Some(Recommended/Optional) → use the default_severity
                    let w = Self::severity_weight(default_severity);
                    total += 1;
                    weight_total += w;
                    if *has_value {
                        filled += 1;
                        weight_filled += w;
                    } else {
                        missing_fields.push(MissingField {
                            field,
                            label_ru,
                            group,
                            severity: default_severity,
                        });
                    }
                }
            }
        }

        // Flat score (backward-compatible)
        let score = if total > 0 {
            ((filled as f64 / total as f64) * 1000.0).round() / 10.0
        } else {
            0.0
        };

        // Weighted score — Required fields matter more
        let weighted_score = if weight_total > 0.0 {
            ((weight_filled / weight_total) * 1000.0).round() / 10.0
        } else {
            0.0
        };

        let has_critical_missing = missing_fields.iter().any(|f| f.severity == "critical");
        let status = if missing_fields.is_empty() {
            "complete"
        } else if has_critical_missing {
            "critical_missing"
        } else {
            "optional_missing"
        };

        // ── Build next_actions: top-5 prioritised items ──
        let next_actions = Self::build_next_actions(&missing_fields);

        DataQualityRow {
            id: r.id,
            name_en: r.name_en,
            product_type: r.product_type,
            score,
            filled,
            total,
            weighted_score,
            weight_filled: (weight_filled * 100.0).round() / 100.0,
            weight_total: (weight_total * 100.0).round() / 100.0,
            missing_fields,
            not_applicable_fields,
            next_actions,
            status,
        }
    }

    /// Weight multiplier per severity level
    fn severity_weight(severity: &str) -> f64 {
        match severity {
            "critical"    => 1.0,
            "recommended" => 0.6,
            "optional"    => 0.3,
            _             => 0.5,
        }
    }

    /// Build top-5 prioritised next actions from missing fields
    fn build_next_actions(missing: &[MissingField]) -> Vec<NextAction> {
        let mut actions: Vec<NextAction> = missing.iter().map(|m| {
            let (priority, reason, fix_hint) = match m.severity {
                "critical" => (
                    "high",
                    Self::field_reason(m.field, "critical"),
                    Self::field_fix_hint(m.field),
                ),
                "recommended" => (
                    "medium",
                    Self::field_reason(m.field, "recommended"),
                    Self::field_fix_hint(m.field),
                ),
                _ => (
                    "low",
                    Self::field_reason(m.field, "optional"),
                    Self::field_fix_hint(m.field),
                ),
            };
            NextAction {
                field: m.field,
                label_ru: m.label_ru,
                priority,
                reason,
                fix_hint,
            }
        }).collect();

        // Sort: high → medium → low
        actions.sort_by(|a, b| {
            let ord = |p: &str| -> u8 { match p { "high" => 0, "medium" => 1, _ => 2 } };
            ord(a.priority).cmp(&ord(b.priority))
        });

        actions.truncate(5);
        actions
    }

    /// Human-readable reason why a field matters
    fn field_reason(field: &str, severity: &str) -> &'static str {
        match field {
            "image_url"        => "Фото — первое впечатление, критично для каталога",
            "name_ru"          => "Название RU — необходимо для русскоязычных пользователей",
            "name_pl"          => "Название PL — необходимо для польскоязычных пользователей",
            "name_uk"          => "Название UK — необходимо для украиноязычных пользователей",
            "description_en"   => "Описание EN — базовый контент продукта",
            "description_ru"   => "Описание RU — контент для русскоязычной аудитории",
            "slug"             => "Slug — нужен для SEO и ссылок",
            "calories_per_100g"=> "Калории — главный показатель пищевой ценности",
            "protein_per_100g" => "Белки — один из трёх макронутриентов",
            "fat_per_100g"     => "Жиры — один из трёх макронутриентов",
            "carbs_per_100g"   => "Углеводы — один из трёх макронутриентов",
            "fiber_per_100g"   => "Клетчатка — важно для растительных продуктов",
            "density_g_per_ml" => "Плотность — нужна для пересчёта объёмов",
            "shelf_life_days"  => "Срок годности — планирование хранения",
            "typical_portion_g"=> "Типичная порция — удобство для пользователей",
            "water_type"       => "Тип воды — пресноводная/морская рыба",
            "wild_farmed"      => "Дикий/фермерский — качество рыбы/морепродуктов",
            "sushi_grade"      => "Сашими-качество — можно есть сырым",
            "seo_title"        => "SEO Title — поисковая оптимизация",
            _ => match severity {
                "critical" => "Критическое поле — без него продукт неполный",
                "recommended" => "Рекомендуемое — улучшает качество карточки",
                _ => "Опциональное — дополняет данные продукта",
            },
        }
    }

    /// Auto-fix suggestion for a missing field
    fn field_fix_hint(field: &str) -> &'static str {
        match field {
            "image_url"        => "Загрузите фото через админку или AI-генерацию",
            "name_ru" | "name_pl" | "name_uk"
                               => "Запустите AI-автоперевод (/autofill)",
            "description_en" | "description_ru"
                               => "Запустите AI-автозаполнение (/autofill)",
            "slug"             => "Генерируется автоматически при создании",
            "calories_per_100g" | "protein_per_100g" | "fat_per_100g" | "carbs_per_100g"
                               => "Запустите AI-автозаполнение (/autofill) — USDA данные",
            "fiber_per_100g"   => "Запустите AI-автозаполнение (/autofill)",
            "density_g_per_ml" => "Запустите AI-автозаполнение или введите вручную",
            "shelf_life_days"  => "Укажите типичный срок годности в днях",
            "typical_portion_g"=> "Запустите AI-автозаполнение (/autofill)",
            "water_type"       => "Укажите: freshwater / saltwater / brackish",
            "wild_farmed"      => "Укажите: wild / farmed / both",
            "sushi_grade"      => "Укажите: true / false",
            "seo_title"        => "Заполните SEO-поля для поисковой выдачи",
            "nutrition_macros" | "nutrition_vitamins" | "nutrition_minerals"
                               => "Запустите AI-автозаполнение — создаст связанные таблицы",
            "product_allergens" | "diet_flags"
                               => "Запустите AI-автозаполнение — заполнит аллергены и диет-флаги",
            "food_culinary" | "food_properties"
                               => "Запустите AI-автозаполнение — создаст кулинарные/физические свойства",
            "ingredient_states"=> "Запустите генерацию состояний (/generate-states)",
            _                  => "Заполните вручную или запустите AI-автозаполнение",
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
