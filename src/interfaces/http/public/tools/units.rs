//! Unit conversion handlers: convert_units, list_units, ingredient_scale,
//! ingredient_convert (density-aware cross-group converter).

use super::shared::{label, label_short, parse_lang, sanitize_value, SmartUnit};
use crate::domain::tools::unit_converter as uc;
use crate::shared::Language;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConvertQuery {
    pub value: f64,
    pub from:  String,
    pub to:    String,
    pub lang:  Option<String>,
}

#[derive(Serialize)]
pub struct ConvertResponse {
    pub value:        f64,
    pub from:         String,
    pub to:           String,
    pub result:       f64,
    pub from_label:   String,
    pub to_label:     String,
    pub supported:    bool,
    pub smart_result: Option<SmartUnit>,
}

#[derive(Deserialize)]
pub struct UnitsQuery {
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct UnitItem {
    pub code:  &'static str,
    pub label: String,
}

#[derive(Serialize)]
pub struct UnitsResponse {
    pub mass:    Vec<UnitItem>,
    pub volume:  Vec<UnitItem>,
    pub kitchen: Vec<UnitItem>,
}

#[derive(Deserialize)]
pub struct IngredientScaleQuery {
    pub ingredient:    Option<String>,
    pub value:         f64,
    pub unit:          Option<String>,
    pub from_portions: f64,
    pub to_portions:   f64,
    pub lang:          Option<String>,
}

#[derive(Serialize)]
pub struct IngredientScaleResponse {
    pub ingredient:    Option<String>,
    pub original_value: f64,
    pub unit:           String,
    pub from_portions:  f64,
    pub to_portions:    f64,
    pub scaled_value:   f64,
    pub smart_result:   Option<SmartUnit>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /public/tools/convert?value=100&from=g&to=oz&lang=ru
pub async fn convert_units(Query(params): Query<ConvertQuery>) -> Json<ConvertResponse> {
    let lang = parse_lang(&params.lang);

    let Some(value) = sanitize_value(params.value) else {
        return Json(ConvertResponse {
            value: 0.0,
            from: params.from.clone(),
            to: params.to.clone(),
            result: 0.0,
            from_label: label(&params.from, lang),
            to_label:   label(&params.to,   lang),
            supported: false,
            smart_result: None,
        });
    };

    let result_raw = uc::convert_units(value, &params.from, &params.to);
    let supported  = result_raw.is_some();
    let result     = uc::display_round(result_raw.unwrap_or(0.0));

    let smart_result = if supported {
        if uc::is_mass(&params.to) {
            let grams = result * uc::mass_to_grams(&params.to).unwrap_or(1.0);
            let (su, sv) = uc::smart_mass_unit(grams);
            Some(SmartUnit { value: uc::smart_round(sv), unit: su.to_string(), label: label(su, lang) })
        } else if uc::is_volume(&params.to) {
            let ml = result * uc::volume_to_ml(&params.to).unwrap_or(1.0);
            let (su, sv) = uc::smart_volume_unit(ml);
            Some(SmartUnit { value: uc::smart_round(sv), unit: su.to_string(), label: label(su, lang) })
        } else {
            None
        }
    } else {
        None
    };

    Json(ConvertResponse {
        from_label: label(&params.from, lang),
        to_label:   label(&params.to,   lang),
        value,
        from: params.from,
        to:   params.to,
        result,
        supported,
        smart_result,
    })
}

/// GET /public/tools/units?lang=pl
pub async fn list_units(Query(params): Query<UnitsQuery>) -> Json<UnitsResponse> {
    let lang = parse_lang(&params.lang);
    let make = |code: &'static str| UnitItem { code, label: label(code, lang) };

    Json(UnitsResponse {
        mass:    uc::mass_units().iter().map(|c| make(c)).collect(),
        volume:  vec![make("ml"), make("l"), make("fl_oz"), make("pint"), make("quart"), make("gallon")],
        kitchen: vec![make("tsp"), make("tbsp"), make("cup"), make("dash"), make("pinch"), make("drop"), make("stick_butter")],
    })
}

/// GET /public/tools/ingredient-scale?ingredient=flour&value=200&unit=g&from_portions=4&to_portions=10&lang=ru
pub async fn ingredient_scale(Query(params): Query<IngredientScaleQuery>) -> Json<IngredientScaleResponse> {
    let lang   = parse_lang(&params.lang);
    let unit   = params.unit.as_deref().unwrap_or("g");
    let scaled = uc::display_round(uc::scale(params.value, params.from_portions, params.to_portions));

    let smart_result = if uc::is_mass(unit) {
        let grams = scaled * uc::mass_to_grams(unit).unwrap_or(1.0);
        let (su, sv) = uc::smart_mass_unit(grams);
        Some(SmartUnit { value: uc::display_round(sv), unit: su.to_string(), label: label(su, lang) })
    } else if uc::is_volume(unit) {
        let ml = scaled * uc::volume_to_ml(unit).unwrap_or(1.0);
        let (su, sv) = uc::smart_volume_unit(ml);
        Some(SmartUnit { value: uc::display_round(sv), unit: su.to_string(), label: label(su, lang) })
    } else {
        None
    };

    Json(IngredientScaleResponse {
        ingredient:     params.ingredient,
        original_value: params.value,
        unit:           unit.to_string(),
        from_portions:  params.from_portions,
        to_portions:    params.to_portions,
        scaled_value:   scaled,
        smart_result,
    })
}

// ── ingredient-convert ────────────────────────────────────────────────────────

/// DB row for density + nutrition lookup
#[derive(sqlx::FromRow)]
struct IngredientConvertRow {
    slug:             Option<String>,
    name_en:          String,
    name_ru:          String,
    name_pl:          String,
    name_uk:          String,
    image_url:        Option<String>,
    density_g_per_ml: Option<rust_decimal::Decimal>,
    // Nutrition per 100 g
    calories_per_100g:   Option<i32>,
    protein_per_100g:    Option<rust_decimal::Decimal>,
    fat_per_100g:        Option<rust_decimal::Decimal>,
    carbs_per_100g:      Option<rust_decimal::Decimal>,
}

#[derive(Deserialize)]
pub struct IngredientConvertQuery {
    pub ingredient: String,
    pub value:      f64,
    pub from:       String,
    pub to:         String,
    pub lang:       Option<String>,
}

#[derive(Serialize)]
pub struct NutritionForResult {
    pub calories: f64,
    pub protein:  f64,
    pub fat:      f64,
    pub carbs:    f64,
}

#[derive(Serialize)]
pub struct Equivalents {
    pub tbsp: f64,
    pub tsp:  f64,
}

#[derive(Serialize)]
pub struct SeoMeta {
    pub title: String,
    pub h1:    String,
    pub text:  String,
}

#[derive(Serialize)]
pub struct IngredientConvertResponse {
    pub ingredient:           String,
    pub ingredient_name:      String,
    pub slug:                 String,
    pub image_url:            Option<String>,
    pub value:                f64,
    pub from:                 String,
    pub from_label:           String,
    pub to:                   String,
    pub to_label:             String,
    pub to_label_short:       &'static str,
    pub result:               f64,
    pub density_g_per_ml:     f64,
    pub source_volume_ml:     Option<f64>,  // ml of source volume (e.g. 236 for 1 cup)
    pub equivalents:          Option<Equivalents>,
    pub nutrition_for_result: Option<NutritionForResult>,
    pub related:              Vec<String>,
    pub seo:                  SeoMeta,
    pub question:             String,
    pub answer:               String,
}

type ApiError = (StatusCode, Json<serde_json::Value>);

/// GET /public/tools/ingredient-convert?ingredient=flour&value=1&from=cup&to=g&lang=pl
///
/// Density-aware converter. Supports all 4 scenarios:
///   mass → mass, volume → volume, volume → mass, mass → volume
///
/// Returns: result, equivalents (tbsp/tsp), nutrition_for_result, question, answer
pub async fn ingredient_convert(
    State(pool): State<PgPool>,
    Query(params): Query<IngredientConvertQuery>,
) -> Result<Json<IngredientConvertResponse>, ApiError> {
    let lang = parse_lang(&params.lang);

    // ── DB lookup: slug OR any language name, exact → prefix → substring ────
    let row: Option<IngredientConvertRow> = sqlx::query_as(
        r#"
        SELECT slug, name_en, name_ru, name_pl, name_uk, image_url, density_g_per_ml,
               calories_per_100g, protein_per_100g, fat_per_100g, carbs_per_100g
        FROM catalog_ingredients
        WHERE is_active = true
          AND (
            slug = $1
            OR LOWER(name_en) = LOWER($1)
            OR LOWER(name_pl) = LOWER($1)
            OR LOWER(name_ru) = LOWER($1)
            OR LOWER(name_uk) = LOWER($1)
            OR slug ILIKE '%' || $1 || '%'
            OR LOWER(name_en) ILIKE '%' || $1 || '%'
            OR LOWER(name_pl) ILIKE '%' || $1 || '%'
            OR LOWER(name_ru) ILIKE '%' || $1 || '%'
            OR LOWER(name_uk) ILIKE '%' || $1 || '%'
          )
        ORDER BY
          (slug = $1)::int +
          (LOWER(name_en) = LOWER($1))::int +
          (LOWER(name_pl) = LOWER($1))::int DESC
        LIMIT 1
        "#,
    )
    .bind(&params.ingredient)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("ingredient_convert DB error: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
    })?;

