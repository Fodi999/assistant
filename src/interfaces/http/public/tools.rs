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
    // ── New: extended density table ──
    ("cocoa powder",   228.0,  19.6,  13.7,  57.9, 0.72),
    ("maple syrup",    260.0,   0.0,   0.1,  67.0, 1.32),
    ("corn starch",    381.0,   0.3,   0.1,  91.3, 0.56),
    ("brown sugar",    380.0,   0.1,   0.0,  98.0, 0.83),
    ("powdered sugar", 389.0,   0.0,   0.0, 100.0, 0.56),
    ("baking powder",   53.0,   0.0,   0.0,  28.0, 0.90),
    ("baking soda",      0.0,   0.0,   0.0,   0.0, 1.10),
    ("vanilla extract", 288.0,  0.1,   0.1,  12.7, 1.04),
    ("soy sauce",       53.0,   8.1,   0.6,   4.9, 1.08),
    ("vinegar",         21.0,   0.0,   0.0,   0.9, 1.01),
    ("ketchup",        112.0,   1.7,   0.1,  27.4, 1.15),
    ("mayonnaise",     680.0,   1.0,  75.0,   0.6, 0.91),
    ("mustard",         66.0,   4.4,   4.0,   5.8, 1.05),
    ("peanut butter",  588.0,  25.0,  50.0,  20.0, 1.09),
    ("oats",           389.0,  16.9,   6.9,  66.3, 0.43),
    ("breadcrumbs",    395.0,  13.4,   5.3,  71.0, 0.55),
    ("parmesan",       431.0,  38.5,  29.0,   4.1, 1.20),
    ("cinnamon",       247.0,   4.0,   1.2,  80.6, 0.56),
    ("black pepper",   251.0,  10.4,   3.3,  63.9, 0.50),
    ("paprika",        282.0,  14.1,  13.0,  53.9, 0.50),
    ("cumin",          375.0,  17.8,  22.3,  44.2, 0.45),
    ("turmeric",       354.0,   7.8,   9.9,  64.9, 0.67),
    ("ginger",          80.0,   1.8,   0.8,  17.8, 0.56),
    ("sesame oil",     884.0,   0.0, 100.0,   0.0, 0.92),
    ("lard",           902.0,   0.0, 100.0,   0.0, 0.92),
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
/// Given a volume unit, returns common ingredients with their weight per that unit.
pub async fn ingredient_suggestions(Query(params): Query<SuggestionsQuery>) -> Json<SuggestionsResponse> {
    let _lang = parse_lang(&params.lang);
    let ml_factor = uc::volume_to_ml(&params.unit);

    let suggestions: Vec<Suggestion> = if let Some(ml) = ml_factor {
        NUTRITION_TABLE
            .iter()
            .filter(|&&(_, _, _, _, _, d)| d > 0.0 && d != 1.0) // skip water-like
            .map(|&(name, _, _, _, _, density)| {
                let grams = uc::display_round(ml * density);
                Suggestion {
                    name: name.to_string(),
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
}

#[derive(Serialize)]
pub struct PopularResponse {
    pub conversions: Vec<PopularConversion>,
}

/// GET /public/tools/popular-conversions?lang=ru
///
/// Returns a curated list of the most-searched cooking conversions (great for SEO).
pub async fn popular_conversions(Query(params): Query<PopularQuery>) -> Json<PopularResponse> {
    let lang = parse_lang(&params.lang);

    let conversions = POPULAR_CONVERSIONS.iter().filter_map(|e| {
        let result = match (e.ingredient, e.density) {
            (Some(_), Some(d)) => uc::convert_with_density(e.value, e.from_unit, e.to_unit, d),
            _ => uc::convert_units(e.value, e.from_unit, e.to_unit),
        };
        result.map(|r| PopularConversion {
            value: e.value,
            from_unit: e.from_unit.to_string(),
            from_label: label(e.from_unit, lang),
            to_unit: e.to_unit.to_string(),
            to_label: label(e.to_unit, lang),
            result: uc::display_round(r),
            ingredient: e.ingredient.map(|s| s.to_string()),
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
            ToolInfo { id: "fish-season",             path: "/public/tools/fish-season",             description: "Fish seasonality calendar" },
            ToolInfo { id: "scale",                   path: "/public/tools/scale",                   description: "Recipe portion scaler" },
            ToolInfo { id: "yield",                   path: "/public/tools/yield",                   description: "Cooking yield & waste calculator" },
            ToolInfo { id: "ingredient-equivalents",  path: "/public/tools/ingredient-equivalents",  description: "Convert ingredient to all units via density" },
            ToolInfo { id: "food-cost",               path: "/public/tools/food-cost",               description: "Food cost, margin & markup calculator" },
            ToolInfo { id: "ingredient-suggestions",  path: "/public/tools/ingredient-suggestions",  description: "Suggest ingredients by volume unit with grams" },
            ToolInfo { id: "popular-conversions",    path: "/public/tools/popular-conversions",    description: "Curated popular cooking conversions (SEO)" },
            ToolInfo { id: "ingredient-scale",       path: "/public/tools/ingredient-scale",       description: "Scale an ingredient between portion sizes" },
        ],
    })
}
