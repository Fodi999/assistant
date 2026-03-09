//! Kitchen calculator handlers:
//! scale_recipe, yield_calc, ingredient_equivalents, food_cost_calc,
//! ingredient_suggestions, popular_conversions.

use super::shared::{label, parse_lang};
use crate::domain::tools::catalog_row::CatalogNutritionRow;
use crate::domain::tools::unit_converter as uc;
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ── 1. Recipe scaler ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ScaleQuery {
    pub value:         f64,
    pub from_portions: f64,
    pub to_portions:   f64,
}

#[derive(Serialize)]
pub struct ScaleResponse {
    pub original:      f64,
    pub from_portions: f64,
    pub to_portions:   f64,
    pub result:        f64,
}

/// GET /public/tools/scale?value=500&from_portions=4&to_portions=10
pub async fn scale_recipe(Query(params): Query<ScaleQuery>) -> Json<ScaleResponse> {
    let result = uc::round_to(uc::scale(params.value, params.from_portions, params.to_portions), 2);
    Json(ScaleResponse {
        original:      params.value,
        from_portions: params.from_portions,
        to_portions:   params.to_portions,
        result,
    })
}

// ── 2. Yield calculator ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct YieldQuery {
    pub raw:    f64,
    pub usable: f64,
}

#[derive(Serialize)]
pub struct YieldResponse {
    pub raw:           f64,
    pub usable:        f64,
    pub yield_percent: f64,
    pub waste_percent: f64,
}

/// GET /public/tools/yield?raw=1000&usable=750
pub async fn yield_calc(Query(params): Query<YieldQuery>) -> Json<YieldResponse> {
    let yp = uc::round_to(uc::yield_percent(params.raw, params.usable), 2);
    Json(YieldResponse {
        raw:           params.raw,
        usable:        params.usable,
        yield_percent: yp,
        waste_percent: uc::round_to(100.0 - yp, 2),
    })
}

// ── 3. Ingredient equivalents ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct EquivalentsQuery {
    pub name:  String,
    pub value: f64,
    pub unit:  String,
    pub lang:  Option<String>,
}

#[derive(Serialize)]
pub struct Equivalent {
    pub unit:  String,
    pub label: String,
    pub value: f64,
}

#[derive(Serialize)]
pub struct EquivalentsResponse {
    pub name:        String,
    pub input_value: f64,
    pub input_unit:  String,
    pub equivalents: Vec<Equivalent>,
}

/// GET /public/tools/ingredient-equivalents?name=flour&value=100&unit=g&lang=ru
pub async fn ingredient_equivalents(
    State(pool): State<PgPool>,
    Query(params): Query<EquivalentsQuery>,
) -> Json<EquivalentsResponse> {
    let lang       = parse_lang(&params.lang);
    let name_lower = params.name.to_lowercase();

    let density = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        r#"SELECT density_g_per_ml FROM catalog_ingredients
           WHERE is_active = true AND density_g_per_ml IS NOT NULL
             AND (LOWER(name_en) = $1 OR slug = $1 OR LOWER(name_en) LIKE '%' || $1 || '%')
           ORDER BY (LOWER(name_en) = $1 OR slug = $1) DESC
           LIMIT 1"#,
    )
    .bind(&name_lower)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten()
    .and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d))
    .unwrap_or(1.0);

    let target_units: &[&str] = &["g", "kg", "oz", "lb", "ml", "l", "fl_oz", "tsp", "tbsp", "cup"];

    let equivalents: Vec<Equivalent> = target_units
        .iter()
        .filter(|&&u| u != params.unit)
        .filter_map(|&u| {
            uc::convert_with_density(params.value, &params.unit, u, density)
                .map(|v| Equivalent { unit: u.to_string(), label: label(u, lang), value: uc::display_round(v) })
        })
        .collect();

    Json(EquivalentsResponse {
        name:        params.name,
        input_value: params.value,
        input_unit:  params.unit,
        equivalents,
    })
}

// ── 4. Food cost calculator ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FoodCostQuery {
    pub price:        f64,
    pub price_amount: Option<f64>,
    pub price_unit:   Option<String>,
    pub amount:       f64,
    pub unit:         Option<String>,
    pub portions:     Option<f64>,
    pub sell_price:   Option<f64>,
    pub lang:         Option<String>,
}

