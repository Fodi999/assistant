//! Constraint Policy — enforce dietary constraints on resolved ingredients.
//!
//! Sits in the pipeline AFTER ingredient resolution, BEFORE cooking_rules.
//! Removes / substitutes ingredients that violate user constraints.
//!
//! Pipeline position:
//!   dish_schema → ingredient_resolver → **constraint_policy** → cooking_rules → nutrition_math
//!
//! Reuses the same slug/product_type matching patterns from `nutrition_math::detect_allergens()`.

use super::recipe_engine::ResolvedIngredient;
use super::user_constraints::{UserConstraints, DietaryMode};

/// Result of applying constraints: what was removed, what was substituted.
#[derive(Debug, Clone, Default)]
pub struct ConstraintReport {
    /// Ingredients removed with reason, e.g. ("milk", "lactose-free")
    pub removed: Vec<(String, String)>,
    /// Substitutions applied, e.g. ("butter", "olive-oil", "dairy-free substitution")
    pub substituted: Vec<(String, String, String)>,
    /// Human-readable summary lines for the TechCard
    pub messages: Vec<String>,
}

/// Apply user dietary constraints: remove or substitute violating ingredients.
///
/// Called in `resolve_dish()` after ingredient resolution + auto_insert_implicit,
/// BEFORE cooking_rules::apply_constraints.
pub fn apply_dietary_constraints(
    ingredients: &mut Vec<ResolvedIngredient>,
    constraints: &UserConstraints,
) -> ConstraintReport {
    if constraints.is_empty() {
        return ConstraintReport::default();
    }

    let mut report = ConstraintReport::default();

    // ── 1. Dietary mode: broad category removal ──────────────────────────

    if let Some(mode) = constraints.dietary_mode {
        apply_dietary_mode(ingredients, mode, &mut report);
    }

    // ── 2. Allergen exclusions ───────────────────────────────────────────

    for allergen in &constraints.exclude_allergens {
        apply_allergen_exclusion(ingredients, allergen, &mut report);
    }

    // ── 3. Specific slug bans ────────────────────────────────────────────

    for slug in &constraints.exclude_slugs {
        apply_slug_ban(ingredients, slug, &mut report);
    }

    // ── 4. Product type bans ─────────────────────────────────────────────

    for ptype in &constraints.exclude_types {
        apply_type_ban(ingredients, ptype, &mut report);
    }

    report
}

// ── Dietary mode enforcement ─────────────────────────────────────────────────

fn apply_dietary_mode(
    ingredients: &mut Vec<ResolvedIngredient>,
    mode: DietaryMode,
    report: &mut ConstraintReport,
) {
    let reason = match mode {
        DietaryMode::Vegan => "vegan diet",
        DietaryMode::Vegetarian => "vegetarian diet",
        DietaryMode::Pescatarian => "pescatarian diet",
    };

    ingredients.retain(|ing| {
        let pt = ing.product.as_ref()
            .map(|p| p.product_type.as_str())
            .unwrap_or("");
        let slug = slug_lower(ing);

        let should_remove = match mode {
            DietaryMode::Vegan => {
                // Remove: meat, fish, seafood, dairy, eggs
                pt == "meat" || pt == "fish" || pt == "seafood" || pt == "dairy"
                    || slug.contains("egg")
                    || slug.contains("butter") || slug.contains("cream")
                    || slug.contains("cheese") || slug.contains("milk")
                    || slug.contains("honey")
            }
            DietaryMode::Vegetarian => {
                // Remove: meat, fish, seafood
                pt == "meat" || pt == "fish" || pt == "seafood"
            }
            DietaryMode::Pescatarian => {
                // Remove: meat only (fish & seafood OK)
                pt == "meat"
            }
        };

        if should_remove {
            let name = &ing.slug_hint;
            report.removed.push((name.clone(), reason.into()));
            report.messages.push(format!("Removed {} ({})", name, reason));
            false
        } else {
            true
        }
    });
}

// ── Allergen exclusion ───────────────────────────────────────────────────────

