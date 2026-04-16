//! Public Nutrition Service
//! Handles queries for:
//!   GET /public/nutrition/:slug  — full nutrition card for a product
//!   GET /public/diet/:flag       — all products matching a diet flag
//!   GET /public/ranking/:metric  — top-N products by a numeric metric

use crate::shared::{AppError, AppResult};
use serde::Serialize;
use sqlx::PgPool;

// ══════════════════════════════════════════════════════
// SERVICE
// ══════════════════════════════════════════════════════

#[derive(Clone)]
pub struct PublicNutritionService {
    pub pool: PgPool,
}

impl PublicNutritionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ══════════════════════════════════════════════════════
// DTO — Nutrition page
// ══════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProductBasicPublicRow {
    pub id: uuid::Uuid,
    pub slug: String,
    pub name_en: Option<String>,
    pub name_ru: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub product_type: Option<String>,
    pub unit: Option<String>,
    pub image_url: Option<String>,
    pub description_en: Option<String>,
    pub typical_portion_g: Option<f32>,
    pub wild_farmed: Option<String>,
    pub water_type: Option<String>,
    pub sushi_grade: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MacrosPublicRow {
    pub calories_kcal: Option<f32>,
    pub protein_g: Option<f32>,
    pub fat_g: Option<f32>,
    pub carbs_g: Option<f32>,
    pub fiber_g: Option<f32>,
    pub sugar_g: Option<f32>,
    pub water_g: Option<f32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VitaminsPublicRow {
    pub vitamin_a: Option<f32>,
    pub vitamin_c: Option<f32>,
    pub vitamin_d: Option<f32>,
    pub vitamin_e: Option<f32>,
    pub vitamin_k: Option<f32>,
    pub vitamin_b1: Option<f32>,
    pub vitamin_b2: Option<f32>,
    pub vitamin_b3: Option<f32>,
    pub vitamin_b6: Option<f32>,
    pub vitamin_b9: Option<f32>,
    pub vitamin_b12: Option<f32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MineralsPublicRow {
    pub calcium: Option<f32>,
    pub iron: Option<f32>,
    pub magnesium: Option<f32>,
    pub phosphorus: Option<f32>,
    pub potassium: Option<f32>,
    pub sodium: Option<f32>,
    pub zinc: Option<f32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DietFlagsPublicRow {
    pub vegan: Option<bool>,
    pub vegetarian: Option<bool>,
    pub keto: Option<bool>,
    pub paleo: Option<bool>,
    pub gluten_free: Option<bool>,
    pub mediterranean: Option<bool>,
    pub low_carb: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PairingPublicRow {
    pub slug: String,
    pub name_en: Option<String>,
    pub name_ru: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub image_url: Option<String>,
    pub pair_score: Option<f32>,
    pub flavor_score: Option<f32>,
    pub nutrition_score: Option<f32>,
    pub culinary_score: Option<f32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CulinaryPublicRow {
    pub sweetness: Option<f32>,
    pub acidity: Option<f32>,
    pub bitterness: Option<f32>,
    pub umami: Option<f32>,
    pub aroma: Option<f32>,
    pub texture: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FoodPropertiesPublicRow {
    pub glycemic_index: Option<f32>,
    pub glycemic_load: Option<f32>,
    pub ph: Option<f32>,
    pub smoke_point: Option<f32>,
    pub water_activity: Option<f32>,
}

// ── Health Profile (public) ───────────────────────────
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct HealthProfilePublicRow {
    pub bioactive_compounds_en: Option<serde_json::Value>,
    pub bioactive_compounds_ru: Option<serde_json::Value>,
    pub bioactive_compounds_pl: Option<serde_json::Value>,
    pub bioactive_compounds_uk: Option<serde_json::Value>,
    pub health_effects_en: Option<serde_json::Value>,
    pub health_effects_ru: Option<serde_json::Value>,
    pub health_effects_pl: Option<serde_json::Value>,
    pub health_effects_uk: Option<serde_json::Value>,
    pub contraindications_en: Option<serde_json::Value>,
    pub contraindications_ru: Option<serde_json::Value>,
    pub contraindications_pl: Option<serde_json::Value>,
    pub contraindications_uk: Option<serde_json::Value>,
    pub food_role: Option<String>,
    pub orac_score: Option<f32>,
    pub absorption_notes_en: Option<String>,
    pub absorption_notes_ru: Option<String>,
    pub absorption_notes_pl: Option<String>,
    pub absorption_notes_uk: Option<String>,
}

// ── Sugar Profile (public) ───────────────────────────
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SugarProfilePublicRow {
    pub glucose: Option<f32>,
    pub fructose: Option<f32>,
    pub sucrose: Option<f32>,
    pub lactose: Option<f32>,
    pub maltose: Option<f32>,
    pub total_sugars: Option<f32>,
    pub added_sugars: Option<f32>,
    pub sweetness_perception: Option<f32>,
    pub sugar_alcohols: Option<f32>,
}

// ── Processing Effects (public) ──────────────────────
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProcessingEffectsPublicRow {
    pub vitamin_retention_pct: Option<f32>,
    pub protein_denature_temp: Option<f32>,
    pub mineral_leaching_risk: Option<String>,
    pub best_cooking_method_en: Option<String>,
    pub best_cooking_method_ru: Option<String>,
    pub best_cooking_method_pl: Option<String>,
    pub best_cooking_method_uk: Option<String>,
    pub maillard_temp: Option<f32>,
    pub processing_notes_en: Option<String>,
    pub processing_notes_ru: Option<String>,
    pub processing_notes_pl: Option<String>,
    pub processing_notes_uk: Option<String>,
}

// ── Culinary Behavior (public) ───────────────────────
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CulinaryBehaviorPublicRow {
    pub behaviors_en: Option<serde_json::Value>,
    pub behaviors_ru: Option<serde_json::Value>,
    pub behaviors_pl: Option<serde_json::Value>,
    pub behaviors_uk: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct NutritionPageResponse {
    pub lang: String,
    pub slug: String,
    pub basic: ProductBasicPublicRow,
    pub macros: Option<MacrosPublicRow>,
    pub vitamins: Option<VitaminsPublicRow>,
    pub minerals: Option<MineralsPublicRow>,
    pub diet_flags: Option<DietFlagsPublicRow>,
    pub culinary: Option<CulinaryPublicRow>,
    pub food_properties: Option<FoodPropertiesPublicRow>,
    pub availability_months: Option<Vec<bool>>,
    pub pairings: Vec<PairingPublicRow>,
    pub health_profile: Option<HealthProfilePublicRow>,
    pub sugar_profile: Option<SugarProfilePublicRow>,
    pub processing_effects: Option<ProcessingEffectsPublicRow>,
    pub culinary_behavior: Option<CulinaryBehaviorPublicRow>,
}

// ══════════════════════════════════════════════════════
// DTO — Diet page
// ══════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DietProductRow {
    pub slug: String,
    pub name_en: Option<String>,
    pub name_ru: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub product_type: Option<String>,
    pub image_url: Option<String>,
    pub calories_kcal: Option<f32>,
    pub protein_g: Option<f32>,
    pub fat_g: Option<f32>,
    pub carbs_g: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct DietPageResponse {
    pub lang: String,
    pub flag: String,
    pub total: i64,
    pub products: Vec<DietProductRow>,
}

// ══════════════════════════════════════════════════════
// DTO — Ranking page
// ══════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RankingProductRow {
    pub rank: Option<i64>,
    pub slug: String,
    pub name_en: Option<String>,
    pub name_ru: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub product_type: Option<String>,
    pub image_url: Option<String>,
    pub metric_value: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct RankingPageResponse {
    pub lang: String,
    pub metric: String,
    pub metric_label_en: String,
    pub unit: String,
    pub order: String,
    pub total: i64,
    pub products: Vec<RankingProductRow>,
}

// ══════════════════════════════════════════════════════
// IMPL
// ══════════════════════════════════════════════════════

impl PublicNutritionService {
    // ── GET /public/nutrition/:slug ────────────────────────────────────────────
    pub async fn get_nutrition_page(&self, slug: &str) -> AppResult<NutritionPageResponse> {
        // 1. Basic product info (with image fallback from catalog_ingredients)
        let basic: Option<ProductBasicPublicRow> = sqlx::query_as(
            r#"SELECT p.id, p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
                      p.product_type, p.unit,
                      COALESCE(p.image_url, ci.image_url) AS image_url,
                      p.description_en,
                      p.typical_portion_g, p.wild_farmed, p.water_type, p.sushi_grade
               FROM products p
               LEFT JOIN catalog_ingredients ci ON ci.id = p.id
               WHERE p.slug = $1"#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        let basic = basic.ok_or_else(|| AppError::NotFound(format!("product '{slug}' not found")))?;
        let product_id = basic.id;

        // 2. Macros
        let macros: Option<MacrosPublicRow> = sqlx::query_as(
            r#"SELECT calories_kcal, protein_g, fat_g, carbs_g, fiber_g, sugar_g, water_g
               FROM nutrition_macros WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?;

        // 3. Vitamins
        let vitamins: Option<VitaminsPublicRow> = sqlx::query_as(
            r#"SELECT vitamin_a, vitamin_c, vitamin_d, vitamin_e, vitamin_k,
                      vitamin_b1, vitamin_b2, vitamin_b3, vitamin_b6, vitamin_b9, vitamin_b12
               FROM nutrition_vitamins WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?;

        // 4. Minerals
        let minerals: Option<MineralsPublicRow> = sqlx::query_as(
            r#"SELECT calcium, iron, magnesium, phosphorus, potassium, sodium, zinc
               FROM nutrition_minerals WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?;

        // 5. Diet flags
        let diet_flags: Option<DietFlagsPublicRow> = sqlx::query_as(
            r#"SELECT vegan, vegetarian, keto, paleo, gluten_free, mediterranean, low_carb
               FROM diet_flags WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?;

        // 6. Culinary profile
        let culinary: Option<CulinaryPublicRow> = sqlx::query_as(
            r#"SELECT sweetness, acidity, bitterness, umami, aroma, texture
               FROM food_culinary_properties WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        // 7. Food properties
        let food_properties: Option<FoodPropertiesPublicRow> = sqlx::query_as(
            r#"SELECT glycemic_index, glycemic_load, ph, smoke_point, water_activity
               FROM food_properties WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        // 8. Availability months
        let availability_months: Option<Vec<bool>> = sqlx::query_scalar(
            r#"SELECT availability_months FROM products WHERE id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None)
        .flatten();

        // 9. Top-10 pairings (best pair_score)
        let pairings: Vec<PairingPublicRow> = sqlx::query_as(
            r#"SELECT b.slug, b.name_en, b.name_ru, b.name_pl, b.name_uk,
                      COALESCE(b.image_url, ci.image_url) AS image_url,
                      fp.pair_score, fp.flavor_score, fp.nutrition_score, fp.culinary_score
               FROM food_pairing fp
               JOIN products b ON b.id = fp.ingredient_b
               LEFT JOIN catalog_ingredients ci ON ci.id = b.id
               WHERE fp.ingredient_a = $1
               ORDER BY fp.pair_score DESC NULLS LAST
               LIMIT 10"#,
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // 10. Health profile
        let health_profile: Option<HealthProfilePublicRow> = sqlx::query_as(
            r#"SELECT bioactive_compounds_en, bioactive_compounds_ru,
                      bioactive_compounds_pl, bioactive_compounds_uk,
                      health_effects_en, health_effects_ru,
                      health_effects_pl, health_effects_uk,
                      contraindications_en, contraindications_ru,
                      contraindications_pl, contraindications_uk,
                      food_role, orac_score,
                      absorption_notes_en, absorption_notes_ru,
                      absorption_notes_pl, absorption_notes_uk
               FROM product_health_profile WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        // 11. Sugar profile
        let sugar_profile: Option<SugarProfilePublicRow> = sqlx::query_as(
            r#"SELECT glucose, fructose, sucrose, lactose, maltose,
                      total_sugars, added_sugars, sweetness_perception, sugar_alcohols
               FROM nutrition_sugar_profile WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        // 12. Processing effects
        let processing_effects: Option<ProcessingEffectsPublicRow> = sqlx::query_as(
            r#"SELECT vitamin_retention_pct, protein_denature_temp, mineral_leaching_risk,
                      best_cooking_method_en, best_cooking_method_ru,
                      best_cooking_method_pl, best_cooking_method_uk,
                      maillard_temp,
                      processing_notes_en, processing_notes_ru,
                      processing_notes_pl, processing_notes_uk
               FROM product_processing_effects WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        // 13. Culinary behavior
        let culinary_behavior: Option<CulinaryBehaviorPublicRow> = sqlx::query_as(
            r#"SELECT behaviors_en, behaviors_ru, behaviors_pl, behaviors_uk
               FROM product_culinary_behavior WHERE product_id = $1"#,
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        Ok(NutritionPageResponse {
            lang: "en".to_string(),
            slug: slug.to_string(),
            basic,
            macros,
            vitamins,
            minerals,
            diet_flags,
            culinary,
            food_properties,
            availability_months,
            pairings,
            health_profile,
            sugar_profile,
            processing_effects,
            culinary_behavior,
        })
    }

    // ── GET /public/diet/:flag ─────────────────────────────────────────────────
    pub async fn get_diet_page(
        &self,
        flag: &str,
        product_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> AppResult<DietPageResponse> {
        // Validate flag name — only known columns allowed (prevent SQL injection)
        let col = match flag {
            "vegan" => "vegan",
            "vegetarian" => "vegetarian",
            "keto" => "keto",
            "paleo" => "paleo",
            "gluten-free" | "gluten_free" => "gluten_free",
            "mediterranean" => "mediterranean",
            "low-carb" | "low_carb" => "low_carb",
            _ => return Err(AppError::Validation(format!("unknown diet flag '{flag}'"))),
        };

        let type_filter = product_type.unwrap_or("");
        let use_type = !type_filter.is_empty();

        // Count
        let total: i64 = if use_type {
            sqlx::query_scalar(&format!(
                r#"SELECT COUNT(*) FROM products p
                   JOIN diet_flags df ON df.product_id = p.id
                   WHERE df.{col} = true AND p.product_type = $1"#
            ))
            .bind(type_filter)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
        } else {
            sqlx::query_scalar(&format!(
                r#"SELECT COUNT(*) FROM products p
                   JOIN diet_flags df ON df.product_id = p.id
                   WHERE df.{col} = true"#
            ))
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
        };

        // Products with macros — fallback to catalog_ingredients for image_url
        let products: Vec<DietProductRow> = if use_type {
            sqlx::query_as(&format!(
                r#"SELECT p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
                          p.product_type,
                          COALESCE(p.image_url, ci.image_url) AS image_url,
                          m.calories_kcal, m.protein_g, m.fat_g, m.carbs_g
                   FROM products p
                   JOIN diet_flags df ON df.product_id = p.id
                   LEFT JOIN nutrition_macros m ON m.product_id = p.id
                   LEFT JOIN catalog_ingredients ci ON ci.id = p.id
                   WHERE df.{col} = true AND p.product_type = $1
                   ORDER BY m.calories_kcal ASC NULLS LAST
                   LIMIT $2 OFFSET $3"#
            ))
            .bind(type_filter)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
        } else {
            sqlx::query_as(&format!(
                r#"SELECT p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
                          p.product_type,
                          COALESCE(p.image_url, ci.image_url) AS image_url,
                          m.calories_kcal, m.protein_g, m.fat_g, m.carbs_g
                   FROM products p
                   JOIN diet_flags df ON df.product_id = p.id
                   LEFT JOIN nutrition_macros m ON m.product_id = p.id
                   LEFT JOIN catalog_ingredients ci ON ci.id = p.id
                   WHERE df.{col} = true
                   ORDER BY m.calories_kcal ASC NULLS LAST
                   LIMIT $1 OFFSET $2"#
            ))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
        };

        Ok(DietPageResponse {
            lang: "en".to_string(),
            flag: flag.to_string(),
            total,
            products,
        })
    }

    // ── GET /public/ranking/:metric ────────────────────────────────────────────
    pub async fn get_ranking_page(
        &self,
        metric: &str,
        product_type: Option<&str>,
        order: &str, // "desc" | "asc"
        limit: i64,
    ) -> AppResult<RankingPageResponse> {
        // Map metric → (table, column, label, unit, default_order)
        let (table, col, label, unit, default_order) = match metric {
            "calories"   => ("nutrition_macros",   "calories_kcal", "Calories",   "kcal", "desc"),
            "protein"    => ("nutrition_macros",   "protein_g",     "Protein",    "g",    "desc"),
            "fat"        => ("nutrition_macros",   "fat_g",         "Fat",        "g",    "desc"),
            "carbs"      => ("nutrition_macros",   "carbs_g",       "Carbohydrates", "g", "desc"),
            "fiber"      => ("nutrition_macros",   "fiber_g",       "Fiber",      "g",    "desc"),
            "sugar"      => ("nutrition_macros",   "sugar_g",       "Sugar",      "g",    "asc"),
            "vitamin-c"  => ("nutrition_vitamins", "vitamin_c",     "Vitamin C",  "mg",   "desc"),
            "vitamin-d"  => ("nutrition_vitamins", "vitamin_d",     "Vitamin D",  "µg",   "desc"),
            "vitamin-b12"=> ("nutrition_vitamins", "vitamin_b12",   "Vitamin B12","µg",   "desc"),
            "iron"       => ("nutrition_minerals", "iron",          "Iron",       "mg",   "desc"),
            "calcium"    => ("nutrition_minerals", "calcium",       "Calcium",    "mg",   "desc"),
            "potassium"  => ("nutrition_minerals", "potassium",     "Potassium",  "mg",   "desc"),
            "magnesium"  => ("nutrition_minerals", "magnesium",     "Magnesium",  "mg",   "desc"),
            "zinc"       => ("nutrition_minerals", "zinc",          "Zinc",       "mg",   "desc"),
            "sodium"     => ("nutrition_minerals", "sodium",        "Sodium",     "mg",   "asc"),
            _ => return Err(AppError::Validation(format!("unknown metric '{metric}'"))),
        };

        let effective_order = if order == "asc" { "ASC" } else { "DESC" };
        let type_filter = product_type.unwrap_or("");
        let use_type = !type_filter.is_empty();

        let count: i64 = if use_type {
            sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM products p JOIN {table} t ON t.product_id = p.id WHERE t.{col} IS NOT NULL AND p.product_type = $1"
            ))
            .bind(type_filter)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
        } else {
            sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM products p JOIN {table} t ON t.product_id = p.id WHERE t.{col} IS NOT NULL"
            ))
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0)
        };

        let products: Vec<RankingProductRow> = if use_type {
            sqlx::query_as(&format!(
                r#"SELECT ROW_NUMBER() OVER (ORDER BY t.{col} {effective_order} NULLS LAST) AS rank,
                          p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
                          p.product_type,
                          COALESCE(p.image_url, ci.image_url) AS image_url,
                          t.{col} AS metric_value
                   FROM products p
                   JOIN {table} t ON t.product_id = p.id
                   LEFT JOIN catalog_ingredients ci ON ci.id = p.id
                   WHERE t.{col} IS NOT NULL AND p.product_type = $1
                   ORDER BY t.{col} {effective_order} NULLS LAST
                   LIMIT $2"#
            ))
            .bind(type_filter)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
        } else {
            sqlx::query_as(&format!(
                r#"SELECT ROW_NUMBER() OVER (ORDER BY t.{col} {effective_order} NULLS LAST) AS rank,
                          p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
                          p.product_type,
                          COALESCE(p.image_url, ci.image_url) AS image_url,
                          t.{col} AS metric_value
                   FROM products p
                   JOIN {table} t ON t.product_id = p.id
                   LEFT JOIN catalog_ingredients ci ON ci.id = p.id
                   WHERE t.{col} IS NOT NULL
                   ORDER BY t.{col} {effective_order} NULLS LAST
                   LIMIT $1"#
            ))
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
        };

        Ok(RankingPageResponse {
            lang: "en".to_string(),
            metric: metric.to_string(),
            metric_label_en: label.to_string(),
            unit: unit.to_string(),
            order: effective_order.to_lowercase(),
            total: count,
            products,
        })
    }

    // ── GET /public/products-slugs ─────────────────────────────────────────────
    pub async fn get_all_slugs(&self) -> AppResult<Vec<String>> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT slug FROM products ORDER BY slug")
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::from)?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    }
}
