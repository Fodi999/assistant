//! Adaptation Engine — smart rebalancing after constraint removal.
//!
//! Pipeline position:
//!   constraint_policy (remove) → **adaptation_engine** (adapt + balance) → cooking_rules
//!
//! What it does:
//!   1. If protein was removed (vegan → meat gone), add plant protein substitute
//!   2. If oil exceeds goal limit, reduce oil portion
//!   3. If carbs exceed goal limit, reduce carb portion
//!   4. If protein below target, increase protein portion
//!   5. If total kcal out of range, scale portions
//!
//! Every change is logged in `AdaptationReport` for the UI "Изменения" block.

use serde::Serialize;
use super::recipe_engine::ResolvedIngredient;
use super::goal_engine::{GoalProfile, GoalStrategy};
use super::nutrition_math::round1;

// ── Adaptation Report ────────────────────────────────────────────────────────

/// A single adaptation action taken.
#[derive(Debug, Clone, Serialize)]
pub struct AdaptationAction {
    /// "added" | "removed" | "increased" | "reduced" | "substituted"
    pub action: String,
    /// The ingredient slug affected
    pub slug: String,
    /// Human-readable detail, e.g. "80g → 50g" or "protein source"
    pub detail: String,
}

/// Full report of all adaptations applied.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdaptationReport {
    pub actions: Vec<AdaptationAction>,
    pub strategy_applied: Option<String>,
}