    let row = row.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(json!({ "error": "Ingredient not found" })))
    })?;

    let density = row.density_g_per_ml
        .and_then(|d| d.to_f64())
        .ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(json!({ "error": "Ingredient has no density data" })))
        })?;

    let raw = uc::convert_with_density(params.value, &params.from, &params.to, density)
        .ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(json!({ "error": "Unsupported unit pair" })))
        })?;

    // kitchen_round: nearest 0.05 when result is ≥0.9 and ≤3 cups range,
    // otherwise smart_round (2 dp ≥1, 3 dp <1)
    let result = kitchen_round(raw);

    // ── Localised name ────────────────────────────────────────────────────────
    let ingredient_name = match lang {
        Language::Pl => &row.name_pl,
        Language::Ru => &row.name_ru,
        Language::Uk => &row.name_uk,
        Language::En => &row.name_en,
    }.clone();

    let from_label = label(&params.from, lang);
    let to_label   = label(&params.to,   lang);
    let to_short   = label_short(&params.to);

    // ── source_volume_ml: ml in source unit (if it's a volume) ───────────────
    let source_volume_ml = uc::volume_to_ml(&params.from)
        .map(|ml_per| params.value * ml_per);

    // ── Question / Answer with natural grammar ────────────────────────────────
    //
    // EN:  "How many g in 1 cup of Wheat flour?"   (short unit code — SEO standard)
    //      "1 cup of Wheat flour equals 125 g."
    //
    // PL:  "Ile gramów ma 1 szklanka Mąka pszenna?"  (localised word)
    //      "1 szklanka Mąka pszenna = 125 gramów."
    //
    // RU:  "Сколько граммов в 1 стакан Рис?"
    //      "1 стакан Рис = 182 грамма."
    //
    // UK:  same pattern with Ukrainian localised labels

    let question = match lang {
        Language::En => format!(
            "How many {} in {} {} of {}?",
            to_short, params.value, from_label, ingredient_name
        ),
        Language::Pl => format!(
            "Ile {} ma {} {} {}?",
            to_label, params.value, from_label, ingredient_name
        ),
        Language::Ru => format!(
            "Сколько {} в {} {} {}?",
            to_label, params.value, from_label, ingredient_name
        ),
        Language::Uk => format!(
            "Скільки {} у {} {} {}?",
            to_label, params.value, from_label, ingredient_name
        ),
    };

    let answer = match lang {
        Language::En => format!(
            "{} {} of {} equals {} {}.",
            params.value, from_label, ingredient_name, result, to_short
        ),
        Language::Pl => format!(
            "{} {} {} = {} {}.",
            params.value, from_label, ingredient_name, result, to_label
        ),
        Language::Ru => format!(
            "{} {} {} = {} {}.",
            params.value, from_label, ingredient_name, result, to_label
        ),
        Language::Uk => format!(
            "{} {} {} = {} {}.",
            params.value, from_label, ingredient_name, result, to_label
        ),
    };

    // ── Equivalents: always in grams, expressed in tbsp/tsp ──────────────────
    // result_g = how many grams the converted result represents
    let result_g: Option<f64> = if uc::is_mass(&params.to) {
        let per_g = uc::mass_to_grams(&params.to).unwrap_or(1.0);
        Some(result * per_g)
    } else if uc::is_volume(&params.to) {
        // volume result → grams via density
        let per_ml = uc::volume_to_ml(&params.to).unwrap_or(1.0);
        Some(result * per_ml * density)
    } else {
        None
    };

    let equivalents = result_g.map(|g| {
        let tbsp_g = uc::TBSP_ML * density;
        let tsp_g  = uc::TSP_ML  * density;
        Equivalents {
            tbsp: uc::smart_round(g / tbsp_g),
            tsp:  uc::smart_round(g / tsp_g),
        }
    });

    // ── Nutrition for result ──────────────────────────────────────────────────
    let nutrition_for_result = result_g.and_then(|g| {
        let cal = row.calories_per_100g? as f64;
        let pro = row.protein_per_100g.and_then(|x| x.to_f64())?;
        let fat = row.fat_per_100g.and_then(|x| x.to_f64())?;
        let crb = row.carbs_per_100g.and_then(|x| x.to_f64())?;
        let factor = g / 100.0;
        Some(NutritionForResult {
            calories: uc::round_to(cal * factor, 1),
            protein:  uc::round_to(pro * factor, 1),
            fat:      uc::round_to(fat * factor, 1),
            carbs:    uc::round_to(crb * factor, 1),
        })
    });

    // ── Related conversions ───────────────────────────────────────────────────
    let slug_ref = row.slug.as_deref().unwrap_or(&params.ingredient);
    let related = build_related(&params.from, &params.to, slug_ref);

    // ── SEO metadata ──────────────────────────────────────────────────────────
    let seo = build_seo(
        &params.from, &params.to, &from_label, to_short, &to_label,
        &ingredient_name, params.value, result, density,
        lang,
    );

    Ok(Json(IngredientConvertResponse {
        ingredient:           params.ingredient,
        ingredient_name,
        slug:                 row.slug.unwrap_or_default(),
        image_url:            row.image_url,
        value:                params.value,
        from:                 params.from,
        from_label,
        to:                   params.to,
        to_label,
        to_label_short:       to_short,
        result,
        density_g_per_ml:     density,
        source_volume_ml,
        equivalents,
        nutrition_for_result,
        related,
        seo,
        question,
        answer,
    }))
}

