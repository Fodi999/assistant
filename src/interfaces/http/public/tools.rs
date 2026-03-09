use crate::domain::tools::unit_converter as uc;
use crate::shared::Language;
use axum::{extract::{Query, State}, response::Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time;

// ── Language helper ───────────────────────────────────────────────────────────

fn parse_lang(lang: &Option<String>) -> Language {
    lang.as_deref()
        .and_then(Language::from_code)
        .unwrap_or_default()
}

// ── Unit labels — static slice, zero allocations ──────────────────────────────

pub struct UnitLabel {
    pub en: &'static str,
    pub pl: &'static str,
    pub ru: &'static str,
    pub uk: &'static str,
}

impl UnitLabel {
    fn for_lang(&self, lang: Language) -> &'static str {
        match lang {
            Language::Pl => self.pl,
            Language::Ru => self.ru,
            Language::Uk => self.uk,
            Language::En => self.en,
        }
    }
}

static UNIT_LABELS: &[(&str, UnitLabel)] = &[
    // Mass
    ("g",     UnitLabel { en: "gram",         pl: "gram",         ru: "грамм",           uk: "грам"          }),
    ("mg",    UnitLabel { en: "milligram",     pl: "miligram",     ru: "миллиграмм",      uk: "міліграм"      }),
    ("kg",    UnitLabel { en: "kilogram",      pl: "kilogram",     ru: "килограмм",       uk: "кілограм"      }),
    ("oz",    UnitLabel { en: "ounce",         pl: "uncja",        ru: "унция",           uk: "унція"         }),
    ("lb",    UnitLabel { en: "pound",         pl: "funt",         ru: "фунт",            uk: "фунт"          }),
    // Volume
    ("ml",    UnitLabel { en: "milliliter",    pl: "mililitr",     ru: "миллилитр",       uk: "мілілітр"      }),
    ("l",     UnitLabel { en: "liter",         pl: "litr",         ru: "литр",            uk: "літр"          }),
    ("fl_oz", UnitLabel { en: "fl. ounce",     pl: "fl. uncja",    ru: "жидк. унция",     uk: "рід. унція"    }),
    // Kitchen
    ("tsp",   UnitLabel { en: "teaspoon",      pl: "łyżeczka",     ru: "чайная ложка",    uk: "чайна ложка"   }),
    ("tbsp",  UnitLabel { en: "tablespoon",    pl: "łyżka",        ru: "столовая ложка",  uk: "столова ложка" }),
    ("cup",   UnitLabel { en: "cup",           pl: "szklanka",     ru: "стакан",          uk: "склянка"       }),
    ("pint",  UnitLabel { en: "pint",          pl: "pinta",        ru: "пинта",           uk: "пінта"         }),
    ("quart", UnitLabel { en: "quart",         pl: "kwarta",       ru: "кварта",          uk: "кварта"        }),
    ("gallon",UnitLabel { en: "gallon",        pl: "galon",        ru: "галлон",          uk: "галон"         }),
    // Micro-kitchen
    ("dash",        UnitLabel { en: "dash",         pl: "odrobina",    ru: "щепотка",      uk: "дрібка"       }),
    ("pinch",       UnitLabel { en: "pinch",        pl: "szczypta",    ru: "щепотка",      uk: "щіпка"        }),
    ("drop",        UnitLabel { en: "drop",         pl: "kropla",      ru: "капля",        uk: "крапля"       }),
    ("stick_butter",UnitLabel { en: "stick butter", pl: "kostka masła",ru: "палочка масла",uk: "паличка масла"}),
];

fn label(unit: &str, lang: Language) -> String {
    UNIT_LABELS
        .iter()
        .find(|(code, _)| *code == unit)
        .map(|(_, l)| l.for_lang(lang).to_string())
        .unwrap_or_else(|| unit.to_string())
}

// ── Guard helpers ─────────────────────────────────────────────────────────────

fn sanitize_value(v: f64) -> Option<f64> {
    if v.is_nan() || v.is_infinite() {
        None
    } else {
        Some(v.clamp(-1_000_000.0, 1_000_000.0))
    }
}

// ── 1. Unit converter ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConvertQuery {
    pub value: f64,
    pub from: String,
    pub to: String,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct ConvertResponse {
    pub value: f64,
    pub from: String,
    pub to: String,
    pub result: f64,
    pub from_label: String,
    pub to_label: String,
    pub supported: bool,
    pub smart_result: Option<SmartUnit>,
}

#[derive(Serialize)]
pub struct SmartUnit {
    pub value: f64,
    pub unit: String,
    pub label: String,
}

/// GET /public/tools/convert?value=100&from=g&to=oz&lang=ru
pub async fn convert_units(Query(params): Query<ConvertQuery>) -> Json<ConvertResponse> {
    let lang = parse_lang(&params.lang);

    let Some(value) = sanitize_value(params.value) else {
        return Json(ConvertResponse {
            value: 0.0, from: params.from.clone(), to: params.to.clone(),
            result: 0.0, from_label: label(&params.from, lang),
            to_label: label(&params.to, lang), supported: false, smart_result: None,
        });
    };

    let result_raw = uc::convert_units(value, &params.from, &params.to);
    let supported = result_raw.is_some();
    let result = uc::display_round(result_raw.unwrap_or(0.0));

    // Smart auto-unit for the result
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
        value, from: params.from, to: params.to,
        result, supported, smart_result,
    })
}

// ── 2. Units list ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UnitsQuery {
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct UnitItem {
    pub code: &'static str,
    pub label: String,
}

#[derive(Serialize)]
pub struct UnitsResponse {
    pub mass:    Vec<UnitItem>,
    pub volume:  Vec<UnitItem>,
    pub kitchen: Vec<UnitItem>,
}

/// GET /public/tools/units?lang=pl
pub async fn list_units(Query(params): Query<UnitsQuery>) -> Json<UnitsResponse> {
    let lang = parse_lang(&params.lang);

    let make = |code: &'static str| UnitItem {
        code,
        label: label(code, lang),
    };

    Json(UnitsResponse {
        mass:    uc::mass_units().iter().map(|c| make(c)).collect(),
        volume:  vec![make("ml"), make("l"), make("fl_oz"), make("pint"), make("quart"), make("gallon")],
        kitchen: vec![make("tsp"), make("tbsp"), make("cup"), make("dash"), make("pinch"), make("drop"), make("stick_butter")],
    })
}

// ── 3. Fish season (fully DB-driven from catalog_ingredients) ─────────────────

#[derive(Deserialize)]
pub struct FishQuery {
    #[serde(default = "default_fish")]
    pub fish: String,
    pub lang: Option<String>,
}
fn default_fish() -> String { "salmon".to_string() }

#[derive(Serialize)]
pub struct FishSeasonEntry { pub month: u8, pub month_name: String, pub available: bool }

#[derive(Serialize)]
pub struct FishSeasonResponse { pub fish: String, pub season: Vec<FishSeasonEntry> }

