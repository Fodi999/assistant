use crate::domain::tools::unit_converter as uc;
use crate::shared::Language;
use axum::{extract::Query, response::Json};
use serde::{Deserialize, Serialize};

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

// ── 3. Fish season ───────────────────────────────────────────────────────────

struct FishData {
    name:   &'static str,
    months: [bool; 12],
}

static FISH_TABLE: &[FishData] = &[
    FishData { name: "salmon", months: [true,  true,  true,  false, false, false, true,  true,  true,  true,  true,  true ] },
    FishData { name: "tuna",   months: [false, false, false, true,  true,  true,  true,  true,  true,  false, false, false] },
    FishData { name: "cod",    months: [true,  true,  true,  true,  false, false, false, false, false, true,  true,  true ] },
];

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
pub async fn fish_season(Query(params): Query<FishQuery>) -> Json<FishSeasonResponse> {
    let lang = parse_lang(&params.lang);
    let fish_lower = params.fish.to_lowercase();
    let availability = FISH_TABLE.iter().find(|f| f.name == fish_lower)
        .map(|f| &f.months).unwrap_or(&[true; 12]);
    let season = (1u8..=12).map(|m| FishSeasonEntry {
        month: m, month_name: month_name(m, lang).to_string(),
        available: availability[(m - 1) as usize],
    }).collect();
    Json(FishSeasonResponse { fish: params.fish, season })
}

// ── 4. Nutrition (static, no DB) ─────────────────────────────────────────────

static NUTRITION_TABLE: &[(&str, f64, f64, f64, f64, f64)] = &[
    //  name              cal   prot   fat   carbs  density_g_per_ml
    ("salmon",         208.0, 20.0,  13.0,   0.0,  1.05),
    ("chicken breast", 165.0, 31.0,   3.6,   0.0,  1.04),
    ("beef",           250.0, 26.0,  15.0,   0.0,  1.05),
    ("egg",            155.0, 13.0,  11.0,   1.1,  0.95),
    ("potato",          77.0,  2.0,   0.1,  17.0,  0.77),
    ("rice",           130.0,  2.7,   0.3,  28.0,  0.77),
    ("pasta",          371.0, 13.0,   1.5,  74.0,  0.56),
    ("butter",         717.0,  0.9,  81.0,   0.1,  0.92),
    ("olive oil",      884.0,  0.0, 100.0,   0.0,  0.91),
    ("milk",            42.0,  3.4,   1.0,   5.0,  1.03),
    ("cheese",         402.0, 25.0,  33.0,   1.3,  1.09),
    ("tomato",          18.0,  0.9,   0.2,   3.9,  0.95),
    ("onion",           40.0,  1.1,   0.1,   9.3,  0.74),
    ("garlic",         149.0,  6.4,   0.5,  33.0,  0.72),
    ("carrot",          41.0,  0.9,   0.2,  10.0,  0.64),
    ("broccoli",        34.0,  2.8,   0.4,   7.0,  0.40),
    ("spinach",         23.0,  2.9,   0.4,   3.6,  0.25),
    ("lemon",           29.0,  1.1,   0.3,   9.3,  1.00),
    ("avocado",        160.0,  2.0,  15.0,   9.0,  0.96),
    ("walnuts",        654.0, 15.0,  65.0,  14.0,  0.52),
    ("almonds",        579.0, 21.0,  50.0,  22.0,  0.55),
    ("flour",          364.0, 10.0,   1.0,  76.0,  0.53),
    ("sugar",          387.0,  0.0,   0.0, 100.0,  0.85),
    ("honey",          304.0,  0.3,   0.0,  82.0,  1.42),
    ("cream",          340.0,  2.1,  37.0,   2.8,  1.01),
    ("sour cream",     193.0,  2.4,  19.0,   3.4,  1.01),
    ("yogurt",          59.0, 10.0,   0.7,   3.6,  1.05),
    ("coconut oil",    862.0,  0.0, 100.0,   0.0,  0.92),
    ("water",            0.0,  0.0,   0.0,   0.0,  1.00),
    ("salt",             0.0,  0.0,   0.0,   0.0,  1.26),
];

fn find_nutrition(name: &str) -> Option<&'static (&'static str, f64, f64, f64, f64, f64)> {
    NUTRITION_TABLE.iter().find(|(n, ..)| *n == name)
}