// ── SEO-friendly alias ────────────────────────────────────────────────────────

/// Path params for `/public/tools/{from}-to-{to}/{slug}`
#[derive(Deserialize)]
pub struct SeoConvertPath {
    pub from_to: String,   // e.g. "cup-to-grams"
    pub slug:    String,   // e.g. "wheat-flour"
}

#[derive(Deserialize)]
pub struct SeoConvertQuery {
    pub value: Option<f64>,
    pub lang:  Option<String>,
}

/// GET /public/tools/cup-to-grams/wheat-flour?value=1&lang=pl
///
/// SEO-friendly alias. Parses `{from}-to-{to}` into unit codes, then delegates
/// to the same logic as `ingredient_convert`.
pub async fn seo_ingredient_convert(
    State(pool): State<PgPool>,
    axum::extract::Path(path): axum::extract::Path<SeoConvertPath>,
    Query(q): Query<SeoConvertQuery>,
) -> Result<Json<IngredientConvertResponse>, ApiError> {
    // Parse "cup-to-grams" → from="cup", to="g"
    let (from, to) = parse_seo_from_to(&path.from_to).ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(json!({ "error": "Cannot parse unit path. Use e.g. cup-to-grams" })))
    })?;

    let params = IngredientConvertQuery {
        ingredient: path.slug,
        value:      q.value.unwrap_or(1.0),
        from:       from.to_string(),
        to:         to.to_string(),
        lang:       q.lang,
    };

    // Re-use existing handler logic
    let pool_state = axum::extract::State(pool);
    ingredient_convert(pool_state, Query(params)).await
}