fn month_name(m: u8, lang: Language) -> &'static str {
    match lang {
        Language::Ru => match m {
            1=>"Январь",2=>"Февраль",3=>"Март",4=>"Апрель",5=>"Май",6=>"Июнь",
            7=>"Июль",8=>"Август",9=>"Сентябрь",10=>"Октябрь",11=>"Ноябрь",12=>"Декабрь",_=>"—",
        },
        Language::Pl => match m {
            1=>"Styczeń",2=>"Luty",3=>"Marzec",4=>"Kwiecień",5=>"Maj",6=>"Czerwiec",
            7=>"Lipiec",8=>"Sierpień",9=>"Wrzesień",10=>"Październik",11=>"Listopad",12=>"Grudzień",_=>"—",
        },
        Language::Uk => match m {
            1=>"Січень",2=>"Лютий",3=>"Березень",4=>"Квітень",5=>"Травень",6=>"Червень",
            7=>"Липень",8=>"Серпень",9=>"Вересень",10=>"Жовтень",11=>"Листопад",12=>"Грудень",_=>"—",
        },
        Language::En => match m {
            1=>"January",2=>"February",3=>"March",4=>"April",5=>"May",6=>"June",
            7=>"July",8=>"August",9=>"September",10=>"October",11=>"November",12=>"December",_=>"—",
        },
    }
}

/// GET /public/tools/fish-season?fish=salmon&lang=ru
///
/// Returns month-by-month availability for a single fish from catalog DB.
pub async fn fish_season(
    State(pool): State<PgPool>,
    Query(params): Query<FishQuery>,
) -> Json<FishSeasonResponse> {
    let lang = parse_lang(&params.lang);
    let fish_lower = params.fish.to_lowercase();

    // Fetch availability_months from catalog
    let row: Option<(Vec<bool>,)> = sqlx::query_as(
        r#"
        SELECT availability_months
        FROM catalog_ingredients
        WHERE is_active = true AND slug = $1
          AND availability_months IS NOT NULL
        "#,
    )
    .bind(&fish_lower)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let months = row
        .map(|(m,)| m)
        .unwrap_or_else(|| vec![true; 12]); // fallback: available all year

    let season = (1u8..=12).map(|m| FishSeasonEntry {
        month: m,
        month_name: month_name(m, lang).to_string(),
        available: months.get((m - 1) as usize).copied().unwrap_or(true),
    }).collect();

    Json(FishSeasonResponse { fish: params.fish, season })
}

// ── 3b. Fish season table (fully DB-driven) ──────────────────────────────────

#[derive(sqlx::FromRow)]
struct FishCatalogRow {
    slug: Option<String>,
    name_en: String,
    name_ru: String,
    name_pl: String,
    name_uk: String,
    image_url: Option<String>,
    availability_months: Option<Vec<bool>>,
}

#[derive(Serialize)]
pub struct FishSeasonTableItem {
    pub slug: String,
    pub name: String,
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub image_url: Option<String>,
    pub season: Vec<FishSeasonEntry>,
}

#[derive(Serialize)]
pub struct FishSeasonTableResponse {
    pub fish: Vec<FishSeasonTableItem>,
    pub lang: String,
}

/// Fish & Seafood category ID (stable UUID in our catalog)
const FISH_CATEGORY_ID: &str = "503794cf-37e0-48c1-a6d8-b5c3f21e03a1";

/// GET /public/tools/fish-season-table?lang=ru
///
/// Returns ALL active fish from catalog_ingredients (category = Fish & Seafood)
/// with availability_months, localized names and images.
/// Single source of truth = catalog DB.
pub async fn fish_season_table(
    State(pool): State<PgPool>,
    Query(params): Query<FishQuery>,
) -> Json<FishSeasonTableResponse> {
    let lang = parse_lang(&params.lang);
    let lang_code = match lang {
        Language::Ru => "ru",
        Language::Pl => "pl",
        Language::Uk => "uk",
        Language::En => "en",
    };

    // Fetch all fish from catalog by category
    let db_rows: Vec<FishCatalogRow> = sqlx::query_as(
        r#"
        SELECT slug, name_en, name_ru, name_pl, name_uk, image_url, availability_months
        FROM catalog_ingredients
        WHERE is_active = true
          AND category_id = $1::uuid
          AND availability_months IS NOT NULL
        ORDER BY name_en
        "#,
    )
    .bind(FISH_CATEGORY_ID)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let fish = db_rows.iter().map(|r| {
        let slug = r.slug.clone().unwrap_or_default();
        let months = r.availability_months.clone().unwrap_or_else(|| vec![true; 12]);

        let name = match lang {
            Language::Ru => r.name_ru.clone(),
            Language::Pl => r.name_pl.clone(),
            Language::Uk => r.name_uk.clone(),
            Language::En => r.name_en.clone(),
        };

        let season = (1u8..=12).map(|m| FishSeasonEntry {
            month: m,
            month_name: month_name(m, lang).to_string(),
            available: months.get((m - 1) as usize).copied().unwrap_or(true),
        }).collect();

        FishSeasonTableItem {
            slug,
            name,
            name_en: r.name_en.clone(),
            name_ru: r.name_ru.clone(),
            name_pl: r.name_pl.clone(),
            name_uk: r.name_uk.clone(),
            image_url: r.image_url.clone(),
            season,
        }
    }).collect();

    Json(FishSeasonTableResponse {
        fish,
        lang: lang_code.to_string(),
    })
}

// ── 4. Nutrition (DB-first, static fallback) ─────────────────────────────────

/// Row from catalog_ingredients with nutrition + image + translations
#[derive(sqlx::FromRow, Clone)]
struct CatalogNutritionRow {
    name_en: String,
    name_ru: String,
    name_pl: String,
    name_uk: String,
    image_url: Option<String>,
    slug: Option<String>,
    calories_per_100g: Option<i32>,
    protein_per_100g: Option<rust_decimal::Decimal>,
    fat_per_100g:     Option<rust_decimal::Decimal>,
    carbs_per_100g:   Option<rust_decimal::Decimal>,
    density_g_per_ml: Option<rust_decimal::Decimal>,
}

impl CatalogNutritionRow {
    fn cal(&self) -> f64 { self.calories_per_100g.unwrap_or(0) as f64 }
    fn prot(&self) -> f64 { self.protein_per_100g.and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d)).unwrap_or(0.0) }
    fn fat(&self) -> f64  { self.fat_per_100g.and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d)).unwrap_or(0.0) }
    fn carbs(&self) -> f64 { self.carbs_per_100g.and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d)).unwrap_or(0.0) }
    fn density(&self) -> f64 { self.density_g_per_ml.and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d)).unwrap_or(1.0) }
    fn localized_name(&self, lang: Language) -> &str {
        match lang {
            Language::Ru => &self.name_ru,
            Language::Pl => &self.name_pl,
            Language::Uk => &self.name_uk,
            Language::En => &self.name_en,
        }
    }
}

