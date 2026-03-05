use axum::{extract::Query, response::Json};
use serde::{Deserialize, Serialize};

// ── Unit converter ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ConvertQuery {
    pub value: f64,
    pub from: String,
    pub to: String,
}

#[derive(Serialize)]
pub struct ConvertResponse {
    pub value: f64,
    pub from: String,
    pub to: String,
    pub result: f64,
    pub supported: bool,
}

/// GET /public/tools/convert?value=100&from=g&to=oz
pub async fn convert_units(Query(params): Query<ConvertQuery>) -> Json<ConvertResponse> {
    let factor: Option<f64> = match (params.from.as_str(), params.to.as_str()) {
        // Mass
        ("g", "oz")   => Some(0.035274),
        ("oz", "g")   => Some(28.3495),
        ("kg", "lb")  => Some(2.20462),
        ("lb", "kg")  => Some(0.453592),
        ("kg", "g")   => Some(1000.0),
        ("g", "kg")   => Some(0.001),
        ("g", "mg")   => Some(1000.0),
        ("mg", "g")   => Some(0.001),
        // Volume
        ("l", "ml")   => Some(1000.0),
        ("ml", "l")   => Some(0.001),
        ("l", "fl_oz") => Some(33.814),
        ("fl_oz", "l") => Some(0.0295735),
        // Kitchen measures
        ("tsp", "ml")  => Some(4.92892),
        ("tbsp", "ml") => Some(14.7868),
        ("cup", "ml")  => Some(236.588),
        ("ml", "tsp")  => Some(1.0 / 4.92892),
        ("ml", "tbsp") => Some(1.0 / 14.7868),
        ("ml", "cup")  => Some(1.0 / 236.588),
        ("tbsp", "tsp") => Some(3.0),
        ("tsp", "tbsp") => Some(1.0 / 3.0),
        ("cup", "tbsp") => Some(16.0),
        ("tbsp", "cup") => Some(1.0 / 16.0),
        // Same unit
        (a, b) if a == b => Some(1.0),
        _ => None,
    };

    let supported = factor.is_some();
    let result = factor.unwrap_or(0.0) * params.value;

    Json(ConvertResponse {
        value: params.value,
        from: params.from,
        to: params.to,
        result,
        supported,
    })
}

// ── Fish season ───────────────────────────────────────────────────────────────

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

/// GET /public/tools/fish-season?fish=salmon
#[derive(Deserialize)]
pub struct FishQuery {
    #[serde(default = "default_fish")]
    pub fish: String,
}

fn default_fish() -> String {
    "salmon".to_string()
}

pub async fn fish_season(Query(params): Query<FishQuery>) -> Json<FishSeasonResponse> {
    let season = match params.fish.to_lowercase().as_str() {
        "salmon" => vec![
            (1,  "January",   true),
            (2,  "February",  true),
            (3,  "March",     true),
            (4,  "April",     false),
            (5,  "May",       false),
            (6,  "June",      false),
            (7,  "July",      true),
            (8,  "August",    true),
            (9,  "September", true),
            (10, "October",   true),
            (11, "November",  true),
            (12, "December",  true),
        ],
        "tuna" => vec![
            (1,  "January",   false),
            (2,  "February",  false),
            (3,  "March",     false),
            (4,  "April",     true),
            (5,  "May",       true),
            (6,  "June",      true),
            (7,  "July",      true),
            (8,  "August",    true),
            (9,  "September", true),
            (10, "October",   false),
            (11, "November",  false),
            (12, "December",  false),
        ],
        "cod" => vec![
            (1,  "January",   true),
            (2,  "February",  true),
            (3,  "March",     true),
            (4,  "April",     true),
            (5,  "May",       false),
            (6,  "June",      false),
            (7,  "July",      false),
            (8,  "August",    false),
            (9,  "September", false),
            (10, "October",   true),
            (11, "November",  true),
            (12, "December",  true),
        ],
        // Unknown fish — all year
        _ => (1u8..=12)
            .map(|m| {
                let name = month_name(m);
                (m, name, true)
            })
            .collect(),
    };

    Json(FishSeasonResponse {
        fish: params.fish,
        season: season
            .into_iter()
            .map(|(month, month_name, available)| FishSeasonEntry {
                month,
                month_name: month_name.to_string(),
                available,
            })
            .collect(),
    })
}

fn month_name(m: u8) -> &'static str {
    match m {
        1  => "January",   2  => "February", 3  => "March",
        4  => "April",     5  => "May",       6  => "June",
        7  => "July",      8  => "August",    9  => "September",
        10 => "October",   11 => "November",  12 => "December",
        _  => "Unknown",
    }
}

// ── Nutrition lookup ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NutritionQuery {
    #[serde(default = "default_fish")]
    pub name: String,
    pub amount: Option<f64>,
}

#[derive(Serialize)]
pub struct NutritionResponse {
    pub name: String,
    pub amount_g: f64,
    pub calories: f64,
    pub protein_g: f64,
    pub fat_g: f64,
    pub carbs_g: f64,
}

/// GET /public/tools/nutrition?name=salmon&amount=150
/// Static reference data (per 100g), scaled to requested amount
pub async fn nutrition(Query(params): Query<NutritionQuery>) -> Json<NutritionResponse> {
    // (calories, protein, fat, carbs) per 100g
    let (cal, prot, fat, carbs) = match params.name.to_lowercase().as_str() {
        "salmon"           => (208.0, 20.0, 13.0, 0.0),
        "chicken breast"   => (165.0, 31.0, 3.6, 0.0),
        "beef"             => (250.0, 26.0, 15.0, 0.0),
        "egg"              => (155.0, 13.0, 11.0, 1.1),
        "potato"           => (77.0,  2.0,  0.1, 17.0),
        "rice"             => (130.0, 2.7,  0.3, 28.0),
        "pasta"            => (371.0, 13.0, 1.5, 74.0),
        "butter"           => (717.0, 0.9,  81.0, 0.1),
        "olive oil"        => (884.0, 0.0,  100.0, 0.0),
        "milk"             => (42.0,  3.4,  1.0, 5.0),
        "cheese"           => (402.0, 25.0, 33.0, 1.3),
        "tomato"           => (18.0,  0.9,  0.2, 3.9),
        "onion"            => (40.0,  1.1,  0.1, 9.3),
        "garlic"           => (149.0, 6.4,  0.5, 33.0),
        "carrot"           => (41.0,  0.9,  0.2, 10.0),
        "broccoli"         => (34.0,  2.8,  0.4, 7.0),
        "spinach"          => (23.0,  2.9,  0.4, 3.6),
        "lemon"            => (29.0,  1.1,  0.3, 9.3),
        "avocado"          => (160.0, 2.0,  15.0, 9.0),
        "walnuts"          => (654.0, 15.0, 65.0, 14.0),
        "almonds"          => (579.0, 21.0, 50.0, 22.0),
        _                  => (0.0,   0.0,  0.0,  0.0),
    };

    let amount = params.amount.unwrap_or(100.0);
    let factor = amount / 100.0;

    Json(NutritionResponse {
        name: params.name,
        amount_g: amount,
        calories: (cal * factor * 10.0).round() / 10.0,
        protein_g: (prot * factor * 10.0).round() / 10.0,
        fat_g: (fat * factor * 10.0).round() / 10.0,
        carbs_g: (carbs * factor * 10.0).round() / 10.0,
    })
}