/// Parse "cup-to-grams" → ("cup", "g"),  "tablespoon-to-grams" → ("tbsp", "g"), etc.
fn parse_seo_from_to(s: &str) -> Option<(&'static str, &'static str)> {
    // Map human-readable names → unit codes
    fn canonical(word: &str) -> Option<&'static str> {
        match word {
            "grams" | "gram" | "g"            => Some("g"),
            "kilograms" | "kilogram" | "kg"   => Some("kg"),
            "oz" | "ounce" | "ounces"         => Some("oz"),
            "lb" | "lbs" | "pound" | "pounds" => Some("lb"),
            "mg" | "milligrams"               => Some("mg"),
            "ml" | "milliliters"              => Some("ml"),
            "l" | "liters" | "litres"         => Some("l"),
            "cup" | "cups"                    => Some("cup"),
            "tbsp" | "tablespoon" | "tablespoons" => Some("tbsp"),
            "tsp" | "teaspoon" | "teaspoons"  => Some("tsp"),
            "fl_oz" | "floz" | "fluid-oz"     => Some("fl_oz"),
            "pint" | "pints"                  => Some("pint"),
            "quart" | "quarts"                => Some("quart"),
            "gallon" | "gallons"              => Some("gallon"),
            _                                 => None,
        }
    }

    // Expect exactly one "-to-" separator
    let idx = s.find("-to-")?;
    let from_str = &s[..idx];
    let to_str   = &s[idx + 4..];
    let from = canonical(from_str)?;
    let to   = canonical(to_str)?;
    Some((from, to))
}

// ── Kitchen rounding ──────────────────────────────────────────────────────────