#[derive(Deserialize)]
pub struct NutritionQuery {
    #[serde(default = "default_fish")]
    pub name: String,
    pub amount: Option<f64>,
    pub unit: Option<String>,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct NutritionResponse {
    pub name: String,
    pub localized_name: String,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub amount_g: f64,
    pub calories: f64,
    pub protein_g: f64,
    pub fat_g: f64,
    pub carbs_g: f64,
    pub unit_label: String,
}

/// GET /public/tools/nutrition?name=salmon&amount=150&unit=g&lang=pl
///
/// Primary source: catalog_ingredients DB (with photo, translations, density).
/// Fallback: static table for ingredients not yet in catalog.
pub async fn nutrition(
    State(pool): State<PgPool>,
    Query(params): Query<NutritionQuery>,
) -> Json<NutritionResponse> {
    let lang = parse_lang(&params.lang);
    let name_lower = params.name.to_lowercase();
    let raw_amount = params.amount.unwrap_or(100.0);
    let unit = params.unit.as_deref().unwrap_or("g");

    // Try DB first — match on name_en, slug, or fuzzy
    let db_row: Option<CatalogNutritionRow> = sqlx::query_as(
        r#"
        SELECT name_en, name_ru, name_pl, name_uk, image_url, slug,
               calories_per_100g, protein_per_100g, fat_per_100g,
               carbs_per_100g, density_g_per_ml
        FROM catalog_ingredients
        WHERE is_active = true
          AND (LOWER(name_en) = $1
               OR slug = $1
               OR LOWER(name_en) LIKE '%' || $1 || '%')
        ORDER BY (LOWER(name_en) = $1 OR slug = $1) DESC, LENGTH(name_en) ASC
        LIMIT 1
        "#,
    )
    .bind(&name_lower)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let (cal, prot, fat, carbs, density, localized_name, slug, image_url) = if let Some(ref row) = db_row {
        (row.cal(), row.prot(), row.fat(), row.carbs(), row.density(),
         row.localized_name(lang).to_string(),
         row.slug.clone(),
         row.image_url.clone())
    } else {
        (0.0, 0.0, 0.0, 0.0, 1.0, params.name.clone(), None, None)
    };

    // Convert to grams
    let amount_g = if unit == "g" {
        raw_amount
    } else if let Some(g) = uc::mass_to_grams(unit) {
        raw_amount * g
    } else if let Some(ml_factor) = uc::volume_to_ml(unit) {
        raw_amount * ml_factor * density
    } else {
        raw_amount
    };

    let f = amount_g / 100.0;
    let r = |x: f64| uc::round_to(x, 1);

    Json(NutritionResponse {
        name: params.name,
        localized_name,
        slug,
        image_url,
        amount_g: uc::round_to(amount_g, 1),
        calories:  r(cal  * f),
        protein_g: r(prot * f),
        fat_g:     r(fat  * f),
        carbs_g:   r(carbs * f),
        unit_label: label("g", lang),
    })
}

// ── 5. Recipe scaler ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ScaleQuery {
    pub value: f64,
    pub from_portions: f64,
    pub to_portions: f64,
}

#[derive(Serialize)]
pub struct ScaleResponse {
    pub original: f64,
    pub from_portions: f64,
    pub to_portions: f64,
    pub result: f64,
}

/// GET /public/tools/scale?value=500&from_portions=4&to_portions=10
pub async fn scale_recipe(Query(params): Query<ScaleQuery>) -> Json<ScaleResponse> {
    let result = uc::round_to(uc::scale(params.value, params.from_portions, params.to_portions), 2);
    Json(ScaleResponse {
        original: params.value,
        from_portions: params.from_portions,
        to_portions: params.to_portions,
        result,
    })
}

// ── 6. Yield calculator ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct YieldQuery {
    pub raw: f64,
    pub usable: f64,
}

#[derive(Serialize)]
pub struct YieldResponse {
    pub raw: f64,
    pub usable: f64,
    pub yield_percent: f64,
    pub waste_percent: f64,
}

/// GET /public/tools/yield?raw=1000&usable=750
pub async fn yield_calc(Query(params): Query<YieldQuery>) -> Json<YieldResponse> {
    let yp = uc::round_to(uc::yield_percent(params.raw, params.usable), 2);
    Json(YieldResponse {
        raw: params.raw,
        usable: params.usable,
        yield_percent: yp,
        waste_percent: uc::round_to(100.0 - yp, 2),
    })
}

// ── 7. Ingredient equivalents (killer feature) ──────────────────────────────