fn apply_allergen_exclusion(
    ingredients: &mut Vec<ResolvedIngredient>,
    allergen: &str,
    report: &mut ConstraintReport,
) {
    let reason = format!("{}-free", allergen);

    // First, check if any substitution is possible
    let mut substitutions: Vec<(usize, String)> = Vec::new();

    for (idx, ing) in ingredients.iter().enumerate() {
        if matches_allergen(ing, allergen) {
            if let Some(sub) = suggest_substitution(ing, allergen) {
                substitutions.push((idx, sub));
            }
        }
    }

    // Apply substitutions (mark for the report, but actual substitute product
    // requires cache lookup which happens at the caller level)
    for (_, sub) in &substitutions {
        report.messages.push(format!("Consider substituting with {}", sub));
    }

    // Remove violating ingredients
    ingredients.retain(|ing| {
        if matches_allergen(ing, allergen) {
            let name = &ing.slug_hint;
            report.removed.push((name.clone(), reason.clone()));
            report.messages.push(format!("Removed {} ({})", name, reason));
            false
        } else {
            true
        }
    });
}

/// Check if an ingredient matches a specific allergen.
/// Reuses the same patterns from `nutrition_math::detect_allergens()`.
fn matches_allergen(ing: &ResolvedIngredient, allergen: &str) -> bool {
    let slug = slug_lower(ing);
    let pt = ing.product.as_ref()
        .map(|p| p.product_type.as_str())
        .unwrap_or("");

    match allergen {
        "lactose" => {
            pt == "dairy" || slug.contains("milk") || slug.contains("cream")
                || slug.contains("cheese") || slug.contains("butter")
                || slug.contains("yogurt") || slug.contains("kefir")
                || slug.contains("sour-cream") || slug.contains("smetana")
        }
        "gluten" => {
            slug.contains("wheat") || slug.contains("flour") || slug.contains("pasta")
                || slug.contains("bread") || slug.contains("barley") || slug.contains("rye")
                || slug.contains("oat") || slug.contains("spaghetti") || slug.contains("noodle")
                || slug.contains("couscous") || slug.contains("semolina")
                || (pt == "grain" && !slug.contains("rice") && !slug.contains("corn")
                    && !slug.contains("buckwheat") && !slug.contains("quinoa"))
        }
        "nuts" => {
            pt == "nut" || slug.contains("almond") || slug.contains("walnut")
                || slug.contains("cashew") || slug.contains("peanut")
                || slug.contains("hazelnut") || slug.contains("pecan")
                || slug.contains("pistachio") || slug.contains("macadamia")
                || slug.contains("pine-nut")
        }
        "eggs" => {
            slug.contains("egg")
        }
        "fish" => {
            pt == "fish"
        }
        "shellfish" => {
            pt == "seafood" || slug.contains("shrimp") || slug.contains("prawn")
                || slug.contains("crab") || slug.contains("lobster")
                || slug.contains("mussel") || slug.contains("oyster")
                || slug.contains("squid") || slug.contains("octopus")
                || slug.contains("clam")
        }
        "soy" => {
            slug.contains("soy") || slug.contains("tofu")
                || slug.contains("edamame") || slug.contains("tempeh")
        }
        _ => false,
    }
}

/// Suggest a substitution for a removed ingredient.
fn suggest_substitution(ing: &ResolvedIngredient, allergen: &str) -> Option<String> {
    let slug = slug_lower(ing);

    match allergen {
        "lactose" => {
            if slug.contains("butter") {
                Some("olive-oil".into())
            } else if slug.contains("milk") || slug.contains("cream") {
                Some("coconut-milk".into())
            } else if slug.contains("cheese") {
                None // no universal cheese substitute
            } else if slug.contains("sour-cream") || slug.contains("smetana") {
                None
            } else {
                None
            }
        }
        "gluten" => {
            if slug.contains("flour") || slug.contains("wheat") {
                Some("rice-flour".into()) // or buckwheat-flour
            } else if slug.contains("pasta") || slug.contains("spaghetti") || slug.contains("noodle") {
                Some("rice-noodle".into())
            } else if slug.contains("bread") {
                None // too varied
            } else {
                None
            }
        }
        "eggs" => {
            // No universal egg substitute in savory cooking
            None
        }
        "soy" => {
            if slug.contains("soy-sauce") {
                Some("coconut-aminos".into())
            } else {
                None
            }
        }
        _ => None,
    }
}

