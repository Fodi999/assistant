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
    ("g",     UnitLabel { en: "gram",         pl: "gram",        ru: "грамм",           uk: "грам"          }),
    ("mg",    UnitLabel { en: "milligram",     pl: "miligram",    ru: "миллиграмм",      uk: "міліграм"      }),
    ("kg",    UnitLabel { en: "kilogram",      pl: "kilogram",    ru: "килограмм",       uk: "кілограм"      }),
    ("oz",    UnitLabel { en: "ounce",         pl: "uncja",       ru: "унция",           uk: "унція"         }),
    ("lb",    UnitLabel { en: "pound",         pl: "funt",        ru: "фунт",            uk: "фунт"          }),
    // Volume
    ("ml",    UnitLabel { en: "milliliter",    pl: "mililitr",    ru: "миллилитр",       uk: "мілілітр"      }),
    ("l",     UnitLabel { en: "liter",         pl: "litr",        ru: "литр",            uk: "літр"          }),
    ("fl_oz", UnitLabel { en: "fl. ounce",     pl: "fl. uncja",   ru: "жидк. унция",     uk: "рід. унція"    }),
    // Kitchen
    ("tsp",   UnitLabel { en: "teaspoon",      pl: "łyżeczka",    ru: "чайная ложка",    uk: "чайна ложка"   }),
    ("tbsp",  UnitLabel { en: "tablespoon",    pl: "łyżka",       ru: "столовая ложка",  uk: "столова ложка" }),
    ("cup",   UnitLabel { en: "cup",           pl: "szklanka",    ru: "стакан",          uk: "склянка"       }),
];

fn label(unit: &str, lang: Language) -> String {
    UNIT_LABELS
        .iter()
        .find(|(code, _)| *code == unit)
        .map(|(_, l)| l.for_lang(lang).to_string())
        .unwrap_or_else(|| unit.to_string())
}

// ── Unit converter ────────────────────────────────────────────────────────────

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
}

// ── Conversion tables (base unit: g for mass, ml for volume/kitchen) ─────────

fn mass_to_grams(unit: &str) -> Option<f64> {
    match unit {
        "mg" => Some(0.001),
        "g"  => Some(1.0),
        "kg" => Some(1_000.0),
        "oz" => Some(28.3495),
        "lb" => Some(453.592),
        _    => None,
    }
}

fn volume_to_ml(unit: &str) -> Option<f64> {
    match unit {
        "ml"    => Some(1.0),
        "l"     => Some(1_000.0),
        "fl_oz" => Some(29.5735),
        "cup"   => Some(236.588),
        "tbsp"  => Some(14.7868),
        "tsp"   => Some(4.92892),
        _       => None,
    }
}

fn convert(value: f64, from: &str, to: &str) -> Option<f64> {
    // Try mass
    if let (Some(f), Some(t)) = (mass_to_grams(from), mass_to_grams(to)) {
        return Some(value * f / t);
    }
    // Try volume / kitchen (same base: ml)
    if let (Some(f), Some(t)) = (volume_to_ml(from), volume_to_ml(to)) {
        return Some(value * f / t);
    }
    None
}

/// GET /public/tools/convert?value=100&from=g&to=oz&lang=ru
pub async fn convert_units(Query(params): Query<ConvertQuery>) -> Json<ConvertResponse> {
    let lang = parse_lang(&params.lang);

    // Guard: reject NaN / Infinity / values above 1 000 000
    if params.value.is_nan() || params.value.is_infinite() {
        return Json(ConvertResponse {
            value: 0.0,
            from: params.from.clone(),
            to: params.to.clone(),
            result: 0.0,
            from_label: label(&params.from, lang),
            to_label:   label(&params.to,   lang),
            supported: false,
        });
    }
    let value = params.value.min(1_000_000.0);

    let result_raw = convert(value, &params.from, &params.to);
    let supported = result_raw.is_some();
    let result = (result_raw.unwrap_or(0.0) * 1_000_000.0).round() / 1_000_000.0;

    Json(ConvertResponse {
        from_label: label(&params.from, lang),
        to_label:   label(&params.to,   lang),
        value,
        from: params.from,
        to: params.to,
        result,
        supported,
    })
}

// ── Units list ────────────────────────────────────────────────────────────────

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
        label: UNIT_LABELS
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, l)| l.for_lang(lang).to_string())
            .unwrap_or_else(|| code.to_string()),
    };

    Json(UnitsResponse {
        mass:    vec![make("g"), make("mg"), make("kg"), make("oz"), make("lb")],
        volume:  vec![make("ml"), make("l"), make("fl_oz")],
        kitchen: vec![make("tsp"), make("tbsp"), make("cup")],
    })
}

// ── Fish season ───────────────────────────────────────────────────────────────

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
pub struct FishSeasonEntry {
    pub month: u8,
    pub month_name: String,
    pub available: bool,
}

#[derive(Serialize)]
pub struct FishSeasonResponse {
    pub fish: String,
    pub season: Vec<FishSeasonEntry>,
}

