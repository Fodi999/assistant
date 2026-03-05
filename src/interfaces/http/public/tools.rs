use crate::shared::Language;
use axum::{extract::Query, response::Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Language helper ───────────────────────────────────────────────────────────

fn parse_lang(lang: &Option<String>) -> Language {
    lang.as_deref()
        .and_then(Language::from_code)
        .unwrap_or_default()
}

// ── Unit labels dictionary ────────────────────────────────────────────────────

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

pub fn unit_labels() -> HashMap<&'static str, UnitLabel> {
    let mut m = HashMap::new();

    // Mass
    m.insert("g",  UnitLabel { en: "gram",       pl: "gram",        ru: "грамм",       uk: "грам"        });
    m.insert("mg", UnitLabel { en: "milligram",   pl: "miligram",    ru: "миллиграмм",  uk: "міліграм"    });
    m.insert("kg", UnitLabel { en: "kilogram",    pl: "kilogram",    ru: "килограмм",   uk: "кілограм"    });
    m.insert("oz", UnitLabel { en: "ounce",       pl: "uncja",       ru: "унция",       uk: "унція"       });
    m.insert("lb", UnitLabel { en: "pound",       pl: "funt",        ru: "фунт",        uk: "фунт"        });

    // Volume
    m.insert("ml",    UnitLabel { en: "milliliter", pl: "mililitr",   ru: "миллилитр",   uk: "мілілітр"   });
    m.insert("l",     UnitLabel { en: "liter",      pl: "litr",       ru: "литр",        uk: "літр"       });
    m.insert("fl_oz", UnitLabel { en: "fl. ounce",  pl: "fl. uncja",  ru: "жидк. унция", uk: "рід. унція" });

    // Kitchen
    m.insert("tsp",  UnitLabel { en: "teaspoon",    pl: "łyżeczka",   ru: "чайная ложка",   uk: "чайна ложка"   });
    m.insert("tbsp", UnitLabel { en: "tablespoon",  pl: "łyżka",      ru: "столовая ложка", uk: "столова ложка" });
    m.insert("cup",  UnitLabel { en: "cup",         pl: "szklanka",   ru: "стакан",          uk: "склянка"       });

    m
}

fn label(unit: &str, lang: Language) -> String {
    let map = unit_labels();
    map.get(unit)
        .map(|l| l.for_lang(lang).to_string())
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

/// GET /public/tools/convert?value=100&from=g&to=oz&lang=ru
pub async fn convert_units(Query(params): Query<ConvertQuery>) -> Json<ConvertResponse> {
    let factor: Option<f64> = match (params.from.as_str(), params.to.as_str()) {
        // Mass
        ("g",  "oz")    => Some(0.035274),
        ("oz", "g")     => Some(28.3495),
        ("kg", "lb")    => Some(2.20462),
        ("lb", "kg")    => Some(0.453592),
        ("kg", "g")     => Some(1000.0),
        ("g",  "kg")    => Some(0.001),
        ("g",  "mg")    => Some(1000.0),
        ("mg", "g")     => Some(0.001),
        // Volume
        ("l",     "ml")    => Some(1000.0),
        ("ml",    "l")     => Some(0.001),
        ("l",     "fl_oz") => Some(33.814),
        ("fl_oz", "l")     => Some(0.0295735),
        // Kitchen
        ("tsp",  "ml")   => Some(4.92892),
        ("tbsp", "ml")   => Some(14.7868),
        ("cup",  "ml")   => Some(236.588),
        ("ml",   "tsp")  => Some(1.0 / 4.92892),
        ("ml",   "tbsp") => Some(1.0 / 14.7868),
        ("ml",   "cup")  => Some(1.0 / 236.588),
        ("tbsp", "tsp")  => Some(3.0),
        ("tsp",  "tbsp") => Some(1.0 / 3.0),
        ("cup",  "tbsp") => Some(16.0),
        ("tbsp", "cup")  => Some(1.0 / 16.0),
        // Same unit
        (a, b) if a == b => Some(1.0),
        _ => None,
    };

    let lang = parse_lang(&params.lang);
    let supported = factor.is_some();
    let result = (factor.unwrap_or(0.0) * params.value * 1_000_000.0).round() / 1_000_000.0;

    Json(ConvertResponse {
        from_label: label(&params.from, lang),
        to_label:   label(&params.to, lang),
        value: params.value,
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
    let map = unit_labels();

    let make = |code: &'static str| UnitItem {
        code,
        label: map.get(code).map(|l| l.for_lang(lang).to_string()).unwrap_or_else(|| code.to_string()),
    };

    Json(UnitsResponse {
        mass:    vec![make("g"), make("mg"), make("kg"), make("oz"), make("lb")],
        volume:  vec![make("ml"), make("l"), make("fl_oz")],
        kitchen: vec![make("tsp"), make("tbsp"), make("cup")],
    })
}

// ── Fish season ───────────────────────────────────────────────────────────────

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

    // availability: true = in season
    let availability: [bool; 12] = match params.fish.to_lowercase().as_str() {
        "salmon" => [true,  true,  true,  false, false, false, true,  true,  true,  true,  true,  true ],
        "tuna"   => [false, false, false, true,  true,  true,  true,  true,  true,  false, false, false],
        "cod"    => [true,  true,  true,  true,  false, false, false, false, false, true,  true,  true ],
        _        => [true;  12],
    };

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
    // (calories, protein, fat, carbs) per 100g
    let (cal, prot, fat, carbs) = match params.name.to_lowercase().as_str() {
        "salmon"         => (208.0, 20.0, 13.0,  0.0),
        "chicken breast" => (165.0, 31.0,  3.6,  0.0),
        "beef"           => (250.0, 26.0, 15.0,  0.0),
        "egg"            => (155.0, 13.0, 11.0,  1.1),
        "potato"         => ( 77.0,  2.0,  0.1, 17.0),
        "rice"           => (130.0,  2.7,  0.3, 28.0),
        "pasta"          => (371.0, 13.0,  1.5, 74.0),
        "butter"         => (717.0,  0.9, 81.0,  0.1),
        "olive oil"      => (884.0,  0.0,100.0,  0.0),
        "milk"           => ( 42.0,  3.4,  1.0,  5.0),
        "cheese"         => (402.0, 25.0, 33.0,  1.3),
        "tomato"         => ( 18.0,  0.9,  0.2,  3.9),
        "onion"          => ( 40.0,  1.1,  0.1,  9.3),
        "garlic"         => (149.0,  6.4,  0.5, 33.0),
        "carrot"         => ( 41.0,  0.9,  0.2, 10.0),
        "broccoli"       => ( 34.0,  2.8,  0.4,  7.0),
        "spinach"        => ( 23.0,  2.9,  0.4,  3.6),
        "lemon"          => ( 29.0,  1.1,  0.3,  9.3),
        "avocado"        => (160.0,  2.0, 15.0,  9.0),
        "walnuts"        => (654.0, 15.0, 65.0, 14.0),
        "almonds"        => (579.0, 21.0, 50.0, 22.0),
        _                => (  0.0,  0.0,  0.0,  0.0),
    };

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
