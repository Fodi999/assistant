//! Category / measure handlers: list_categories, measure_conversion, ingredient_measures.

use super::shared::parse_lang;
use crate::domain::tools::catalog_row::{CatalogNutritionRow, CATALOG_NUTRITION_COLS};
use crate::domain::tools::unit_converter as uc;
use crate::shared::Language;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ── 1. List categories (tool index) ──────────────────────────────────────────

#[derive(Serialize)]
pub struct ToolInfo {
    pub id:          &'static str,
    pub path:        &'static str,
    pub description: &'static str,
}

#[derive(Serialize)]
pub struct CategoriesResponse {
    pub tools: Vec<ToolInfo>,
}

/// GET /public/tools/categories
pub async fn list_categories() -> Json<CategoriesResponse> {
    Json(CategoriesResponse {
        tools: vec![
            ToolInfo { id: "converter",             path: "/public/tools/convert",               description: "Universal unit converter (mass & volume)" },
            ToolInfo { id: "units",                  path: "/public/tools/units",                 description: "List all supported units with labels" },
            ToolInfo { id: "nutrition",              path: "/public/tools/nutrition",              description: "Nutrition calculator (supports any unit)" },
            ToolInfo { id: "fish-season",            path: "/public/tools/fish-season",            description: "Fish seasonality calendar (single fish)" },
            ToolInfo { id: "fish-season-table",      path: "/public/tools/fish-season-table",      description: "Full fish seasonality table with catalog data (name, image)" },
            ToolInfo { id: "scale",                  path: "/public/tools/scale",                  description: "Recipe portion scaler" },
            ToolInfo { id: "yield",                  path: "/public/tools/yield",                  description: "Cooking yield & waste calculator" },
            ToolInfo { id: "ingredient-equivalents", path: "/public/tools/ingredient-equivalents", description: "Convert ingredient to all units via density" },
            ToolInfo { id: "food-cost",              path: "/public/tools/food-cost",              description: "Food cost, margin & markup calculator" },
            ToolInfo { id: "ingredient-suggestions", path: "/public/tools/ingredient-suggestions", description: "Suggest ingredients by volume unit with grams" },
            ToolInfo { id: "popular-conversions",    path: "/public/tools/popular-conversions",    description: "Curated popular cooking conversions (SEO)" },
            ToolInfo { id: "ingredient-scale",       path: "/public/tools/ingredient-scale",       description: "Scale an ingredient between portion sizes" },
            ToolInfo { id: "measure-conversion",     path: "/public/tools/measure-conversion",     description: "SEO: how many grams in a cup/tbsp/tsp of an ingredient" },
            ToolInfo { id: "ingredient-measures",    path: "/public/tools/ingredient-measures",    description: "SEO: full cup/tbsp/tsp grams table for an ingredient" },
        ],
    })
}

// ── Helpers shared by measure handlers ───────────────────────────────────────

fn ml_for_unit(unit: &str) -> Option<f64> {
    match unit.to_lowercase().as_str() {
        "cup" | "cups"                              => Some(uc::CUP_ML),
        "tbsp" | "tablespoon" | "tablespoons"       => Some(uc::TBSP_ML),
        "tsp"  | "teaspoon"   | "teaspoons"         => Some(uc::TSP_ML),
        _ => None,
    }
}

fn unit_display_label(unit: &str, lang: Language) -> String {
    match (unit.to_lowercase().as_str(), lang) {
        ("cup" | "cups", Language::Pl) => "szklanka".to_string(),
        ("cup" | "cups", Language::Ru) => "стакан".to_string(),
        ("cup" | "cups", Language::Uk) => "склянка".to_string(),
        ("cup" | "cups", _)            => "cup".to_string(),
        ("tbsp" | "tablespoon" | "tablespoons", Language::Pl) => "łyżka stołowa".to_string(),
        ("tbsp" | "tablespoon" | "tablespoons", Language::Ru) => "столовая ложка".to_string(),
        ("tbsp" | "tablespoon" | "tablespoons", Language::Uk) => "столова ложка".to_string(),
        ("tbsp" | "tablespoon" | "tablespoons", _)            => "tbsp".to_string(),
        ("tsp"  | "teaspoon"   | "teaspoons", Language::Pl) => "łyżeczka".to_string(),
        ("tsp"  | "teaspoon"   | "teaspoons", Language::Ru) => "чайная ложка".to_string(),
        ("tsp"  | "teaspoon"   | "teaspoons", Language::Uk) => "чайна ложка".to_string(),
        ("tsp"  | "teaspoon"   | "teaspoons", _)            => "tsp".to_string(),
        ("g" | "grams" | "gram", Language::Pl) => "gram".to_string(),
        ("g" | "grams" | "gram", Language::Ru) => "грамм".to_string(),
        ("g" | "grams" | "gram", Language::Uk) => "грам".to_string(),
        ("g" | "grams" | "gram", _)            => "g".to_string(),
        _ => unit.to_string(),
    }
}

fn measure_question(unit: &str, name: &str, lang: Language) -> String {
    match lang {
        Language::Pl => format!("Ile gramów ma {} {}?", unit, name),
        Language::Ru => format!("Сколько граммов в {} {}?", unit, name),
        Language::Uk => format!("Скільки грамів у {} {}?", unit, name),
        Language::En => format!("How many grams in a {} of {}?", unit, name),
    }
}