#[derive(Deserialize)]
pub struct EquivalentsQuery {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct Equivalent {
    pub unit: String,
    pub label: String,
    pub value: f64,
}

#[derive(Serialize)]
pub struct EquivalentsResponse {
    pub name: String,
    pub input_value: f64,
    pub input_unit: String,
    pub equivalents: Vec<Equivalent>,
}

/// GET /public/tools/ingredient-equivalents?name=flour&value=100&unit=g&lang=ru
///
/// Returns the same amount in all possible units using ingredient density.
/// Now reads density from catalog_ingredients DB.
pub async fn ingredient_equivalents(
    State(pool): State<PgPool>,
    Query(params): Query<EquivalentsQuery>,
) -> Json<EquivalentsResponse> {
    let lang = parse_lang(&params.lang);
    let name_lower = params.name.to_lowercase();

    // DB lookup for density (by name_en, slug, or fuzzy)
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

    let target_units: &[&str] = &[
        "g", "kg", "oz", "lb",
        "ml", "l", "fl_oz",
        "tsp", "tbsp", "cup",
    ];

    let equivalents: Vec<Equivalent> = target_units
        .iter()
        .filter(|&&u| u != params.unit)
        .filter_map(|&u| {
            uc::convert_with_density(params.value, &params.unit, u, density)
                .map(|v| Equivalent {
                    unit: u.to_string(),
                    label: label(u, lang),
                    value: uc::display_round(v),
                })
        })
        .collect();

    Json(EquivalentsResponse {
        name: params.name,
        input_value: params.value,
        input_unit: params.unit,
        equivalents,
    })
}

// ── 8. Food cost calculator ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FoodCostQuery {
    /// Price for a given price_amount in price_unit (e.g. 5.50 for 1 kg)
    pub price: f64,
    /// How much of the price_unit the price covers (default 1)
    pub price_amount: Option<f64>,
    /// Unit of the price (default "kg")
    pub price_unit: Option<String>,
    /// Amount actually used
    pub amount: f64,
    /// Unit of the used amount (default same as price_unit)
    pub unit: Option<String>,
    /// Number of portions this produces
    pub portions: Option<f64>,
    /// Menu sell price per portion
    pub sell_price: Option<f64>,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct FoodCostResponse {
    pub price: f64,
    pub price_unit: String,
    pub amount: f64,
    pub unit: String,
    pub total_cost: f64,
    pub cost_per_portion: Option<f64>,
    pub sell_price: Option<f64>,
    pub margin_percent: Option<f64>,
    pub markup_percent: Option<f64>,
}

/// GET /public/tools/food-cost?price=5.50&price_unit=kg&amount=500&unit=g&portions=4&sell_price=15.0
///
/// `price` = cost for `price_amount` (default 1) of `price_unit` (default "kg").
/// `amount` = how much you actually use in `unit` (default = price_unit).
/// Converts amount to the same base as price_unit to compute total_cost.
pub async fn food_cost_calc(Query(params): Query<FoodCostQuery>) -> Json<FoodCostResponse> {
    let price_unit = params.price_unit.as_deref().unwrap_or("kg");
    let unit = params.unit.as_deref().unwrap_or(price_unit);
    let price_amount = params.price_amount.unwrap_or(1.0);

    // Convert used amount into the same unit as price_unit
    let amount_in_price_unit = if unit == price_unit {
        params.amount
    } else {
        uc::convert_units(params.amount, unit, price_unit).unwrap_or(params.amount)
    };

    // price_per_one = price / price_amount  →  total = price_per_one * amount_in_price_unit
    let price_per_one = if price_amount > 0.0 { params.price / price_amount } else { params.price };
    let total_cost = uc::round_to(price_per_one * amount_in_price_unit, 2);

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

// ── 9. Ingredient suggestions ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SuggestionsQuery {
    pub unit: String,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct Suggestion {
    pub name: String,
    pub name_en: String,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub density_g_per_ml: f64,
    pub equivalent_g: f64,
}

#[derive(Serialize)]
pub struct SuggestionsResponse {
    pub unit: String,
    pub ml_per_unit: Option<f64>,
    pub suggestions: Vec<Suggestion>,
}

/// GET /public/tools/ingredient-suggestions?unit=cup&lang=ru
///
/// Given a volume unit, returns common ingredients from catalog with their weight per that unit.
pub async fn ingredient_suggestions(
    State(pool): State<PgPool>,
    Query(params): Query<SuggestionsQuery>,
) -> Json<SuggestionsResponse> {
    let lang = parse_lang(&params.lang);
    let ml_factor = uc::volume_to_ml(&params.unit);

    let suggestions: Vec<Suggestion> = if let Some(ml) = ml_factor {
        // Fetch all ingredients with density from DB
        let rows: Vec<CatalogNutritionRow> = sqlx::query_as(
            r#"SELECT name_en, name_ru, name_pl, name_uk, image_url, slug,
                      calories_per_100g, protein_per_100g, fat_per_100g,
                      carbs_per_100g, density_g_per_ml
               FROM catalog_ingredients
               WHERE is_active = true AND density_g_per_ml IS NOT NULL
                 AND density_g_per_ml != 1.0
               ORDER BY name_en ASC"#,
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        rows.iter()
            .map(|r| {
                let density = r.density();
                let grams = uc::display_round(ml * density);
                Suggestion {
                    name: r.localized_name(lang).to_string(),
                    name_en: r.name_en.clone(),
                    slug: r.slug.clone(),
                    image_url: r.image_url.clone(),
                    density_g_per_ml: density,
                    equivalent_g: grams,
                }
            })
            .collect()
    } else {
        vec![]
    };

    Json(SuggestionsResponse {
        unit: params.unit,
        ml_per_unit: ml_factor,
        suggestions,
    })
}

// ── 10. Popular conversions (SEO) ────────────────────────────────────────────

struct PopularEntry {
    value: f64,
    from_unit: &'static str,
    to_unit: &'static str,
    ingredient: Option<&'static str>,
    density: Option<f64>,
}

static POPULAR_CONVERSIONS: &[PopularEntry] = &[
    // Flour
    PopularEntry { value: 1.0, from_unit: "cup",  to_unit: "g", ingredient: Some("flour"),  density: Some(0.53) },
    PopularEntry { value: 1.0, from_unit: "tbsp", to_unit: "g", ingredient: Some("flour"),  density: Some(0.53) },
    // Sugar
    PopularEntry { value: 1.0, from_unit: "cup",  to_unit: "g", ingredient: Some("sugar"),  density: Some(0.85) },
    PopularEntry { value: 1.0, from_unit: "tbsp", to_unit: "g", ingredient: Some("sugar"),  density: Some(0.85) },
    // Butter
    PopularEntry { value: 1.0, from_unit: "tbsp", to_unit: "g", ingredient: Some("butter"), density: Some(0.92) },
    PopularEntry { value: 1.0, from_unit: "cup",  to_unit: "g", ingredient: Some("butter"), density: Some(0.92) },
    PopularEntry { value: 1.0, from_unit: "stick_butter", to_unit: "g", ingredient: Some("butter"), density: Some(0.92) },
    // Honey
    PopularEntry { value: 1.0, from_unit: "tbsp", to_unit: "g", ingredient: Some("honey"),  density: Some(1.42) },
    PopularEntry { value: 1.0, from_unit: "cup",  to_unit: "g", ingredient: Some("honey"),  density: Some(1.42) },
    // Rice
    PopularEntry { value: 1.0, from_unit: "cup",  to_unit: "g", ingredient: Some("rice"),   density: Some(0.77) },
    // Milk
    PopularEntry { value: 1.0, from_unit: "cup",  to_unit: "g", ingredient: Some("milk"),   density: Some(1.03) },
    PopularEntry { value: 1.0, from_unit: "cup",  to_unit: "ml", ingredient: None, density: None },
    // Pure unit conversions
    PopularEntry { value: 1.0, from_unit: "lb",     to_unit: "g",   ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "oz",     to_unit: "g",   ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "gallon", to_unit: "l",   ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "pint",   to_unit: "ml",  ingredient: None, density: None },
    PopularEntry { value: 1.0, from_unit: "quart",  to_unit: "ml",  ingredient: None, density: None },
];

#[derive(Deserialize)]
pub struct PopularQuery {
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct PopularConversion {
    pub value: f64,
    pub from_unit: String,
    pub from_label: String,
    pub to_unit: String,
    pub to_label: String,
    pub result: f64,
    pub ingredient: Option<String>,
    pub localized_name: Option<String>,
    pub slug: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Serialize)]
pub struct PopularResponse {
    pub conversions: Vec<PopularConversion>,
}

/// GET /public/tools/popular-conversions?lang=ru
///
/// Returns a curated list of the most-searched cooking conversions (great for SEO).
/// Each ingredient-based conversion is enriched with localized name, slug & image from DB.
pub async fn popular_conversions(
    State(pool): State<PgPool>,
    Query(params): Query<PopularQuery>,
) -> Json<PopularResponse> {
    let lang = parse_lang(&params.lang);

    // Preload all ingredient rows we need
    let ingredient_names: Vec<&str> = POPULAR_CONVERSIONS.iter()
        .filter_map(|e| e.ingredient)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let db_rows: Vec<CatalogNutritionRow> = if !ingredient_names.is_empty() {
        sqlx::query_as(
            r#"SELECT name_en, name_ru, name_pl, name_uk, image_url, slug,
                      calories_per_100g, protein_per_100g, fat_per_100g,
                      carbs_per_100g, density_g_per_ml
               FROM catalog_ingredients
               WHERE is_active = true"#,
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

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
                value: e.value,
                from_unit: e.from_unit.to_string(),
                from_label: label(e.from_unit, lang),
                to_unit: e.to_unit.to_string(),
                to_label: label(e.to_unit, lang),
                result: uc::display_round(r),
                ingredient: e.ingredient.map(|s| s.to_string()),
                localized_name: db.map(|row| row.localized_name(lang).to_string()),
                slug: db.and_then(|row| row.slug.clone()),
                image_url: db.and_then(|row| row.image_url.clone()),
            }
        })
    }).collect();