impl AdaptationReport {
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

// ── Main Entry Point ─────────────────────────────────────────────────────────

/// Adapt ingredient list to match goal profile.
/// Called AFTER constraint_policy removal, BEFORE cooking_rules.
///
/// `removed_types` = product_types that were removed by constraints (e.g. "meat", "dairy").
pub fn adapt_to_goal(
    ingredients: &mut Vec<ResolvedIngredient>,
    profile: &GoalProfile,
    removed_types: &[String],
    servings: u8,
) -> AdaptationReport {
    let mut report = AdaptationReport {
        strategy_applied: Some(format!("{:?}", profile.strategy)),
        ..Default::default()
    };

    // ── 1. Protein compensation: add plant protein if meat/fish was removed ──
    compensate_protein(ingredients, profile, removed_types, &mut report);

    // ── 2. Oil reduction per goal ─────────────────────────────────────────
    cap_oil(ingredients, profile, &mut report);

    // ── 3. Strategy-specific portion adjustments ──────────────────────────
    match profile.strategy {
        GoalStrategy::ReduceCalories => reduce_for_weight_loss(ingredients, profile, servings, &mut report),
        GoalStrategy::IncreaseProtein => boost_protein(ingredients, profile, servings, &mut report),
        GoalStrategy::IncreaseCalories => boost_for_muscle(ingredients, profile, servings, &mut report),
        GoalStrategy::ReduceCarbs => reduce_carbs(ingredients, profile, servings, &mut report),
        GoalStrategy::Balanced => {} // no extra adjustments
    }

    report
}

// ── 1. Protein Compensation ──────────────────────────────────────────────────

/// If constraint removal eliminated all protein sources, insert a plant-based substitute.
fn compensate_protein(
    ingredients: &mut Vec<ResolvedIngredient>,
    _profile: &GoalProfile,
    removed_types: &[String],
    report: &mut AdaptationReport,
) {
    let had_animal_protein = removed_types.iter().any(|t| {
        t == "meat" || t == "fish" || t == "seafood" || t == "dairy"
    });

    if !had_animal_protein {
        return;
    }

    // Check if recipe still has a protein source
    let has_protein = ingredients.iter().any(|i| i.role == "protein");
    if has_protein {
        return;
    }

    // Check if any existing ingredient is a legume/tofu
    let has_plant_protein = ingredients.iter().any(|i| {
        let slug = slug_lower(i);
        slug.contains("tofu") || slug.contains("tempeh")
            || slug.contains("chickpea") || slug.contains("lentil")
            || slug.contains("bean") || slug.contains("edamame")
            || i.product.as_ref().map(|p| p.product_type.as_str()) == Some("legume")
    });

    if has_plant_protein {
        return;
    }

    // Insert a synthetic chickpea ingredient as placeholder.
    // The actual product resolution happens in the pipeline caller.
    let substitute = ResolvedIngredient {
        product: None,
        slug_hint: "chickpeas".into(),
        resolved_slug: None,
        state: "boiled".into(),
        role: "protein".into(),
        gross_g: 80.0,
        cleaned_net_g: 80.0,
        cooked_net_g: 80.0,
        // Approximate: chickpeas cooked ~160kcal/100g, 9g protein/100g
        kcal: 128,
        protein_g: 7.2,
        fat_g: 2.0,
        carbs_g: 20.0,
    };

    ingredients.push(substitute);
    report.actions.push(AdaptationAction {
        action: "added".into(),
        slug: "chickpeas".into(),
        detail: "plant protein substitute (80g)".into(),
    });
}

// ── 2. Oil Cap ───────────────────────────────────────────────────────────────

/// Cap oil/fat ingredients to goal limit.
fn cap_oil(
    ingredients: &mut Vec<ResolvedIngredient>,
    profile: &GoalProfile,
    report: &mut AdaptationReport,
) {
    for ing in ingredients.iter_mut() {
        if ing.role != "oil" { continue; }

        let max = profile.max_oil_g;
        if ing.gross_g > max {
            let old = ing.gross_g;
            let ratio = max / old;

            ing.gross_g = max;
            ing.cleaned_net_g = max;
            ing.cooked_net_g = max;
            ing.kcal = (ing.kcal as f32 * ratio).round() as u32;
            ing.fat_g = round1(ing.fat_g * ratio);
            ing.protein_g = round1(ing.protein_g * ratio);
            ing.carbs_g = round1(ing.carbs_g * ratio);

            report.actions.push(AdaptationAction {
                action: "reduced".into(),
                slug: ing.slug_hint.clone(),
                detail: format!("{:.0}g → {:.0}g", old, max),
            });
        }
    }
}

// ── 3a. Weight Loss: reduce carbs/fat, keep protein ──────────────────────────

fn reduce_for_weight_loss(
    ingredients: &mut Vec<ResolvedIngredient>,
    _profile: &GoalProfile,
    _servings: u8,
    report: &mut AdaptationReport,
) {
    // Reduce carb-heavy ingredients (base) by 30%
    for ing in ingredients.iter_mut() {
        if ing.role == "base" {
            let old = ing.cooked_net_g;
            let factor = 0.7;
            scale_ingredient(ing, factor);
            report.actions.push(AdaptationAction {
                action: "reduced".into(),
                slug: ing.slug_hint.clone(),
                detail: format!("{:.0}g → {:.0}g (weight loss)", old, ing.cooked_net_g),
            });
        }
    }

    // Reduce condiment/sauce portions by 40%
    for ing in ingredients.iter_mut() {
        if ing.role == "condiment" {
            let old = ing.cooked_net_g;
            let factor = 0.6;
            scale_ingredient(ing, factor);
            report.actions.push(AdaptationAction {
                action: "reduced".into(),
                slug: ing.slug_hint.clone(),
                detail: format!("{:.0}g → {:.0}g (weight loss)", old, ing.cooked_net_g),
            });
        }
    }
}

// ── 3b. High Protein: increase protein portion ───────────────────────────────

fn boost_protein(
    ingredients: &mut Vec<ResolvedIngredient>,
    profile: &GoalProfile,
    servings: u8,
    report: &mut AdaptationReport,
) {
    let total_protein: f32 = ingredients.iter().map(|i| i.protein_g).sum();
    let per_serving = total_protein / servings.max(1) as f32;

    // Use goal profile range, not generic ProteinLevel —
    // e.g. HighProtein demands 40g+ even if 25g is "Optimal" generically.
    let deficit = profile.protein_deficit(per_serving);
    if deficit <= 0.0 { return; }

    // Increase protein ingredients by up to 40%
    for ing in ingredients.iter_mut() {
        if ing.role == "protein" {
            let old = ing.cooked_net_g;
            let factor = 1.4_f32.min(1.0 + deficit / per_serving.max(1.0));
            scale_ingredient(ing, factor);
            report.actions.push(AdaptationAction {
                action: "increased".into(),
                slug: ing.slug_hint.clone(),
                detail: format!("{:.0}g → {:.0}g (protein boost)", old, ing.cooked_net_g),
            });
        }
    }
}

// ── 3c. Muscle Gain: increase protein + carbs + portion ──────────────────────

fn boost_for_muscle(
    ingredients: &mut Vec<ResolvedIngredient>,
    _profile: &GoalProfile,
    _servings: u8,
    report: &mut AdaptationReport,
) {
    // Increase protein by 30%
    for ing in ingredients.iter_mut() {
        if ing.role == "protein" {
            let old = ing.cooked_net_g;
            scale_ingredient(ing, 1.3);
            report.actions.push(AdaptationAction {
                action: "increased".into(),
                slug: ing.slug_hint.clone(),
                detail: format!("{:.0}g → {:.0}g (muscle gain)", old, ing.cooked_net_g),
            });
        }
    }

    // Increase base (carbs) by 25%
    for ing in ingredients.iter_mut() {
        if ing.role == "base" {
            let old = ing.cooked_net_g;
            scale_ingredient(ing, 1.25);
            report.actions.push(AdaptationAction {
                action: "increased".into(),
                slug: ing.slug_hint.clone(),
                detail: format!("{:.0}g → {:.0}g (muscle gain)", old, ing.cooked_net_g),
            });
        }
    }
}

// ── 3d. Reduce Carbs (keto) ─────────────────────────────────────────────────

fn reduce_carbs(
    ingredients: &mut Vec<ResolvedIngredient>,
    _profile: &GoalProfile,
    _servings: u8,
    report: &mut AdaptationReport,
) {
    // Reduce base (carbs) by 50%
    for ing in ingredients.iter_mut() {
        if ing.role == "base" {
            let old = ing.cooked_net_g;
            scale_ingredient(ing, 0.5);
            report.actions.push(AdaptationAction {
                action: "reduced".into(),
                slug: ing.slug_hint.clone(),
                detail: format!("{:.0}g → {:.0}g (low carb)", old, ing.cooked_net_g),
            });
        }
    }

    // Boost protein slightly to compensate
    for ing in ingredients.iter_mut() {
        if ing.role == "protein" {
            let old = ing.cooked_net_g;
            scale_ingredient(ing, 1.15);
            report.actions.push(AdaptationAction {
                action: "increased".into(),
                slug: ing.slug_hint.clone(),
                detail: format!("{:.0}g → {:.0}g (low carb compensation)", old, ing.cooked_net_g),
            });
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Scale all gram/macro fields of an ingredient by a factor.
fn scale_ingredient(ing: &mut ResolvedIngredient, factor: f32) {
    ing.gross_g = round1(ing.gross_g * factor);
    ing.cleaned_net_g = round1(ing.cleaned_net_g * factor);
    ing.cooked_net_g = round1(ing.cooked_net_g * factor);
    ing.kcal = (ing.kcal as f32 * factor).round() as u32;
    ing.protein_g = round1(ing.protein_g * factor);
    ing.fat_g = round1(ing.fat_g * factor);
    ing.carbs_g = round1(ing.carbs_g * factor);
}

fn slug_lower(ing: &ResolvedIngredient) -> String {
    ing.resolved_slug
        .as_deref()
        .unwrap_or(&ing.slug_hint)
        .to_lowercase()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ingredient_cache::IngredientData;
    use super::super::goal_engine::profile_for;
    use super::super::goal_modifier::HealthModifier;

    fn make_ing(slug: &str, product_type: &str, role: &str, grams: f32) -> ResolvedIngredient {
        let kcal_100 = match product_type {
            "meat" => 200.0,
            "vegetable" => 40.0,
            "grain" => 350.0,
            "oil" => 884.0,
            _ => 100.0,
        };
        let protein_100 = match product_type {
            "meat" => 25.0,
            "legume" => 9.0,
            _ => 2.0,
        };
        ResolvedIngredient {
            product: Some(IngredientData {
                slug: slug.into(),
                name_en: slug.into(),
                name_ru: slug.into(),
                name_pl: slug.into(),
                name_uk: slug.into(),
                calories_per_100g: kcal_100,
                protein_per_100g: protein_100,
                fat_per_100g: 5.0,
                carbs_per_100g: 10.0,
                image_url: None,
                product_type: product_type.into(),
                density_g_per_ml: None, behaviors: vec![], states: vec![],
            }),
            slug_hint: slug.into(),
            resolved_slug: Some(slug.into()),
            state: "raw".into(),
            role: role.into(),
            gross_g: grams,
            cleaned_net_g: grams,
            cooked_net_g: grams,
            kcal: (kcal_100 * grams / 100.0).round() as u32,
            protein_g: round1(protein_100 * grams / 100.0),
            fat_g: round1(5.0 * grams / 100.0),
            carbs_g: round1(10.0 * grams / 100.0),
        }
    }

    #[test]
    fn protein_compensation_adds_chickpeas_when_meat_removed() {
        let mut ings = vec![
            make_ing("beet", "vegetable", "side", 50.0),
            make_ing("potato", "vegetable", "side", 80.0),
            make_ing("olive-oil", "oil", "oil", 15.0),
        ];
        let profile = profile_for(HealthModifier::None);
        let removed_types = vec!["meat".to_string()];

        let report = adapt_to_goal(&mut ings, &profile, &removed_types, 1);

        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert!(slugs.contains(&"chickpeas"), "should add chickpeas");
        assert!(report.actions.iter().any(|a| a.action == "added" && a.slug == "chickpeas"));
    }

    #[test]
    fn no_compensation_if_protein_exists() {
        let mut ings = vec![
            make_ing("chicken-breast", "meat", "protein", 100.0),
            make_ing("beet", "vegetable", "side", 50.0),
        ];
        let profile = profile_for(HealthModifier::None);
        let removed_types = vec![];

        let report = adapt_to_goal(&mut ings, &profile, &removed_types, 1);
        assert!(report.actions.iter().all(|a| a.slug != "chickpeas"));
    }

    #[test]
    fn oil_capped_for_weight_loss() {
        let mut ings = vec![
            make_ing("olive-oil", "oil", "oil", 20.0),
            make_ing("chicken-breast", "meat", "protein", 100.0),
        ];
        let profile = profile_for(HealthModifier::LowCalorie);

        let report = adapt_to_goal(&mut ings, &profile, &[], 1);

        let oil = ings.iter().find(|i| i.slug_hint == "olive-oil").unwrap();
        assert!(oil.gross_g <= 5.0, "oil should be capped at 5g, got {}", oil.gross_g);
        assert!(report.actions.iter().any(|a| a.action == "reduced" && a.slug == "olive-oil"));
    }

    #[test]
    fn carbs_reduced_for_weight_loss() {
        let mut ings = vec![
            make_ing("rice", "grain", "base", 80.0),
            make_ing("chicken-breast", "meat", "protein", 100.0),
        ];
        let profile = profile_for(HealthModifier::LowCalorie);

        adapt_to_goal(&mut ings, &profile, &[], 1);

        let rice = ings.iter().find(|i| i.slug_hint == "rice").unwrap();
        assert!(rice.cooked_net_g < 80.0, "rice should be reduced");
    }

    #[test]
    fn protein_boosted_for_high_protein_goal() {
        let mut ings = vec![
            make_ing("chicken-breast", "meat", "protein", 100.0),
            make_ing("beet", "vegetable", "side", 50.0),
        ];
        let profile = profile_for(HealthModifier::HighProtein);

        let report = adapt_to_goal(&mut ings, &profile, &[], 1);

        let chicken = ings.iter().find(|i| i.slug_hint == "chicken-breast").unwrap();
        // Protein was 25g for 100g at 25% — below 40g target, so should be boosted
        assert!(chicken.cooked_net_g > 100.0, "protein should be increased for high protein goal");
        assert!(report.actions.iter().any(|a| a.action == "increased"));
    }

    #[test]
    fn muscle_gain_increases_both_protein_and_carbs() {
        let mut ings = vec![
            make_ing("chicken-breast", "meat", "protein", 100.0),
            make_ing("rice", "grain", "base", 60.0),
            make_ing("beet", "vegetable", "side", 50.0),
        ];
        let profile = profile_for(HealthModifier::ComfortFood);

        let report = adapt_to_goal(&mut ings, &profile, &[], 1);

        let chicken = ings.iter().find(|i| i.slug_hint == "chicken-breast").unwrap();
        let rice = ings.iter().find(|i| i.slug_hint == "rice").unwrap();
        assert!(chicken.cooked_net_g > 100.0, "protein increased");
        assert!(rice.cooked_net_g > 60.0, "carbs increased");
        assert!(report.actions.len() >= 2);
    }

    #[test]
    fn low_carb_halves_base() {
        let mut ings = vec![
            make_ing("rice", "grain", "base", 80.0),
            make_ing("chicken-breast", "meat", "protein", 100.0),
        ];
        let profile = profile_for(HealthModifier::LowCarb);

        adapt_to_goal(&mut ings, &profile, &[], 1);

        let rice = ings.iter().find(|i| i.slug_hint == "rice").unwrap();
        assert!((rice.cooked_net_g - 40.0).abs() < 1.0, "rice should be ~40g, got {}", rice.cooked_net_g);
    }

    #[test]
    fn balanced_no_changes() {
        let mut ings = vec![
            make_ing("chicken-breast", "meat", "protein", 100.0),
            make_ing("rice", "grain", "base", 60.0),
            make_ing("olive-oil", "oil", "oil", 15.0),
        ];
        let profile = profile_for(HealthModifier::None);

        let report = adapt_to_goal(&mut ings, &profile, &[], 1);
        assert!(report.actions.is_empty(), "balanced should not change anything");
    }
}