/// Rounds converted values in a kitchen-friendly way:
///   • ≥ 10 g/ml → round to nearest integer   (182.15 → 182)
///   • ≥ 1       → snap to nearest 0.05 if within 2%  (0.997 → 1.00, 1.498 → 1.50)
///   • < 1       → 3 decimal places             (0.042)
fn kitchen_round(v: f64) -> f64 {
    if v >= 10.0 {
        v.round()
    } else if v >= 1.0 {
        let step = 0.05_f64;
        let snapped = (v / step).round() * step;
        let rel_err = (v - snapped).abs() / v.max(0.001);
        if rel_err < 0.02 {
            uc::round_to(snapped, 2)
        } else {
            uc::round_to(v, 2)
        }
    } else {
        uc::round_to(v, 3)
    }
}

// ── Related conversions ───────────────────────────────────────────────────────

/// Returns 3 related SEO-path strings for cross-linking.
/// Excludes the current from→to pair.
fn build_related(from: &str, to: &str, slug: &str) -> Vec<String> {
    // Common useful pairs, priority-ordered
    let pairs: &[(&str, &str)] = &[
        ("cup",  "g"),
        ("tbsp", "g"),
        ("tsp",  "g"),
        ("g",    "cup"),
        ("g",    "tbsp"),
        ("g",    "tsp"),
        ("oz",   "g"),
        ("g",    "oz"),
        ("ml",   "g"),
        ("g",    "ml"),
    ];

    pairs.iter()
        .filter(|(f, t)| !(*f == from && *t == to))
        .take(3)
        .map(|(f, t)| format!("{}-to-{}/{}", seo_unit_name(f), seo_unit_name(t), slug))
        .collect()
}

/// Maps an internal unit code to its SEO-friendly URL segment.
fn seo_unit_name(unit: &str) -> &'static str {
    match unit {
        "g"    => "grams",
        "kg"   => "kilograms",
        "oz"   => "ounces",
        "lb"   => "pounds",
        "mg"   => "milligrams",
        "ml"   => "milliliters",
        "l"    => "liters",
        "cup"  => "cup",
        "tbsp" => "tablespoon",
        "tsp"  => "teaspoon",
        "pint" => "pint",
        "quart"  => "quart",
        "gallon" => "gallon",
        _      => "unit",
    }
}

// ── SEO metadata ──────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_seo(
    from: &str, to: &str,
    from_label: &str, to_short: &str, to_label: &str,
    ingredient_name: &str,
    value: f64, result: f64, density: f64,
    lang: Language,
) -> SeoMeta {
    let density_str = uc::round_to(density, 2);

    match lang {
        Language::En => SeoMeta {
            title: format!(
                "{} {} {} in {} ({}) – Kitchen Converter",
                value, from_label, ingredient_name,
                to_short.to_uppercase(), to_short
            ),
            h1: format!(
                "How many {} in {} {} of {}?",
                to_short, value, from_label, ingredient_name
            ),
            text: format!(
                "{} {} of {} equals {} {}.\n\nThis conversion is based on an average density of {} g/ml.",
                value, from_label, ingredient_name, result, to_short, density_str
            ),
        },
        Language::Pl => SeoMeta {
            title: format!(
                "{} {} {} w {} – Przelicznik kuchenny",
                value, from_label, ingredient_name, to_short
            ),
            h1: format!(
                "Ile {} ma {} {} {}?",
                to_label, value, from_label, ingredient_name
            ),
            text: format!(
                "{} {} {} = {} {}.\n\nPrzeliczenie opiera się na gęstości {} g/ml.",
                value, from_label, ingredient_name, result, to_label, density_str
            ),
        },
        Language::Ru => SeoMeta {
            title: format!(
                "{} {} {} в {} – Кухонный конвертер",
                value, from_label, ingredient_name, to_short
            ),
            h1: format!(
                "Сколько {} в {} {} {}?",
                to_label, value, from_label, ingredient_name
            ),
            text: format!(
                "{} {} {} = {} {}.\n\nРасчёт основан на плотности {} г/мл.",
                value, from_label, ingredient_name, result, to_label, density_str
            ),
        },
        Language::Uk => SeoMeta {
            title: format!(
                "{} {} {} у {} – Кухонний конвертер",
                value, from_label, ingredient_name, to_short
            ),
            h1: format!(
                "Скільки {} у {} {} {}?",
                to_label, value, from_label, ingredient_name
            ),
            text: format!(
                "{} {} {} = {} {}.\n\nРозрахунок базується на густині {} г/мл.",
                value, from_label, ingredient_name, result, to_label, density_str
            ),
        },
    }
}