    Json(PopularResponse { conversions })
}

// ── 11. Ingredient scale ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngredientScaleQuery {
    pub ingredient: Option<String>,
    pub value: f64,
    pub unit: Option<String>,
    pub from_portions: f64,
    pub to_portions: f64,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct IngredientScaleResponse {
    pub ingredient: Option<String>,
    pub original_value: f64,
    pub unit: String,
    pub from_portions: f64,
    pub to_portions: f64,
    pub scaled_value: f64,
    pub smart_result: Option<SmartUnit>,
}

/// GET /public/tools/ingredient-scale?ingredient=flour&value=200&unit=g&from_portions=4&to_portions=10&lang=ru
///
/// Scales an ingredient amount between portion sizes.
/// Returns smart_result with auto-optimised unit (e.g. 2500g → 2.5 kg).
pub async fn ingredient_scale(Query(params): Query<IngredientScaleQuery>) -> Json<IngredientScaleResponse> {
    let lang = parse_lang(&params.lang);
    let unit = params.unit.as_deref().unwrap_or("g");
    let scaled = uc::display_round(uc::scale(params.value, params.from_portions, params.to_portions));

    // Smart auto-unit
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
        ingredient: params.ingredient,
        original_value: params.value,
        unit: unit.to_string(),
        from_portions: params.from_portions,
        to_portions: params.to_portions,
        scaled_value: scaled,
        smart_result,
    })
}

// ── 12. Categories ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ToolInfo {
    pub id: &'static str,
    pub path: &'static str,
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
            ToolInfo { id: "converter",              path: "/public/tools/convert",                description: "Universal unit converter (mass & volume)" },
            ToolInfo { id: "units",                   path: "/public/tools/units",                  description: "List all supported units with labels" },
            ToolInfo { id: "nutrition",               path: "/public/tools/nutrition",               description: "Nutrition calculator (supports any unit)" },
            ToolInfo { id: "fish-season",             path: "/public/tools/fish-season",             description: "Fish seasonality calendar (single fish)" },
            ToolInfo { id: "fish-season-table",       path: "/public/tools/fish-season-table",       description: "Full fish seasonality table with catalog data (name, image)" },
            ToolInfo { id: "scale",                   path: "/public/tools/scale",                   description: "Recipe portion scaler" },
            ToolInfo { id: "yield",                   path: "/public/tools/yield",                   description: "Cooking yield & waste calculator" },
            ToolInfo { id: "ingredient-equivalents",  path: "/public/tools/ingredient-equivalents",  description: "Convert ingredient to all units via density" },
            ToolInfo { id: "food-cost",               path: "/public/tools/food-cost",               description: "Food cost, margin & markup calculator" },
            ToolInfo { id: "ingredient-suggestions",  path: "/public/tools/ingredient-suggestions",  description: "Suggest ingredients by volume unit with grams" },
            ToolInfo { id: "popular-conversions",    path: "/public/tools/popular-conversions",    description: "Curated popular cooking conversions (SEO)" },
            ToolInfo { id: "ingredient-scale",       path: "/public/tools/ingredient-scale",       description: "Scale an ingredient between portion sizes" },
            ToolInfo { id: "measure-conversion",     path: "/public/tools/measure-conversion",     description: "SEO: how many grams in a cup/tbsp/tsp of an ingredient" },
            ToolInfo { id: "ingredient-measures",    path: "/public/tools/ingredient-measures",    description: "SEO: full cup/tbsp/tsp grams table for an ingredient" },
        ],
    })
}

// ── 13. Measure conversion (SEO) ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MeasureConversionQuery {
    pub ingredient: String,
    pub from: String,
    pub to: String,
    pub lang: Option<String>,
    pub value: Option<f64>,
}