#[derive(Serialize)]
pub struct FoodCostResponse {
    pub price:            f64,
    pub price_unit:       String,
    pub amount:           f64,
    pub unit:             String,
    pub total_cost:       f64,
    pub cost_per_portion: Option<f64>,
    pub sell_price:       Option<f64>,
    pub margin_percent:   Option<f64>,
    pub markup_percent:   Option<f64>,
}

/// GET /public/tools/food-cost?price=5.50&price_unit=kg&amount=500&unit=g&portions=4&sell_price=15.0
pub async fn food_cost_calc(Query(params): Query<FoodCostQuery>) -> Json<FoodCostResponse> {
    let price_unit   = params.price_unit.as_deref().unwrap_or("kg");
    let unit         = params.unit.as_deref().unwrap_or(price_unit);
    let price_amount = params.price_amount.unwrap_or(1.0);

    let amount_in_price_unit = if unit == price_unit {
        params.amount
    } else {
        uc::convert_units(params.amount, unit, price_unit).unwrap_or(params.amount)
    };

    let price_per_one = if price_amount > 0.0 { params.price / price_amount } else { params.price };
    let total_cost    = uc::round_to(price_per_one * amount_in_price_unit, 2);

    let cost_per_portion = params.portions.map(|p| uc::round_to(uc::cost_per_portion(total_cost, p), 2));

    let margin_percent = match (params.sell_price, cost_per_portion) {
        (Some(sp), Some(cpp)) if sp > 0.0 => Some(uc::round_to(uc::margin_percent(sp, cpp), 1)),
        _ => None,
    };

    let markup_percent = match (params.sell_price, cost_per_portion) {
        (Some(sp), Some(cpp)) if cpp > 0.0 => Some(uc::round_to(((sp - cpp) / cpp) * 100.0, 1)),
        _ => None,
    };

    Json(FoodCostResponse {
        price: params.price,
        price_unit: price_unit.to_string(),
        amount: params.amount,
        unit: unit.to_string(),
        total_cost,
        cost_per_portion,
        sell_price: params.sell_price,
        margin_percent,
        markup_percent,
    })
}

// ── 5. Ingredient suggestions ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SuggestionsQuery {
    pub unit: String,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct Suggestion {
    pub name:             String,
    pub name_en:          String,
    pub slug:             Option<String>,
    pub image_url:        Option<String>,
    pub density_g_per_ml: f64,
    pub equivalent_g:     f64,
}

#[derive(Serialize)]
pub struct SuggestionsResponse {
    pub unit:         String,
    pub ml_per_unit:  Option<f64>,
    pub suggestions:  Vec<Suggestion>,
}

