use axum::{extract::Query, response::Json};
use serde::{Deserialize, Serialize};

// ── Unit conversion ──────────────────────────────────────────────────────────

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
}

/// GET /public/chef-reference/convert?value=100&from=g&to=oz
pub async fn convert_units(Query(params): Query<ConvertQuery>) -> Json<ConvertResponse> {
    let factor = match (params.from.as_str(), params.to.as_str()) {
        ("g", "oz") => 0.035274,
        ("oz", "g") => 28.3495,
        ("kg", "lb") => 2.20462,
        ("lb", "kg") => 0.453592,
        ("kg", "g") => 1000.0,
        ("g", "kg") => 0.001,
        ("l", "ml") => 1000.0,
        ("ml", "l") => 0.001,
        ("l", "fl_oz") => 33.814,
        ("fl_oz", "l") => 0.0295735,
        ("tsp", "ml") => 4.92892,
        ("tbsp", "ml") => 14.7868,
        ("cup", "ml") => 236.588,
        ("ml", "tsp") => 1.0 / 4.92892,
        ("ml", "tbsp") => 1.0 / 14.7868,
        ("ml", "cup") => 1.0 / 236.588,
        _ => 1.0,
    };

    Json(ConvertResponse {
        value: params.value,
        from: params.from,
        to: params.to,
        result: params.value * factor,
    })
}

// ── Ingredient nutrition ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct IngredientResponse {
    pub name: String,
    pub protein: f32,
    pub fat: f32,
    pub carbs: f32,
    pub calories: f32,
}

/// GET /public/chef-reference/ingredient
pub async fn get_ingredient() -> Json<IngredientResponse> {
    Json(IngredientResponse {
        name: "salmon".to_string(),
        protein: 20.0,
        fat: 13.0,
        carbs: 0.0,
        calories: 208.0,
    })
}

// ── Fish season ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct FishSeason {
    pub fish: String,
    pub month: String,
    pub available: bool,
}

/// GET /public/chef-reference/fish-season
pub async fn fish_season() -> Json<Vec<FishSeason>> {
    Json(vec![
        FishSeason { fish: "salmon".to_string(), month: "january".to_string(),   available: true  },
        FishSeason { fish: "salmon".to_string(), month: "february".to_string(),  available: true  },
        FishSeason { fish: "salmon".to_string(), month: "march".to_string(),     available: true  },
        FishSeason { fish: "salmon".to_string(), month: "april".to_string(),     available: false },
        FishSeason { fish: "salmon".to_string(), month: "may".to_string(),       available: false },
        FishSeason { fish: "salmon".to_string(), month: "june".to_string(),      available: false },
        FishSeason { fish: "salmon".to_string(), month: "july".to_string(),      available: true  },
        FishSeason { fish: "salmon".to_string(), month: "august".to_string(),    available: true  },
        FishSeason { fish: "salmon".to_string(), month: "september".to_string(), available: true  },
        FishSeason { fish: "salmon".to_string(), month: "october".to_string(),   available: true  },
        FishSeason { fish: "salmon".to_string(), month: "november".to_string(),  available: true  },
        FishSeason { fish: "salmon".to_string(), month: "december".to_string(),  available: true  },
    ])
}
