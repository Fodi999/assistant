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

#[derive(Debug, Serialize)]
pub struct NutritionPageResponse {
    pub lang: String,
    pub slug: String,
    pub basic: ProductBasicPublicRow,
    pub macros: Option<MacrosPublicRow>,
    pub vitamins: Option<VitaminsPublicRow>,
    pub minerals: Option<MineralsPublicRow>,
    pub diet_flags: Option<DietFlagsPublicRow>,
    pub pairings: Vec<PairingPublicRow>,
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
        // 1. Basic product info
        let basic: Option<ProductBasicPublicRow> = sqlx::query_as(
            r#"SELECT id, slug, name_en, name_ru, name_pl, name_uk,
                      product_type, unit, image_url, description_en,
                      typical_portion_g, wild_farmed, water_type, sushi_grade
               FROM products WHERE slug = $1"#,
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

        // 6. Top-10 pairings (best pair_score)
        let pairings: Vec<PairingPublicRow> = sqlx::query_as(
            r#"SELECT b.slug, b.name_en, b.name_ru, b.name_pl, b.name_uk, b.image_url,
                      fp.pair_score, fp.flavor_score, fp.nutrition_score, fp.culinary_score
               FROM food_pairing fp
               JOIN products b ON b.id = fp.ingredient_b
               WHERE fp.ingredient_a = $1
               ORDER BY fp.pair_score DESC NULLS LAST
               LIMIT 10"#,
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        Ok(NutritionPageResponse {
            lang: "en".to_string(),
            slug: slug.to_string(),
            basic,
            macros,
            vitamins,
            minerals,
            diet_flags,
            pairings,
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

        // Products with macros
        let products: Vec<DietProductRow> = if use_type {
            sqlx::query_as(&format!(
                r#"SELECT p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
                          p.product_type, p.image_url,
                          m.calories_kcal, m.protein_g, m.fat_g, m.carbs_g
                   FROM products p
                   JOIN diet_flags df ON df.product_id = p.id
                   LEFT JOIN nutrition_macros m ON m.product_id = p.id
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
                          p.product_type, p.image_url,
                          m.calories_kcal, m.protein_g, m.fat_g, m.carbs_g
                   FROM products p
                   JOIN diet_flags df ON df.product_id = p.id
                   LEFT JOIN nutrition_macros m ON m.product_id = p.id
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
                          p.product_type, p.image_url,
                          t.{col} AS metric_value
                   FROM products p
                   JOIN {table} t ON t.product_id = p.id
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
                          p.product_type, p.image_url,
                          t.{col} AS metric_value
                   FROM products p
                   JOIN {table} t ON t.product_id = p.id
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
