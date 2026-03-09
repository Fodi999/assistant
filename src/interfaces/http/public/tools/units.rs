//! Unit conversion handlers: convert_units, list_units, ingredient_scale.

use super::shared::{label, parse_lang, sanitize_value, SmartUnit};
use crate::domain::tools::unit_converter as uc;
use axum::extract::Query;
use axum::Json;
use serde::{Deserialize, Serialize};

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