/// GET /public/tools/ingredient-suggestions?unit=cup&lang=ru
pub async fn ingredient_suggestions(
    State(pool): State<PgPool>,
    Query(params): Query<SuggestionsQuery>,
) -> Json<SuggestionsResponse> {
    let lang      = parse_lang(&params.lang);
    let ml_factor = uc::volume_to_ml(&params.unit);

    let suggestions: Vec<Suggestion> = if let Some(ml) = ml_factor {
        let rows: Vec<CatalogNutritionRow> = sqlx::query_as(
            &format!(
                r#"SELECT {}
                   FROM catalog_ingredients
                   WHERE is_active = true AND density_g_per_ml IS NOT NULL
                     AND density_g_per_ml != 1.0
                   ORDER BY name_en ASC"#,
                crate::domain::tools::catalog_row::CATALOG_NUTRITION_COLS
            ),
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        rows.iter().map(|r| {
            let density = r.density();
            let grams   = uc::display_round(ml * density);
            Suggestion {
                name:             r.localized_name(lang).to_string(),
                name_en:          r.name_en.clone(),
                slug:             r.slug.clone(),
                image_url:        r.image_url.clone(),
                density_g_per_ml: density,
                equivalent_g:     grams,
            }
        }).collect()
    } else {
        vec![]
    };

    Json(SuggestionsResponse { unit: params.unit, ml_per_unit: ml_factor, suggestions })
}

// ── 6. Popular conversions ────────────────────────────────────────────────────

struct PopularEntry {
    value:      f64,
    from_unit:  &'static str,
    to_unit:    &'static str,
    ingredient: Option<&'static str>,
    density:    Option<f64>,
}

static POPULAR_CONVERSIONS: &[PopularEntry] = &[
    PopularEntry { value: 1.0, from_unit: "cup",          to_unit: "g",  ingredient: Some("flour"),  density: Some(0.53) },
    PopularEntry { value: 1.0, from_unit: "tbsp",         to_unit: "g",  ingredient: Some("flour"),  density: Some(0.53) },
    PopularEntry { value: 1.0, from_unit: "cup",          to_unit: "g",  ingredient: Some("sugar"),  density: Some(0.85) },
    PopularEntry { value: 1.0, from_unit: "tbsp",         to_unit: "g",  ingredient: Some("sugar"),  density: Some(0.85) },
    PopularEntry { value: 1.0, from_unit: "tbsp",         to_unit: "g",  ingredient: Some("butter"), density: Some(0.92) },
    PopularEntry { value: 1.0, from_unit: "cup",          to_unit: "g",  ingredient: Some("butter"), density: Some(0.92) },
    PopularEntry { value: 1.0, from_unit: "stick_butter", to_unit: "g",  ingredient: Some("butter"), density: Some(0.92) },
    PopularEntry { value: 1.0, from_unit: "tbsp",         to_unit: "g",  ingredient: Some("honey"),  density: Some(1.42) },
    PopularEntry { value: 1.0, from_unit: "cup",          to_unit: "g",  ingredient: Some("honey"),  density: Some(1.42) },
    PopularEntry { value: 1.0, from_unit: "cup",          to_unit: "g",  ingredient: Some("rice"),   density: Some(0.77) },
    PopularEntry { value: 1.0, from_unit: "cup",          to_unit: "g",  ingredient: Some("milk"),   density: Some(1.03) },
    PopularEntry { value: 1.0, from_unit: "cup",          to_unit: "ml", ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "lb",           to_unit: "g",  ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "oz",           to_unit: "g",  ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "gallon",       to_unit: "l",  ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "pint",         to_unit: "ml", ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "quart",        to_unit: "ml", ingredient: None, density: None },
];

#[derive(Deserialize)]
pub struct PopularQuery {
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct PopularConversion {
    pub value:          f64,
    pub from_unit:      String,
    pub from_label:     String,
    pub to_unit:        String,
    pub to_label:       String,
    pub result:         f64,
    pub ingredient:     Option<String>,
    pub localized_name: Option<String>,
    pub slug:           Option<String>,
    pub image_url:      Option<String>,
}

#[derive(Serialize)]
pub struct PopularResponse {
    pub conversions: Vec<PopularConversion>,
}

/// GET /public/tools/popular-conversions?lang=ru
pub async fn popular_conversions(
    State(pool): State<PgPool>,
    Query(params): Query<PopularQuery>,
) -> Json<PopularResponse> {
    let lang = parse_lang(&params.lang);

    let db_rows: Vec<CatalogNutritionRow> = sqlx::query_as(
        &format!(
            r#"SELECT {}
               FROM catalog_ingredients
               WHERE is_active = true"#,
            crate::domain::tools::catalog_row::CATALOG_NUTRITION_COLS
        ),
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let find_db = |name: &str| -> Option<&CatalogNutritionRow> {
        let n = name.to_lowercase();
        db_rows.iter().find(|r| r.name_en.to_lowercase() == n || r.slug.as_deref() == Some(name))
            .or_else(|| db_rows.iter().find(|r| r.name_en.to_lowercase().contains(&n)))
    };

    let conversions = POPULAR_CONVERSIONS.iter().filter_map(|e| {
        let result = match (e.ingredient, e.density) {
            (Some(_), Some(d)) => uc::convert_with_density(e.value, e.from_unit, e.to_unit, d),
            _ => uc::convert_units(e.value, e.from_unit, e.to_unit),
        };
        result.map(|r| {
            let db = e.ingredient.and_then(find_db);
            PopularConversion {
                value:          e.value,
                from_unit:      e.from_unit.to_string(),
                from_label:     label(e.from_unit, lang),
                to_unit:        e.to_unit.to_string(),
                to_label:       label(e.to_unit, lang),
                result:         uc::display_round(r),
                ingredient:     e.ingredient.map(|s| s.to_string()),
                localized_name: db.map(|row| row.localized_name(lang).to_string()),
                slug:           db.and_then(|row| row.slug.clone()),
                image_url:      db.and_then(|row| row.image_url.clone()),
            }
        })
    }).collect();

    Json(PopularResponse { conversions })
}