#[derive(Serialize)]
pub struct MeasureConversionResponse {
    pub ingredient: String,
    pub ingredient_name: String,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub value: f64,
    pub from: String,
    pub from_label: String,
    pub to: String,
    pub to_label: String,
    pub result: f64,
    pub question: String,
    pub answer: String,
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

fn ml_for_unit(unit: &str) -> Option<f64> {
    match unit.to_lowercase().as_str() {
        "cup" | "cups" => Some(uc::CUP_ML),
        "tbsp" | "tablespoon" | "tablespoons" => Some(uc::TBSP_ML),
        "tsp" | "teaspoon" | "teaspoons" => Some(uc::TSP_ML),
        _ => None,
    }
}

fn unit_display_label(unit: &str, lang: Language) -> String {
    match (unit.to_lowercase().as_str(), lang) {
        ("cup" | "cups", Language::Pl) => "szklanka".to_string(),
        ("cup" | "cups", Language::Ru) => "стакан".to_string(),
        ("cup" | "cups", Language::Uk) => "склянка".to_string(),
        ("cup" | "cups", _) => "cup".to_string(),
        ("tbsp" | "tablespoon" | "tablespoons", Language::Pl) => "łyżka stołowa".to_string(),
        ("tbsp" | "tablespoon" | "tablespoons", Language::Ru) => "столовая ложка".to_string(),
        ("tbsp" | "tablespoon" | "tablespoons", Language::Uk) => "столова ложка".to_string(),
        ("tbsp" | "tablespoon" | "tablespoons", _) => "tbsp".to_string(),
        ("tsp" | "teaspoon" | "teaspoons", Language::Pl) => "łyżeczka".to_string(),
        ("tsp" | "teaspoon" | "teaspoons", Language::Ru) => "чайная ложка".to_string(),
        ("tsp" | "teaspoon" | "teaspoons", Language::Uk) => "чайна ложка".to_string(),
        ("tsp" | "teaspoon" | "teaspoons", _) => "tsp".to_string(),
        ("g" | "grams" | "gram", Language::Pl) => "gram".to_string(),
        ("g" | "grams" | "gram", Language::Ru) => "грамм".to_string(),
        ("g" | "grams" | "gram", Language::Uk) => "грам".to_string(),
        ("g" | "grams" | "gram", _) => "g".to_string(),
        _ => unit.to_string(),
    }
}

/// GET /public/tools/measure-conversion?ingredient=flour&from=cup&to=g&lang=en&value=1
pub async fn measure_conversion(
    State(pool): State<PgPool>,
    Query(params): Query<MeasureConversionQuery>,
) -> Json<MeasureConversionResponse> {
    let lang = parse_lang(&params.lang);
    let value = params.value.unwrap_or(1.0);
    let name_lower = params.ingredient.to_lowercase();

    let db_row: Option<CatalogNutritionRow> = sqlx::query_as(
        r#"
        SELECT name_en, name_ru, name_pl, name_uk, image_url, slug,
               calories_per_100g, protein_per_100g, fat_per_100g,
               carbs_per_100g, density_g_per_ml
        FROM catalog_ingredients
        WHERE is_active = true
          AND (LOWER(name_en) = $1
               OR slug = $1
               OR LOWER(name_ru) = $1
               OR LOWER(name_pl) = $1
               OR LOWER(name_uk) = $1
               OR LOWER(name_en) LIKE '%' || $1 || '%')
        ORDER BY (LOWER(name_en) = $1 OR slug = $1) DESC, LENGTH(name_en) ASC
        LIMIT 1
        "#,
    )
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

    // Only cup/tbsp/tsp → g supported (to=g always meaningful; from=volume unit)
    let result = if let Some(ml) = ml_for_unit(&params.from) {
        uc::round_to(value * uc::grams_from_volume(density, ml), 2)
    } else if params.from.to_lowercase() == "g" {
        // g → volume: inverse, but answer still in grams
        uc::round_to(value, 2)
    } else {
        0.0
    };

    let from_label = unit_display_label(&params.from, lang);
    let to_label = unit_display_label(&params.to, lang);
    let question = measure_question(&from_label, &ingredient_name, lang);
    let answer = measure_answer(value, &from_label, &ingredient_name, result, lang);

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

// ── 14. Ingredient measures table (SEO) ──────────────────────────────────────

#[derive(Deserialize)]
pub struct IngredientMeasuresQuery {
    pub ingredient: String,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct MeasureEntry {
    pub unit: String,
    pub unit_label: String,
    pub grams: f64,
}

#[derive(Serialize)]
pub struct IngredientMeasuresResponse {
    pub ingredient: String,
    pub ingredient_name: String,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub density_g_per_ml: Option<f64>,
    pub measures: Vec<MeasureEntry>,
}

/// GET /public/tools/ingredient-measures?ingredient=flour&lang=en
pub async fn ingredient_measures(
    State(pool): State<PgPool>,
    Query(params): Query<IngredientMeasuresQuery>,
) -> Json<IngredientMeasuresResponse> {
    let lang = parse_lang(&params.lang);
    let name_lower = params.ingredient.to_lowercase();

    let db_row: Option<CatalogNutritionRow> = sqlx::query_as(
        r#"
        SELECT name_en, name_ru, name_pl, name_uk, image_url, slug,
               calories_per_100g, protein_per_100g, fat_per_100g,
               carbs_per_100g, density_g_per_ml
        FROM catalog_ingredients
        WHERE is_active = true
          AND (LOWER(name_en) = $1
               OR slug = $1
               OR LOWER(name_ru) = $1
               OR LOWER(name_pl) = $1
               OR LOWER(name_uk) = $1
               OR LOWER(name_en) LIKE '%' || $1 || '%')
        ORDER BY (LOWER(name_en) = $1 OR slug = $1) DESC, LENGTH(name_en) ASC
        LIMIT 1
        "#,
    )
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

    let units = [
        ("cup", uc::CUP_ML),
        ("tbsp", uc::TBSP_ML),
        ("tsp", uc::TSP_ML),
    ];

    let measures = units.iter().map(|(unit, ml)| MeasureEntry {
        unit: unit.to_string(),
        unit_label: unit_display_label(unit, lang),
        grams: uc::round_to(uc::grams_from_volume(density, *ml), 2),
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

// ── 8. Seasonal Calendar (fully DB-driven, universal) ─────────────────────────

#[derive(Deserialize)]
pub struct SeasonalCalendarQuery {
    pub r#type: Option<String>,   // seafood | vegetable | fruit | meat | ...
    pub lang:   Option<String>,
    pub region: Option<String>,   // default: GLOBAL
}

#[derive(Serialize, sqlx::FromRow)]
pub struct SeasonalCalendarProduct {
    pub slug:      String,
    pub name:      String,
    pub image_url: Option<String>,
    pub season:    Vec<FishSeasonEntry>,
}

#[derive(Serialize)]
pub struct SeasonalCalendarResponse {
    pub product_type: String,
    pub lang:         String,
    pub region:       String,
    pub products:     Vec<SeasonalCalendarProduct>,
}

#[derive(sqlx::FromRow)]
struct SeasonalProductRow {
    slug:      Option<String>,
    name_en:   String,
    name_ru:   String,
    name_pl:   String,
    name_uk:   String,
    image_url: Option<String>,
}

#[derive(sqlx::FromRow)]
struct SeasonalityRow {
    product_id: uuid::Uuid,
    month:      i16,
    status:     String,
}

/// GET /public/tools/seasonal-calendar?type=seafood&lang=ru&region=GLOBAL
pub async fn seasonal_calendar(
    State(pool): State<PgPool>,
    Query(params): Query<SeasonalCalendarQuery>,
) -> Json<SeasonalCalendarResponse> {
    let lang        = parse_lang(&params.lang);
    let product_type = params.r#type.clone().unwrap_or_else(|| "seafood".to_string());
    let region      = params.region.clone().unwrap_or_else(|| "GLOBAL".to_string());
    let lang_code   = match lang {
        Language::Ru => "ru", Language::Pl => "pl",
        Language::Uk => "uk", Language::En => "en",
    };

    // Fetch all products of given type that have seasonality data
    let products: Vec<SeasonalProductRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk, ci.image_url
        FROM catalog_ingredients ci
        JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
        WHERE ci.is_active = true
          AND ci.product_type = $1
          AND cps.region_code = $2
        ORDER BY ci.name_en
        "#,
    )
    .bind(&product_type)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    // Fetch all seasonality rows for these products at once
    let seasonality: Vec<SeasonalityRow> = sqlx::query_as(
        r#"
        SELECT cps.product_id, cps.month, cps.status
        FROM catalog_product_seasonality cps
        JOIN catalog_ingredients ci ON ci.id = cps.product_id
        WHERE ci.is_active = true
          AND ci.product_type = $1
          AND cps.region_code = $2
        ORDER BY cps.product_id, cps.month
        "#,
    )
    .bind(&product_type)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    // Also fetch product IDs to map slug → id
    let product_ids: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        r#"
        SELECT DISTINCT ci.id, ci.slug
        FROM catalog_ingredients ci
        JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
        WHERE ci.is_active = true AND ci.product_type = $1 AND cps.region_code = $2
        "#,
    )
    .bind(&product_type)
    .bind(&region)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let result = products.iter().map(|p| {
        let slug = p.slug.clone().unwrap_or_default();

        // Find product id
        let product_id = product_ids.iter()
            .find(|(_, s)| s.as_str() == slug.as_str())
            .map(|(id, _)| *id);

        let name = match lang {
            Language::Ru => p.name_ru.clone(),
            Language::Pl => p.name_pl.clone(),
            Language::Uk => p.name_uk.clone(),
            Language::En => p.name_en.clone(),
        };

        let season = (1u8..=12).map(|m| {
            let status = product_id
                .and_then(|pid| seasonality.iter()
                    .find(|r| r.product_id == pid && r.month == m as i16)
                    .map(|r| r.status.clone()))
                .unwrap_or_else(|| "off".to_string());

            FishSeasonEntry {
                month: m,
                month_name: month_name(m, lang).to_string(),
                available: status != "off",
            }
        }).collect();

        SeasonalCalendarProduct {
            slug,
            name,
            image_url: p.image_url.clone(),
            season,
        }
    }).collect();

    Json(SeasonalCalendarResponse {
        product_type,
        lang: lang_code.to_string(),
        region,
        products: result,
    })
}

// ── 9. In-season-now ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct InSeasonItem {
    pub slug:      String,
    pub name:      String,
    pub image_url: Option<String>,
    pub status:    String,
}

#[derive(Serialize)]
pub struct InSeasonNowResponse {
    pub product_type: String,
    pub month:        u8,
    pub lang:         String,
    pub region:       String,
    pub items:        Vec<InSeasonItem>,
}

/// GET /public/tools/in-season-now?type=seafood&lang=ru
pub async fn in_season_now(
    State(pool): State<PgPool>,
    Query(params): Query<SeasonalCalendarQuery>,
) -> Json<InSeasonNowResponse> {
    let lang         = parse_lang(&params.lang);
    let product_type = params.r#type.clone().unwrap_or_else(|| "seafood".to_string());
    let region       = params.region.clone().unwrap_or_else(|| "GLOBAL".to_string());
    let lang_code    = match lang {
        Language::Ru => "ru", Language::Pl => "pl",
        Language::Uk => "uk", Language::En => "en",
    };

    // Current month (UTC)
    let now   = time::OffsetDateTime::now_utc();
    let month = now.month() as u8;

    #[derive(sqlx::FromRow)]
    struct InSeasonRow {
        slug:      Option<String>,
        name_en:   String,
        name_ru:   String,
        name_pl:   String,
        name_uk:   String,
        image_url: Option<String>,
        status:    String,
    }

    let rows: Vec<InSeasonRow> = sqlx::query_as(
        r#"
        SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk, ci.image_url,
               cps.status
        FROM catalog_ingredients ci
        JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
        WHERE ci.is_active = true
          AND ci.product_type = $1
          AND cps.region_code = $2
          AND cps.month = $3
          AND cps.status != 'off'
        ORDER BY
            CASE cps.status
                WHEN 'peak'    THEN 1
                WHEN 'good'    THEN 2
                WHEN 'limited' THEN 3
                ELSE 4
            END,
            ci.name_en
        "#,
    )
    .bind(&product_type)
    .bind(&region)
    .bind(month as i16)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let items = rows.iter().map(|r| {
        let name = match lang {
            Language::Ru => r.name_ru.clone(),
            Language::Pl => r.name_pl.clone(),
            Language::Uk => r.name_uk.clone(),
            Language::En => r.name_en.clone(),
        };
        InSeasonItem {
            slug:      r.slug.clone().unwrap_or_default(),
            name,
            image_url: r.image_url.clone(),
            status:    r.status.clone(),
        }
    }).collect();

    Json(InSeasonNowResponse {
        product_type,
        month,
        lang: lang_code.to_string(),
        region,
        items,
    })
}

// ── 10. Product seasonality detail ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProductSeasonalityQuery {
    pub slug:   String,
    pub lang:   Option<String>,
    pub region: Option<String>,
}

#[derive(Serialize)]
pub struct ProductSeasonalityEntry {
    pub month:      u8,
    pub month_name: String,
    pub status:     String,   // peak | good | limited | off
    pub available:  bool,
    pub note:       Option<String>,
}

#[derive(Serialize)]
pub struct ProductSeasonalityResponse {
    pub slug:         String,
    pub name:         String,
    pub product_type: String,
    pub image_url:    Option<String>,
    pub region:       String,
    pub lang:         String,
    pub season:       Vec<ProductSeasonalityEntry>,
}

/// GET /public/tools/product-seasonality?slug=salmon&lang=ru
pub async fn product_seasonality(
    State(pool): State<PgPool>,
    Query(params): Query<ProductSeasonalityQuery>,
) -> Json<ProductSeasonalityResponse> {
    let lang   = parse_lang(&params.lang);
    let region = params.region.clone().unwrap_or_else(|| "GLOBAL".to_string());
    let lang_code = match lang {
        Language::Ru => "ru", Language::Pl => "pl",
        Language::Uk => "uk", Language::En => "en",
    };

    #[derive(sqlx::FromRow)]
    struct ProdRow {
        id:           uuid::Uuid,
        name_en:      String,
        name_ru:      String,
        name_pl:      String,
        name_uk:      String,
        image_url:    Option<String>,
        product_type: Option<String>,
    }

    let prod: Option<ProdRow> = sqlx::query_as(
        r#"SELECT id, name_en, name_ru, name_pl, name_uk, image_url, product_type
           FROM catalog_ingredients
           WHERE is_active = true AND slug = $1"#,
    )
    .bind(&params.slug)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let (prod_id, name, product_type, image_url) = if let Some(ref p) = prod {
        let n = match lang {
            Language::Ru => p.name_ru.clone(),
            Language::Pl => p.name_pl.clone(),
            Language::Uk => p.name_uk.clone(),
            Language::En => p.name_en.clone(),
        };
        (Some(p.id), n, p.product_type.clone().unwrap_or_else(|| "other".to_string()), p.image_url.clone())
    } else {
        (None, params.slug.clone(), "other".to_string(), None)
    };

    #[derive(sqlx::FromRow)]
    struct SRow { month: i16, status: String, note: Option<String> }

    let srows: Vec<SRow> = if let Some(pid) = prod_id {
        sqlx::query_as(
            r#"SELECT month, status, note
               FROM catalog_product_seasonality
               WHERE product_id = $1 AND region_code = $2
               ORDER BY month"#,
        )
        .bind(pid)
        .bind(&region)
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    let season = (1u8..=12).map(|m| {
        let row = srows.iter().find(|r| r.month == m as i16);
        let status = row.map(|r| r.status.clone()).unwrap_or_else(|| "off".to_string());
        let note   = row.and_then(|r| r.note.clone());
        ProductSeasonalityEntry {
            month:      m,
            month_name: month_name(m, lang).to_string(),
            available:  status != "off",
            status,
            note,
        }
    }).collect();

    Json(ProductSeasonalityResponse {
        slug:         params.slug,
        name,
        product_type,
        image_url,
        region,
        lang:         lang_code.to_string(),
        season,
    })
}

// ── 11. Best in season ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BestInSeasonQuery {
    pub r#type: Option<String>,
    pub month:  Option<u8>,
    pub lang:   Option<String>,
    pub region: Option<String>,
}

#[derive(Serialize)]
pub struct BestInSeasonItem {
    pub slug:      String,
    pub name:      String,
    pub image_url: Option<String>,
    pub status:    String,
}

#[derive(Serialize)]
pub struct BestInSeasonResponse {
    pub product_type: String,
    pub month:        u8,
    pub lang:         String,
    pub region:       String,
    pub items:        Vec<BestInSeasonItem>,
}

/// GET /public/tools/best-in-season?type=seafood&month=8&lang=ru
///
/// Returns only peak-season products for a given type and month.
/// Ideal for SEO pages: "Best fish in August", "Best vegetables in July"
pub async fn best_in_season(
    State(pool): State<PgPool>,
    Query(params): Query<BestInSeasonQuery>,
) -> Json<BestInSeasonResponse> {
    let lang         = parse_lang(&params.lang);
    let product_type = params.r#type.clone().unwrap_or_else(|| "seafood".to_string());
    let region       = params.region.clone().unwrap_or_else(|| "GLOBAL".to_string());
    let lang_code    = match lang {
        Language::Ru => "ru", Language::Pl => "pl",
        Language::Uk => "uk", Language::En => "en",
    };

    // Use provided month or current UTC month
    let month: u8 = params.month.unwrap_or_else(|| {
        time::OffsetDateTime::now_utc().month() as u8
    });

    #[derive(sqlx::FromRow)]
    struct Row {
        slug:      Option<String>,
        name_en:   String,
        name_ru:   String,
        name_pl:   String,
        name_uk:   String,
        image_url: Option<String>,
        status:    String,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
               ci.image_url, cps.status
        FROM catalog_ingredients ci
        JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
        WHERE ci.is_active = true
          AND ci.product_type = $1
          AND cps.region_code = $2
          AND cps.month       = $3
          AND cps.status      = 'peak'
        ORDER BY ci.name_en
        "#,
    )
    .bind(&product_type)
    .bind(&region)
    .bind(month as i16)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let items = rows.iter().map(|r| {
        let name = match lang {
            Language::Ru => r.name_ru.clone(),
            Language::Pl => r.name_pl.clone(),
            Language::Uk => r.name_uk.clone(),
            Language::En => r.name_en.clone(),
        };
        BestInSeasonItem {
            slug:      r.slug.clone().unwrap_or_default(),
            name,
            image_url: r.image_url.clone(),
            status:    r.status.clone(),
        }
    }).collect();

    Json(BestInSeasonResponse {
        product_type,
        month,
        lang: lang_code.to_string(),
        region,
        items,
    })
}

// ── 12. Products by month ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProductsByMonthQuery {
    pub month:  Option<u8>,
    pub r#type: Option<String>,
    pub lang:   Option<String>,
    pub region: Option<String>,
}

#[derive(Serialize)]
pub struct ProductsByMonthItem {
    pub slug:      String,
    pub name:      String,
    pub image_url: Option<String>,
    pub status:    String,   // peak | good | limited
}

#[derive(Serialize)]
pub struct ProductsByMonthResponse {
    pub month:        u8,
    pub month_name:   String,
    pub product_type: Option<String>,
    pub lang:         String,
    pub region:       String,
    pub items:        Vec<ProductsByMonthItem>,
}

/// GET /public/tools/products-by-month?month=7&type=vegetable&lang=ru
///
/// Returns all in-season products for a given month (optionally filtered by type).
/// Sorted by status: peak → good → limited
/// SEO powerhouse: "What vegetables are in season in July?"
pub async fn products_by_month(
    State(pool): State<PgPool>,
    Query(params): Query<ProductsByMonthQuery>,
) -> Json<ProductsByMonthResponse> {
    let lang       = parse_lang(&params.lang);
    let region     = params.region.clone().unwrap_or_else(|| "GLOBAL".to_string());
    let lang_code  = match lang {
        Language::Ru => "ru", Language::Pl => "pl",
        Language::Uk => "uk", Language::En => "en",
    };

    let month: u8 = params.month.unwrap_or_else(|| {
        time::OffsetDateTime::now_utc().month() as u8
    });

    #[derive(sqlx::FromRow)]
    struct Row {
        slug:         Option<String>,
        name_en:      String,
        name_ru:      String,
        name_pl:      String,
        name_uk:      String,
        image_url:    Option<String>,
        status:       String,
    }

    // Build query — type filter is optional
    let rows: Vec<Row> = match &params.r#type {
        Some(ptype) => sqlx::query_as(
            r#"
            SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
                   ci.image_url, cps.status
            FROM catalog_ingredients ci
            JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
            WHERE ci.is_active = true
              AND ci.product_type = $1
              AND cps.region_code = $2
              AND cps.month       = $3
              AND cps.status     != 'off'
            ORDER BY
                CASE cps.status
                    WHEN 'peak'    THEN 1
                    WHEN 'good'    THEN 2
                    WHEN 'limited' THEN 3
                    ELSE 4
                END,
                ci.name_en
            "#,
        )
        .bind(ptype)
        .bind(&region)
        .bind(month as i16)
        .fetch_all(&pool)
        .await
        .unwrap_or_default(),

        None => sqlx::query_as(
            r#"
            SELECT ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
                   ci.image_url, cps.status
            FROM catalog_ingredients ci
            JOIN catalog_product_seasonality cps ON cps.product_id = ci.id
            WHERE ci.is_active = true
              AND cps.region_code = $1
              AND cps.month       = $2
              AND cps.status     != 'off'
            ORDER BY
                CASE cps.status
                    WHEN 'peak'    THEN 1
                    WHEN 'good'    THEN 2
                    WHEN 'limited' THEN 3
                    ELSE 4
                END,
                ci.product_type, ci.name_en
            "#,
        )
        .bind(&region)
        .bind(month as i16)
        .fetch_all(&pool)
        .await
        .unwrap_or_default(),
    };

    let items = rows.iter().map(|r| {
        let name = match lang {
            Language::Ru => r.name_ru.clone(),
            Language::Pl => r.name_pl.clone(),
            Language::Uk => r.name_uk.clone(),
            Language::En => r.name_en.clone(),
        };
        ProductsByMonthItem {
            slug:      r.slug.clone().unwrap_or_default(),
            name,
            image_url: r.image_url.clone(),
            status:    r.status.clone(),
        }
    }).collect();

    Json(ProductsByMonthResponse {
        month,
        month_name:   month_name(month, lang).to_string(),
        product_type: params.r#type,
        lang:         lang_code.to_string(),
        region,
        items,
    })
}
