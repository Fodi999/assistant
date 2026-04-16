//! Flavor & texture analysis engine.
//!
//! Computes a dish's flavor/texture profile from ingredient behaviors (DSL),
//! then generates balance suggestions (add_acidity, add_moisture, etc.).
//!
//! Pipeline position: after cooking_rules, BEFORE validation.
//! This module is READ-ONLY — it analyzes but never mutates the recipe.

use serde::Serialize;

use crate::infrastructure::ingredient_cache::CachedBehavior;
use super::recipe_engine::ResolvedIngredient;

// ── Analysis result ──────────────────────────────────────────────────────────

/// Aggregated flavor/texture profile of a dish.
#[derive(Debug, Clone, Serialize)]
pub struct FlavorAnalysis {
    pub sweetness: f32,
    pub acidity: f32,
    pub bitterness: f32,
    pub umami: f32,
    pub moisture: f32,
    pub dryness: f32,
    /// Machine-readable suggestions: "add_acidity", "add_moisture", "reduce_bitterness"
    pub suggestions: Vec<String>,
    /// Dominant flavor axis (highest absolute value)
    pub dominant: Option<String>,
    /// Overall balance score 0.0 (imbalanced) – 1.0 (perfect)
    pub balance_score: f32,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Analyze a resolved dish and return its flavor/texture profile.
pub fn analyze_dish(ingredients: &[ResolvedIngredient]) -> FlavorAnalysis {
    let mut state = FlavorAnalysis {
        sweetness: 0.0,
        acidity: 0.0,
        bitterness: 0.0,
        umami: 0.0,
        moisture: 0.0,
        dryness: 0.0,
        suggestions: vec![],
        dominant: None,
        balance_score: 0.0,
    };

    for ing in ingredients {
        if let Some(ref product) = ing.product {
            // Weight factor: heavier ingredients contribute more
            let weight_factor = (ing.gross_g / 100.0).min(3.0).max(0.1);
            for b in &product.behaviors {
                apply_behavior(&mut state, b, weight_factor);
            }
        }
    }

    state.dominant = detect_dominant(&state);
    state.balance_score = compute_balance_score(&state);
    state.suggestions = analyze_balance(&state);

    state
}

// ── Behavior application ─────────────────────────────────────────────────────

fn apply_behavior(state: &mut FlavorAnalysis, b: &CachedBehavior, weight: f32) {
    let intensity = b.intensity.unwrap_or(0.5);
    let polarity_sign = match b.polarity.as_deref() {
        Some("+") => 1.0_f32,
        Some("-") => -1.0,
        _ => 0.0,
    };
    let k = intensity * polarity_sign * weight;

    match b.effect.as_deref().unwrap_or("") {
        // ── Flavor axes ──
        "sweetness_increase" => state.sweetness += k,
        "acidity_increase" => state.acidity += k,
        "bitterness_increase" => state.bitterness += k,
        "bitterness_reduce" => state.bitterness += k, // k is negative (polarity "-")
        "umami_boost" | "umami_increase" => state.umami += k,
        "aroma_release" => {} // tracked separately in future
        "flavor_balance" => {
            // sweet_sour_balance: slight nudge toward equilibrium
            let delta = intensity * weight * 0.3;
            state.sweetness += delta;
            state.acidity += delta;
        }

        // ── Texture axes ──
        "moisture_release" => state.moisture += k,
        "softening" => state.moisture += k * 0.5,
        "drying" => state.dryness += k.abs(), // drying is always positive on dryness axis
        "crust_formation" => state.dryness += intensity * weight * 0.3,
        "thickening" | "gel_formation" => state.moisture += intensity * weight * 0.2,

        // ── Chemistry (indirect flavor effects) ──
        "maillard_reaction" => {
            state.sweetness += intensity * weight * 0.2;
            state.umami += intensity * weight * 0.3;
        }
        "starch_gelatinization" => state.moisture += intensity * weight * 0.2,
        "protein_denaturation" => state.umami += intensity * weight * 0.15,

        _ => {}
    }
}

// ── Balance analysis ─────────────────────────────────────────────────────────

fn analyze_balance(state: &FlavorAnalysis) -> Vec<String> {
    let mut out = vec![];

    // Sweet vs Acid balance
    if state.sweetness > 0.0 && state.sweetness > state.acidity * 1.5 {
        out.push("add_acidity".into());
    }
    if state.acidity > 0.0 && state.acidity > state.sweetness * 1.5 {
        out.push("add_sweetness".into());
    }

    // Moisture vs Dryness
    if state.dryness > state.moisture && state.dryness > 0.3 {
        out.push("add_moisture".into());
    }
    if state.moisture > state.dryness * 2.0 && state.moisture > 1.0 {
        out.push("reduce_moisture".into());
    }

    // Bitterness threshold
    if state.bitterness > 0.7 {
        out.push("reduce_bitterness".into());
    }

    // Flat dish (nothing stands out)
    let max_val = [state.sweetness, state.acidity, state.umami, state.bitterness]
        .iter()
        .cloned()
        .fold(0.0_f32, f32::max);
    if max_val < 0.2 && state.moisture < 0.3 {
        out.push("dish_too_flat".into());
    }

    // Umami boost suggestion
    if state.umami < 0.1 && state.sweetness > 0.3 {
        out.push("consider_umami".into());
    }

    out
}

fn detect_dominant(state: &FlavorAnalysis) -> Option<String> {
    let axes = [
        ("sweetness", state.sweetness),
        ("acidity", state.acidity),
        ("bitterness", state.bitterness),
        ("umami", state.umami),
    ];
    axes.iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .filter(|(_, v)| *v > 0.1)
        .map(|(name, _)| name.to_string())
}

fn compute_balance_score(state: &FlavorAnalysis) -> f32 {
    // Balance = how close the flavor axes are to each other
    // Perfect balance = all axes roughly equal (or all zero)
    let vals = [state.sweetness, state.acidity, state.bitterness, state.umami];
    let max = vals.iter().cloned().fold(0.0_f32, f32::max);
    if max < 0.05 {
        return 0.5; // no data → neutral score
    }
    let mean = vals.iter().sum::<f32>() / vals.len() as f32;
    let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / vals.len() as f32;
    // Lower variance = better balance. Map to 0–1.
    (1.0 - (variance / (max * max)).sqrt()).max(0.0).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_behavior(effect: &str, polarity: &str, intensity: f32) -> CachedBehavior {
        CachedBehavior {
            key: effect.to_string(),
            behavior_type: "flavor".into(),
            effect: Some(effect.into()),
            trigger: Some("heat".into()),
            intensity: Some(intensity),
            temp_threshold: None,
            targets: vec![],
            polarity: Some(polarity.into()),
            domain: Some("flavor".into()),
            pairing_score: None,
        }
    }

    #[test]
    fn test_sweetness_dominant_suggests_acidity() {
        use crate::infrastructure::ingredient_cache::IngredientData;
        let ingredients = vec![ResolvedIngredient {
            product: Some(IngredientData {
                slug: "apple".into(),
                name_en: "Apple".into(),
                name_ru: "Яблоко".into(),
                name_pl: "Jabłko".into(),
                name_uk: "Яблуко".into(),
                calories_per_100g: 52.0,
                protein_per_100g: 0.3,
                fat_per_100g: 0.2,
                carbs_per_100g: 14.0,
                image_url: None,
                product_type: "fruit".into(),
                density_g_per_ml: None,
                behaviors: vec![
                    make_behavior("sweetness_increase", "+", 0.9),
                ],
            }),
            slug_hint: "apple".into(),
            resolved_slug: Some("apple".into()),
            state: "raw".into(),
            role: "main".into(),
            gross_g: 200.0,
            cleaned_net_g: 180.0,
            cooked_net_g: 180.0,
            kcal: 104,
            protein_g: 0.6,
            fat_g: 0.4,
            carbs_g: 28.0,
        }];

        let analysis = analyze_dish(&ingredients);
        assert!(analysis.sweetness > 0.0);
        assert!(analysis.suggestions.contains(&"add_acidity".to_string()));
        assert_eq!(analysis.dominant, Some("sweetness".into()));
    }
}