fn month_name(m: u8, lang: Language) -> &'static str {
    match lang {
        Language::Ru => match m {
            1 => "Январь",   2 => "Февраль",  3 => "Март",
            4 => "Апрель",   5 => "Май",      6 => "Июнь",
            7 => "Июль",     8 => "Август",   9 => "Сентябрь",
            10 => "Октябрь", 11 => "Ноябрь",  12 => "Декабрь",
            _ => "—",
        },
        Language::Pl => match m {
            1 => "Styczeń",     2 => "Luty",       3 => "Marzec",
            4 => "Kwiecień",    5 => "Maj",         6 => "Czerwiec",
            7 => "Lipiec",      8 => "Sierpień",    9 => "Wrzesień",
            10 => "Październik", 11 => "Listopad",  12 => "Grudzień",
            _ => "—",
        },
        Language::Uk => match m {
            1 => "Січень",    2 => "Лютий",    3 => "Березень",
            4 => "Квітень",   5 => "Травень",  6 => "Червень",
            7 => "Липень",    8 => "Серпень",  9 => "Вересень",
            10 => "Жовтень",  11 => "Листопад", 12 => "Грудень",
            _ => "—",
        },
        Language::En => match m {
            1 => "January",   2 => "February",  3 => "March",
            4 => "April",     5 => "May",        6 => "June",
            7 => "July",      8 => "August",     9 => "September",
            10 => "October",  11 => "November",  12 => "December",
            _ => "—",
        },
    }
}

/// GET /public/tools/fish-season?fish=salmon&lang=ru
pub async fn fish_season(Query(params): Query<FishQuery>) -> Json<FishSeasonResponse> {
    let lang = parse_lang(&params.lang);
    let fish_lower = params.fish.to_lowercase();

    let availability: &[bool; 12] = FISH_TABLE
        .iter()
        .find(|f| f.name == fish_lower)
        .map(|f| &f.months)
        .unwrap_or(&[true; 12]);

    let season = (1u8..=12)
        .map(|m| FishSeasonEntry {
            month: m,
            month_name: month_name(m, lang).to_string(),
            available: availability[(m - 1) as usize],
        })
        .collect();

    Json(FishSeasonResponse { fish: params.fish, season })
}

// ── Nutrition ─────────────────────────────────────────────────────────────────

// (calories, protein_g, fat_g, carbs_g) per 100 g
static NUTRITION_TABLE: &[(&str, f64, f64, f64, f64)] = &[
    ("salmon",         208.0, 20.0,  13.0,   0.0),
    ("chicken breast", 165.0, 31.0,   3.6,   0.0),
    ("beef",           250.0, 26.0,  15.0,   0.0),
    ("egg",            155.0, 13.0,  11.0,   1.1),
    ("potato",          77.0,  2.0,   0.1,  17.0),
    ("rice",           130.0,  2.7,   0.3,  28.0),
    ("pasta",          371.0, 13.0,   1.5,  74.0),
    ("butter",         717.0,  0.9,  81.0,   0.1),
    ("olive oil",      884.0,  0.0, 100.0,   0.0),
    ("milk",            42.0,  3.4,   1.0,   5.0),
    ("cheese",         402.0, 25.0,  33.0,   1.3),
    ("tomato",          18.0,  0.9,   0.2,   3.9),
    ("onion",           40.0,  1.1,   0.1,   9.3),
    ("garlic",         149.0,  6.4,   0.5,  33.0),
    ("carrot",          41.0,  0.9,   0.2,  10.0),
    ("broccoli",        34.0,  2.8,   0.4,   7.0),
    ("spinach",         23.0,  2.9,   0.4,   3.6),
    ("lemon",           29.0,  1.1,   0.3,   9.3),
    ("avocado",        160.0,  2.0,  15.0,   9.0),
    ("walnuts",        654.0, 15.0,  65.0,  14.0),
    ("almonds",        579.0, 21.0,  50.0,  22.0),
];

#[derive(Deserialize)]
pub struct NutritionQuery {
    #[serde(default = "default_fish")]
    pub name: String,
    pub amount: Option<f64>,
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

/// GET /public/tools/nutrition?name=salmon&amount=150&lang=pl
pub async fn nutrition(Query(params): Query<NutritionQuery>) -> Json<NutritionResponse> {
    let name_lower = params.name.to_lowercase();
    let (cal, prot, fat, carbs) = NUTRITION_TABLE
        .iter()
        .find(|(n, ..)| *n == name_lower)
        .map(|&(_, cal, prot, fat, carbs)| (cal, prot, fat, carbs))
        .unwrap_or((0.0, 0.0, 0.0, 0.0));

    let lang = parse_lang(&params.lang);
    let amount = params.amount.unwrap_or(100.0);
    let f = amount / 100.0;
    let round = |x: f64| (x * 10.0).round() / 10.0;

    Json(NutritionResponse {
        name: params.name,
        amount_g: amount,
        calories:  round(cal  * f),
        protein_g: round(prot * f),
        fat_g:     round(fat  * f),
        carbs_g:   round(carbs * f),
        unit_label: label("g", lang),
    })
}

// ── Categories ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CategoriesResponse {
    pub tools: Vec<&'static str>,
}

/// GET /public/tools/categories
pub async fn list_categories() -> Json<CategoriesResponse> {
    Json(CategoriesResponse {
        tools: vec!["converter", "units", "nutrition", "fish-season"],
    })
}