fn measure_answer(value: f64, unit: &str, name: &str, result: f64, lang: Language) -> String {
    match lang {
        Language::Pl => format!("{} {} {} to {} gramów.", value, unit, name, result),
        Language::Ru => format!("{} {} {} = {} граммов.", value, unit, name, result),
        Language::Uk => format!("{} {} {} = {} грамів.", value, unit, name, result),
        Language::En => format!("{} {} of {} equals {} grams.", value, unit, name, result),
    }
}

fn db_row_query() -> String {
    format!(
        r#"SELECT {}
           FROM catalog_ingredients
           WHERE is_active = true
             AND (LOWER(name_en) = $1
                  OR slug = $1
                  OR LOWER(name_ru) = $1
                  OR LOWER(name_pl) = $1
                  OR LOWER(name_uk) = $1
                  OR LOWER(name_en) LIKE '%' || $1 || '%')
           ORDER BY (LOWER(name_en) = $1 OR slug = $1) DESC, LENGTH(name_en) ASC
           LIMIT 1"#,
        CATALOG_NUTRITION_COLS
    )
}

// ── 2. Measure conversion ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MeasureConversionQuery {
    pub ingredient: String,
    pub from:       String,
    pub to:         String,
    pub lang:       Option<String>,
    pub value:      Option<f64>,
}

#[derive(Serialize)]
pub struct MeasureConversionResponse {
    pub ingredient:      String,
    pub ingredient_name: String,
    pub slug:            Option<String>,
    pub image_url:       Option<String>,
    pub value:           f64,
    pub from:            String,
    pub from_label:      String,
    pub to:              String,
    pub to_label:        String,
    pub result:          f64,
    pub question:        String,
    pub answer:          String,
}

/// GET /public/tools/measure-conversion?ingredient=flour&from=cup&to=g&lang=en&value=1
pub async fn measure_conversion(
    State(pool): State<PgPool>,
    Query(params): Query<MeasureConversionQuery>,
) -> Json<MeasureConversionResponse> {
    let lang       = parse_lang(&params.lang);
    let value      = params.value.unwrap_or(1.0);
    let name_lower = params.ingredient.to_lowercase();

    let db_row: Option<CatalogNutritionRow> = sqlx::query_as(&db_row_query())
        .bind(&name_lower)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

    let (density, ingredient_name, slug, image_url) = if let Some(ref row) = db_row {
        (row.density(), row.localized_name(lang).to_string(), row.slug.clone(), row.image_url.clone())
    } else {
        (1.0, params.ingredient.clone(), None, None)
    };

    let result = if let Some(ml) = ml_for_unit(&params.from) {
        uc::round_to(value * uc::grams_from_volume(density, ml), 2)
    } else if params.from.to_lowercase() == "g" {
        uc::round_to(value, 2)
    } else {
        0.0
    };

    let from_label = unit_display_label(&params.from, lang);
    let to_label   = unit_display_label(&params.to,   lang);
    let question   = measure_question(&from_label, &ingredient_name, lang);
    let answer     = measure_answer(value, &from_label, &ingredient_name, result, lang);

    Json(MeasureConversionResponse {
        ingredient: params.ingredient,
        ingredient_name,
        slug,
        image_url,
        value,
        from: params.from,
        from_label,
        to: params.to,
        to_label,
        result,
        question,
        answer,
    })
}

// ── 3. Ingredient measures table ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngredientMeasuresQuery {
    pub ingredient: String,
    pub lang:       Option<String>,
}

#[derive(Serialize)]
pub struct MeasureEntry {
    pub unit:       String,
    pub unit_label: String,
    pub grams:      f64,
}

#[derive(Serialize)]
pub struct IngredientMeasuresResponse {
    pub ingredient:       String,
    pub ingredient_name:  String,
    pub slug:             Option<String>,
    pub image_url:        Option<String>,
    pub density_g_per_ml: Option<f64>,
    pub measures:         Vec<MeasureEntry>,
}

/// GET /public/tools/ingredient-measures?ingredient=flour&lang=en
pub async fn ingredient_measures(
    State(pool): State<PgPool>,
    Query(params): Query<IngredientMeasuresQuery>,
) -> Json<IngredientMeasuresResponse> {
    let lang       = parse_lang(&params.lang);
    let name_lower = params.ingredient.to_lowercase();

    let db_row: Option<CatalogNutritionRow> = sqlx::query_as(&db_row_query())
        .bind(&name_lower)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

    let (density, ingredient_name, slug, image_url, density_opt) = if let Some(ref row) = db_row {
        let d = row.density();
        (d, row.localized_name(lang).to_string(), row.slug.clone(), row.image_url.clone(), Some(d))
    } else {
        (1.0, params.ingredient.clone(), None, None, None)
    };

    let units = [("cup", uc::CUP_ML), ("tbsp", uc::TBSP_ML), ("tsp", uc::TSP_ML)];

    let measures = units.iter().map(|(unit, ml)| MeasureEntry {
        unit:       unit.to_string(),
        unit_label: unit_display_label(unit, lang),
        grams:      uc::round_to(uc::grams_from_volume(density, *ml), 2),
    }).collect();

    Json(IngredientMeasuresResponse {
        ingredient: params.ingredient,
        ingredient_name,
        slug,
        image_url,
        density_g_per_ml: density_opt,
        measures,
    })
}