#[derive(Deserialize)]
pub struct NutritionQuery {
    #[serde(default = "default_fish")]
    pub name: String,
    /// Amount value (default 100)
    pub amount: Option<f64>,
    /// Unit of the amount (default "g")
    pub unit: Option<String>,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct NutritionResponse {
    pub name: String,
    pub amount_g: f64,
    pub calories: f64,
    pub protein_g: f64,
    pub fat_g: f64,
    pub carbs_g: f64,
    pub unit_label: String,
}

/// GET /public/tools/nutrition?name=salmon&amount=150&unit=g&lang=pl
///
/// Now supports any unit: `1 cup rice` → auto-converts to grams via density.
pub async fn nutrition(Query(params): Query<NutritionQuery>) -> Json<NutritionResponse> {
    let name_lower = params.name.to_lowercase();
    let entry = find_nutrition(&name_lower);
    let (cal, prot, fat, carbs, density) = entry
        .map(|&(_, c, p, f, cb, d)| (c, p, f, cb, d))
        .unwrap_or((0.0, 0.0, 0.0, 0.0, 1.0));

    let lang = parse_lang(&params.lang);
    let raw_amount = params.amount.unwrap_or(100.0);
    let unit = params.unit.as_deref().unwrap_or("g");

    // Convert to grams
    let amount_g = if unit == "g" {
        raw_amount
    } else if let Some(g) = uc::mass_to_grams(unit) {
        raw_amount * g
    } else if let Some(ml_factor) = uc::volume_to_ml(unit) {
        raw_amount * ml_factor * density
    } else {
        raw_amount // fallback: assume grams
    };

    let f = amount_g / 100.0;
    let r = |x: f64| uc::round_to(x, 1);

    Json(NutritionResponse {
        name: params.name,
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
pub async fn ingredient_equivalents(Query(params): Query<EquivalentsQuery>) -> Json<EquivalentsResponse> {
    let lang = parse_lang(&params.lang);
    let name_lower = params.name.to_lowercase();
    let density = find_nutrition(&name_lower)
        .map(|&(_, _, _, _, _, d)| d)
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
    /// Price per 1 unit (e.g. per kg)
    pub price: f64,
    /// Amount used
    pub amount: f64,
    /// Unit of amount (default "kg")
    pub unit: Option<String>,
    /// Number of portions this produces
    pub portions: Option<f64>,
    /// Menu sell price per portion
    pub sell_price: Option<f64>,
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct FoodCostResponse {
    pub price_per_unit: f64,
    pub amount: f64,
    pub unit: String,
    pub total_cost: f64,
    pub cost_per_portion: Option<f64>,
    pub sell_price: Option<f64>,
    pub margin_percent: Option<f64>,
    pub markup_percent: Option<f64>,
}

/// GET /public/tools/food-cost?price=12.50&amount=0.5&unit=kg&portions=4&sell_price=8.00
pub async fn food_cost_calc(Query(params): Query<FoodCostQuery>) -> Json<FoodCostResponse> {
    let unit = params.unit.as_deref().unwrap_or("kg");

    // Normalize amount to base unit (g or ml) then back to price-unit to get total cost
    // price is assumed to be per 1 of the given unit, so total = price * amount
    let total_cost = uc::round_to(uc::food_cost(params.price, params.amount), 2);

    let cost_per_portion = params.portions.map(|p| uc::round_to(uc::cost_per_portion(total_cost, p), 2));

    let margin_percent = match (params.sell_price, cost_per_portion) {
        (Some(sp), Some(cpp)) => Some(uc::round_to(uc::margin_percent(sp, cpp), 1)),
        _ => None,
    };

    let markup_percent = match (params.sell_price, cost_per_portion) {
        (Some(sp), Some(cpp)) if cpp > 0.0 => Some(uc::round_to(((sp - cpp) / cpp) * 100.0, 1)),
        _ => None,
    };

    Json(FoodCostResponse {
        price_per_unit: params.price,
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
    pub density: f64,
    pub grams_per_unit: f64,
}

#[derive(Serialize)]
pub struct SuggestionsResponse {
    pub unit: String,
    pub ml_per_unit: Option<f64>,
    pub suggestions: Vec<Suggestion>,
}

/// GET /public/tools/ingredient-suggestions?unit=cup&lang=ru
///
/// Given a volume unit, returns common ingredients with their weight per that unit.
pub async fn ingredient_suggestions(Query(params): Query<SuggestionsQuery>) -> Json<SuggestionsResponse> {
    let _lang = parse_lang(&params.lang);
    let ml_factor = uc::volume_to_ml(&params.unit);

    let suggestions: Vec<Suggestion> = if let Some(ml) = ml_factor {
        NUTRITION_TABLE
            .iter()
            .filter(|&&(_, _, _, _, _, d)| d > 0.0 && d != 1.0) // skip water-like
            .map(|&(name, _, _, _, _, density)| {
                let grams = uc::round_to(ml * density, 1);
                Suggestion {
                    name: name.to_string(),
                    density,
                    grams_per_unit: grams,
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

// ── 10. Categories ───────────────────────────────────────────────────────────

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
            ToolInfo { id: "fish-season",             path: "/public/tools/fish-season",             description: "Fish seasonality calendar" },
            ToolInfo { id: "scale",                   path: "/public/tools/scale",                   description: "Recipe portion scaler" },
            ToolInfo { id: "yield",                   path: "/public/tools/yield",                   description: "Cooking yield & waste calculator" },
            ToolInfo { id: "ingredient-equivalents",  path: "/public/tools/ingredient-equivalents",  description: "Convert ingredient to all units via density" },
            ToolInfo { id: "food-cost",               path: "/public/tools/food-cost",               description: "Food cost, margin & markup calculator" },
            ToolInfo { id: "ingredient-suggestions",  path: "/public/tools/ingredient-suggestions",  description: "Suggest ingredients by volume unit with grams" },
        ],
    })
}