// ── Slug / type bans ─────────────────────────────────────────────────────────

fn apply_slug_ban(
    ingredients: &mut Vec<ResolvedIngredient>,
    banned_slug: &str,
    report: &mut ConstraintReport,
) {
    let reason = format!("excluded: {}", banned_slug);
    ingredients.retain(|ing| {
        let slug = slug_lower(ing);
        if slug.contains(banned_slug) {
            report.removed.push((ing.slug_hint.clone(), reason.clone()));
            report.messages.push(format!("Removed {} ({})", ing.slug_hint, reason));
            false
        } else {
            true
        }
    });
}

fn apply_type_ban(
    ingredients: &mut Vec<ResolvedIngredient>,
    banned_type: &str,
    report: &mut ConstraintReport,
) {
    let reason = format!("excluded type: {}", banned_type);
    ingredients.retain(|ing| {
        let pt = ing.product.as_ref()
            .map(|p| p.product_type.as_str())
            .unwrap_or("");
        if pt == banned_type {
            report.removed.push((ing.slug_hint.clone(), reason.clone()));
            report.messages.push(format!("Removed {} ({})", ing.slug_hint, reason));
            false
        } else {
            true
        }
    });
}

// ── Helpers ──────────────────────────────────────────────────────────────────

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

    fn make_ing(slug: &str, product_type: &str) -> ResolvedIngredient {
        ResolvedIngredient {
            product: Some(IngredientData {
                slug: slug.into(),
                name_en: slug.into(),
                name_ru: slug.into(),
                name_pl: slug.into(),
                name_uk: slug.into(),
                calories_per_100g: 100.0,
                protein_per_100g: 10.0,
                fat_per_100g: 5.0,
                carbs_per_100g: 10.0,
                image_url: None,
                product_type: product_type.into(),
                density_g_per_ml: None,
            }),
            slug_hint: slug.into(),
            resolved_slug: Some(slug.into()),
            state: "raw".into(),
            role: "protein".into(),
            gross_g: 100.0,
            cleaned_net_g: 100.0,
            cooked_net_g: 100.0,
            kcal: 100,
            protein_g: 10.0,
            fat_g: 5.0,
            carbs_g: 10.0,
        }
    }

    #[test]
    fn vegan_removes_meat_dairy_eggs() {
        let mut ings = vec![
            make_ing("beet", "vegetable"),
            make_ing("beef", "meat"),
            make_ing("butter", "dairy"),
            make_ing("egg", "other"),
            make_ing("olive-oil", "oil"),
        ];
        let constraints = UserConstraints {
            dietary_mode: Some(DietaryMode::Vegan),
            ..Default::default()
        };
        let report = apply_dietary_constraints(&mut ings, &constraints);
        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert_eq!(slugs, vec!["beet", "olive-oil"]);
        assert_eq!(report.removed.len(), 3);
    }

    #[test]
    fn vegetarian_keeps_dairy() {
        let mut ings = vec![
            make_ing("beet", "vegetable"),
            make_ing("beef", "meat"),
            make_ing("butter", "dairy"),
            make_ing("cheese", "dairy"),
        ];
        let constraints = UserConstraints {
            dietary_mode: Some(DietaryMode::Vegetarian),
            ..Default::default()
        };
        let report = apply_dietary_constraints(&mut ings, &constraints);
        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert_eq!(slugs, vec!["beet", "butter", "cheese"]);
        assert_eq!(report.removed.len(), 1);
    }

    #[test]
    fn pescatarian_keeps_fish() {
        let mut ings = vec![
            make_ing("beet", "vegetable"),
            make_ing("beef", "meat"),
            make_ing("salmon", "fish"),
            make_ing("shrimp", "seafood"),
        ];
        let constraints = UserConstraints {
            dietary_mode: Some(DietaryMode::Pescatarian),
            ..Default::default()
        };
        let report = apply_dietary_constraints(&mut ings, &constraints);
        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert_eq!(slugs, vec!["beet", "salmon", "shrimp"]);
        assert_eq!(report.removed.len(), 1);
    }

    #[test]
    fn lactose_free_removes_dairy() {
        let mut ings = vec![
            make_ing("beet", "vegetable"),
            make_ing("butter", "dairy"),
            make_ing("milk", "dairy"),
            make_ing("olive-oil", "oil"),
        ];
        let constraints = UserConstraints {
            exclude_allergens: vec!["lactose".into()],
            ..Default::default()
        };
        let report = apply_dietary_constraints(&mut ings, &constraints);
        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert_eq!(slugs, vec!["beet", "olive-oil"]);
        assert_eq!(report.removed.len(), 2);
    }

    #[test]
    fn gluten_free_removes_flour_pasta() {
        let mut ings = vec![
            make_ing("wheat-flour", "grain"),
            make_ing("spaghetti", "grain"),
            make_ing("rice", "grain"),
            make_ing("chicken-breast", "meat"),
        ];
        let constraints = UserConstraints {
            exclude_allergens: vec!["gluten".into()],
            ..Default::default()
        };
        let report = apply_dietary_constraints(&mut ings, &constraints);
        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert_eq!(slugs, vec!["rice", "chicken-breast"]);
        assert_eq!(report.removed.len(), 2);
    }

    #[test]
    fn nut_free() {
        let mut ings = vec![
            make_ing("almond", "nut"),
            make_ing("walnut", "nut"),
            make_ing("chicken-breast", "meat"),
        ];
        let constraints = UserConstraints {
            exclude_allergens: vec!["nuts".into()],
            ..Default::default()
        };
        let report = apply_dietary_constraints(&mut ings, &constraints);
        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert_eq!(slugs, vec!["chicken-breast"]);
        assert_eq!(report.removed.len(), 2);
    }

    #[test]
    fn sugar_ban() {
        let mut ings = vec![
            make_ing("sugar", "other"),
            make_ing("beet", "vegetable"),
        ];
        let constraints = UserConstraints {
            exclude_slugs: vec!["sugar".into()],
            ..Default::default()
        };
        let report = apply_dietary_constraints(&mut ings, &constraints);
        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert_eq!(slugs, vec!["beet"]);
        assert_eq!(report.removed.len(), 1);
    }

    #[test]
    fn combined_vegan_plus_gluten_free() {
        let mut ings = vec![
            make_ing("beet", "vegetable"),
            make_ing("beef", "meat"),
            make_ing("wheat-flour", "grain"),
            make_ing("rice", "grain"),
            make_ing("olive-oil", "oil"),
        ];
        let constraints = UserConstraints {
            dietary_mode: Some(DietaryMode::Vegan),
            exclude_allergens: vec!["gluten".into()],
            ..Default::default()
        };
        let report = apply_dietary_constraints(&mut ings, &constraints);
        let slugs: Vec<&str> = ings.iter().map(|i| i.slug_hint.as_str()).collect();
        assert_eq!(slugs, vec!["beet", "rice", "olive-oil"]);
        assert_eq!(report.removed.len(), 2); // beef (vegan) + wheat-flour (gluten)
    }

    #[test]
    fn empty_constraints_noop() {
        let mut ings = vec![
            make_ing("beef", "meat"),
            make_ing("butter", "dairy"),
        ];
        let constraints = UserConstraints::default();
        let report = apply_dietary_constraints(&mut ings, &constraints);
        assert_eq!(ings.len(), 2);
        assert!(report.removed.is_empty());
    }

    #[test]
    fn substitution_suggestions() {
        let ing = make_ing("butter", "dairy");
        assert_eq!(suggest_substitution(&ing, "lactose"), Some("olive-oil".into()));

        let flour = make_ing("wheat-flour", "grain");
        assert_eq!(suggest_substitution(&flour, "gluten"), Some("rice-flour".into()));
    }
}
