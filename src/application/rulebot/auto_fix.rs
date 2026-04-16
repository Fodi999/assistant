//! Auto-Fix Engine — automatically repair validation issues in a TechCard.
//!
//! Pipeline position:
//!   validation → **auto_fix** → revalidation → final TechCard
//!
//! Takes `ValidationReport` + `TechCard` + `GoalProfile` and attempts to fix:
//!   - NO_PROTEIN → insert dish-appropriate protein (beans for soup, chicken for salad, etc.)
//!   - KCAL_TOO_LOW → increase portions proportionally
//!   - KCAL_TOO_HIGH → reduce fat/oil/condiment portions
//!   - NO_STEPS → generate basic steps
//!   - WEIGHT_TOO_LOW → double side/base portions
//!
//! All messages are localized via lang parameter.
//! Every fix is logged for the UI "explain" block.

use serde::Serialize;
use super::recipe_engine::{TechCard, ResolvedIngredient, CookingStep, DishType};
use super::recipe_validation::{ValidationReport, Severity};
use super::goal_engine::{GoalProfile, GoalStrategy};
use super::nutrition_math::round1;
use super::intent_router::ChatLang;

// ── Fix Report ───────────────────────────────────────────────────────────────

/// A single auto-fix action.
#[derive(Debug, Clone, Serialize)]
pub struct FixAction {
    /// The validation code that triggered this fix
    pub trigger: String,
    /// What was done
    pub action: String,
    /// Human-readable detail
    pub detail: String,
}

/// Full report of auto-fixes applied.
#[derive(Debug, Clone, Default, Serialize)]
pub struct FixReport {
    pub fixes: Vec<FixAction>,
}

impl FixReport {
    pub fn is_empty(&self) -> bool {
        self.fixes.is_empty()
    }

    pub fn messages(&self) -> Vec<String> {
        self.fixes.iter().map(|f| format!("{}: {}", f.action, f.detail)).collect()
    }
}

// ── Main Entry Point ─────────────────────────────────────────────────────────

/// Attempt to auto-fix validation issues in the TechCard.
/// Modifies the TechCard in-place and returns a report.
/// `goal` controls calorie-aware decisions (e.g. weight loss → don't add high-kcal protein).
/// `lang` controls localized fix descriptions.
pub fn auto_fix(
    tech_card: &mut TechCard,
    validation: &ValidationReport,
    goal: &GoalProfile,
    lang: ChatLang,
) -> FixReport {
    let mut report = FixReport::default();
    let dish_type = DishType::detect(&tech_card.dish_name);

    for issue in &validation.issues {
        match issue.code {
            "NO_PROTEIN" | "NO_PROTEIN_DIETARY" => {
                fix_no_protein(tech_card, dish_type, goal, lang, &mut report);
            }
            "KCAL_TOO_LOW" => {
                fix_kcal_too_low(tech_card, goal, lang, &mut report);
            }
            "KCAL_TOO_HIGH" => {
                fix_kcal_too_high(tech_card, goal, lang, &mut report);
            }
            "NO_STEPS" => {
                fix_no_steps(tech_card, lang, &mut report);
            }
            "WEIGHT_TOO_LOW" => {
                fix_weight_too_low(tech_card, lang, &mut report);
            }
            _ => {
                // TOO_FEW_INGREDIENTS, HIGH_UNRESOLVED — can't auto-fix
            }
        }
    }

    // Recalculate totals after fixes
    if !report.is_empty() {
        recalculate_totals(tech_card);
    }

    report
}

// ── Individual Fixes ─────────────────────────────────────────────────────────

// ── Protein source selection table ───────────────────────────────────────────

/// Pick the best protein source for the dish type + goal.
/// Returns (slug, state, grams, kcal, protein_g, fat_g, carbs_g).
fn pick_protein(dish_type: DishType, goal: &GoalProfile) -> (&'static str, &'static str, f32, u32, f32, f32, f32) {
    let is_low_cal = matches!(goal.strategy, GoalStrategy::ReduceCalories);

    match dish_type {
        // Soups & stews: legumes first (culinary logic), then chicken
        DishType::Soup | DishType::Stew => {
            if is_low_cal {
                // Lentils: low cal, high protein, perfect for soup
                ("lentils", "boiled", 80.0, 93, 7.2, 0.3, 16.0)
            } else {
                // White beans: hearty, classic in soup
                ("white-beans", "boiled", 80.0, 106, 7.5, 0.4, 19.2)
            }
        }
        // Salads: chicken breast or chickpeas
        DishType::Salad | DishType::Raw => {
            if is_low_cal {
                ("chickpeas", "boiled", 80.0, 131, 7.1, 2.1, 21.8)
            } else {
                ("chicken-breast", "grilled", 100.0, 165, 31.0, 3.6, 0.0)
            }
        }
        // Stir-fry / wok: tofu for low-cal, chicken otherwise
        DishType::StirFry => {
            if is_low_cal {
                ("tofu", "sauteed", 100.0, 76, 8.1, 4.8, 1.9)
            } else {
                ("chicken-breast", "sauteed", 100.0, 165, 31.0, 3.6, 0.0)
            }
        }
        // Grill / bake / pasta / default: eggs as universal fallback
        _ => {
            if is_low_cal {
                ("chickpeas", "boiled", 80.0, 131, 7.1, 2.1, 21.8)
            } else {
                ("eggs", "boiled", 120.0, 186, 15.6, 12.0, 1.2)
            }
        }
    }
}

/// Target-driven protein fix: add dish-appropriate protein and scale until
/// per-serving protein reaches `goal.protein_g.start`.
/// This is a `while protein < target` loop — not a one-shot add.
fn fix_no_protein(
    tc: &mut TechCard,
    dish_type: DishType,
    goal: &GoalProfile,
    lang: ChatLang,
    report: &mut FixReport,
) {
    // Don't double-add if adaptation_engine already added a protein
    let has_protein = tc.ingredients.iter().any(|i| i.role == "protein");

    let has_legume = tc.ingredients.iter().any(|i| {
        let slug = i.resolved_slug.as_deref().unwrap_or(&i.slug_hint).to_lowercase();
        slug.contains("chickpea") || slug.contains("bean") || slug.contains("lentil")
            || slug.contains("tofu") || slug.contains("tempeh")
    });

    // ── Step 1: Insert protein source if none exists ─────────────────────
    if !has_protein && !has_legume {
        let (slug, state, grams, kcal, protein_g, fat_g, carbs_g) = pick_protein(dish_type, goal);

        tc.ingredients.push(ResolvedIngredient {
            product: None,
            slug_hint: slug.into(),
            resolved_slug: None,
            state: state.into(),
            role: "protein".into(),
            gross_g: grams,
            cleaned_net_g: grams,
            cooked_net_g: grams,
            kcal,
            protein_g,
            fat_g,
            carbs_g,
        });

        let detail = match lang {
            ChatLang::Ru => format!("{} ({:.0}г) как источник белка", slug, grams),
            ChatLang::En => format!("{} ({:.0}g) as protein source", slug, grams),
            ChatLang::Pl => format!("{} ({:.0}g) jako źródło białka", slug, grams),
            ChatLang::Uk => format!("{} ({:.0}г) як джерело білка", slug, grams),
        };
        let action = match lang {
            ChatLang::Ru => "Добавлено",
            ChatLang::En => "Added",
            ChatLang::Pl => "Dodano",
            ChatLang::Uk => "Додано",
        };
        report.fixes.push(FixAction {
            trigger: "NO_PROTEIN".into(),
            action: action.into(),
            detail,
        });
    }

    // ── Step 2: Scale protein ingredients until target is reached ────────
    // target-driven: while per_serving_protein < goal.protein_g.start → scale up
    let servings = tc.servings.max(1) as f32;
    let target = goal.protein_g.start;

    // Max 3 iterations to avoid infinite loops
    for _ in 0..3 {
        let current_protein: f32 = tc.ingredients.iter().map(|i| i.protein_g).sum::<f32>() / servings;
        if current_protein >= target { break; }

        // Scale all protein-role ingredients by the deficit ratio (capped at 2x per step)
        let deficit_ratio = (target / current_protein.max(1.0)).min(2.0);
        let mut scaled = Vec::new();

        for ing in tc.ingredients.iter_mut() {
            if ing.role == "protein" || {
                let s = ing.resolved_slug.as_deref().unwrap_or(&ing.slug_hint).to_lowercase();
                s.contains("chickpea") || s.contains("bean") || s.contains("lentil")
                    || s.contains("tofu") || s.contains("tempeh")
            } {
                let old_g = ing.gross_g;
                scale_ingredient(ing, deficit_ratio);
                scaled.push(format!("{}: {:.0}g → {:.0}g", ing.slug_hint, old_g, ing.gross_g));
            }
        }

        if scaled.is_empty() { break; }

        let action = match lang {
            ChatLang::Ru => "Увеличен белок",
            ChatLang::En => "Boosted protein",
            ChatLang::Pl => "Zwiększono białko",
            ChatLang::Uk => "Збільшено білок",
        };
        let detail = match lang {
            ChatLang::Ru => format!("до {:.0}г/порцию (цель: {:.0}г): {}", current_protein * deficit_ratio, target, scaled.join(", ")),
            ChatLang::En => format!("to {:.0}g/serv (target: {:.0}g): {}", current_protein * deficit_ratio, target, scaled.join(", ")),
            ChatLang::Pl => format!("do {:.0}g/porcja (cel: {:.0}g): {}", current_protein * deficit_ratio, target, scaled.join(", ")),
            ChatLang::Uk => format!("до {:.0}г/порцію (ціль: {:.0}г): {}", current_protein * deficit_ratio, target, scaled.join(", ")),
        };
        report.fixes.push(FixAction {
            trigger: "LOW_PROTEIN".into(),
            action: action.into(),
            detail,
        });
    }
}

/// Increase all portions proportionally if kcal is too low.
fn fix_kcal_too_low(tc: &mut TechCard, _goal: &GoalProfile, lang: ChatLang, report: &mut FixReport) {
    if tc.per_serving_kcal >= 50 { return; }

    let factor = 1.5_f32;
    for ing in tc.ingredients.iter_mut() {
        if ing.role == "spice" || ing.role == "oil" || ing.role == "condiment" {
            continue;
        }
        scale_ingredient(ing, factor);
    }

    let detail = match lang {
        ChatLang::Ru => "все основные ингредиенты +50%".into(),
        ChatLang::En => "all main ingredients +50%".into(),
        ChatLang::Pl => "wszystkie główne składniki +50%".into(),
        ChatLang::Uk => "всі основні інгредієнти +50%".into(),
    };
    let action = match lang {
        ChatLang::Ru => "Увеличены порции",
        ChatLang::En => "Increased portions",
        ChatLang::Pl => "Zwiększono porcje",
        ChatLang::Uk => "Збільшено порції",
    };

    report.fixes.push(FixAction {
        trigger: "KCAL_TOO_LOW".into(),
        action: action.into(),
        detail,
    });
}

/// Reduce fat/oil if kcal is too high.
/// For weight-loss goals, use a stricter threshold from the GoalProfile.
fn fix_kcal_too_high(tc: &mut TechCard, goal: &GoalProfile, lang: ChatLang, report: &mut FixReport) {
    let threshold = match goal.strategy {
        GoalStrategy::ReduceCalories => goal.kcal.end as u32, // e.g. 500
        _ => 1500,
    };
    if tc.per_serving_kcal <= threshold { return; }

    let factor = match goal.strategy {
        GoalStrategy::ReduceCalories => 0.3_f32, // aggressive for weight loss
        _ => 0.5,
    };

    let mut reduced = Vec::new();

    for ing in tc.ingredients.iter_mut() {
        if ing.role == "oil" || ing.role == "condiment" {
            let old = ing.gross_g;
            scale_ingredient(ing, factor);
            reduced.push(format!("{}: {:.0}g → {:.0}g", ing.slug_hint, old, ing.gross_g));
        }
    }

    if !reduced.is_empty() {
        let action = match lang {
            ChatLang::Ru => "Уменьшены жиры",
            ChatLang::En => "Reduced fats",
            ChatLang::Pl => "Zmniejszono tłuszcze",
            ChatLang::Uk => "Зменшено жири",
        };
        report.fixes.push(FixAction {
            trigger: "KCAL_TOO_HIGH".into(),
            action: action.into(),
            detail: reduced.join(", "),
        });
    }
}

/// Generate minimal steps if none exist.
fn fix_no_steps(tc: &mut TechCard, lang: ChatLang, report: &mut FixReport) {
    if !tc.steps.is_empty() { return; }

    let ingredients_list: String = tc.ingredients.iter()
        .filter(|i| i.role != "spice" && i.role != "oil" && i.role != "condiment")
        .map(|i| i.slug_hint.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let (step1_text, step2_text) = match lang {
        ChatLang::Ru => (
            format!("Подготовьте: {}", ingredients_list),
            "Готовьте до готовности, приправьте по вкусу.".to_string(),
        ),
        ChatLang::En => (
            format!("Prepare: {}", ingredients_list),
            "Cook until ready, season to taste.".to_string(),
        ),
        ChatLang::Pl => (
            format!("Przygotuj: {}", ingredients_list),
            "Gotuj do miękkości, dopraw do smaku.".to_string(),
        ),
        ChatLang::Uk => (
            format!("Підготуйте: {}", ingredients_list),
            "Готуйте до готовності, приправте за смаком.".to_string(),
        ),
    };

    tc.steps.push(CookingStep {
        step: 1,
        text: step1_text,
        time_min: Some(10),
        temp_c: None,
        tip: None,
    });
    tc.steps.push(CookingStep {
        step: 2,
        text: step2_text,
        time_min: Some(20),
        temp_c: None,
        tip: None,
    });

    let action = match lang {
        ChatLang::Ru => "Сгенерированы шаги",
        ChatLang::En => "Generated basic steps",
        ChatLang::Pl => "Wygenerowano kroki",
        ChatLang::Uk => "Згенеровано кроки",
    };
    let detail = match lang {
        ChatLang::Ru => "добавлено 2 базовых шага",
        ChatLang::En => "2 generic steps added",
        ChatLang::Pl => "dodano 2 podstawowe kroki",
        ChatLang::Uk => "додано 2 базових кроки",
    };

    report.fixes.push(FixAction {
        trigger: "NO_STEPS".into(),
        action: action.into(),
        detail: detail.into(),
    });
}

/// If total weight is unreasonably low, increase side portions.
fn fix_weight_too_low(tc: &mut TechCard, lang: ChatLang, report: &mut FixReport) {
    if tc.total_output_g >= 100.0 { return; }

    let factor = 2.0_f32;
    for ing in tc.ingredients.iter_mut() {
        if ing.role == "side" || ing.role == "base" {
            scale_ingredient(ing, factor);
        }
    }

    let action = match lang {
        ChatLang::Ru => "Увеличены гарниры",
        ChatLang::En => "Doubled side portions",
        ChatLang::Pl => "Zwiększono dodatki",
        ChatLang::Uk => "Збільшено гарніри",
    };

    report.fixes.push(FixAction {
        trigger: "WEIGHT_TOO_LOW".into(),
        action: action.into(),
        detail: format!("{:.0}g", tc.total_output_g),
    });
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

// ── Recalculate ──────────────────────────────────────────────────────────────

/// Recalculate TechCard totals after modifications.
fn recalculate_totals(tc: &mut TechCard) {
    tc.total_gross_g = tc.ingredients.iter().map(|i| i.gross_g).sum();
    tc.total_output_g = tc.ingredients.iter().map(|i| i.cooked_net_g).sum();
    tc.total_kcal = tc.ingredients.iter().map(|i| i.kcal).sum();
    tc.total_protein = tc.ingredients.iter().map(|i| i.protein_g).sum();
    tc.total_fat = tc.ingredients.iter().map(|i| i.fat_g).sum();
    tc.total_carbs = tc.ingredients.iter().map(|i| i.carbs_g).sum();

    let s = tc.servings.max(1) as f32;
    tc.per_serving_kcal = (tc.total_kcal as f32 / s).round() as u32;
    tc.per_serving_protein = round1(tc.total_protein / s);
    tc.per_serving_fat = round1(tc.total_fat / s);
    tc.per_serving_carbs = round1(tc.total_carbs / s);
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::recipe_validation::{ValidationIssue, Severity};
    use crate::infrastructure::ingredient_cache::IngredientData;

    use super::super::goal_engine::profile_for;
    use super::super::goal_modifier::HealthModifier;

    fn balanced() -> GoalProfile { profile_for(HealthModifier::None) }
    fn low_cal()  -> GoalProfile { profile_for(HealthModifier::LowCalorie) }

    fn make_techcard(ings: Vec<(&str, &str, f32, u32)>) -> TechCard {
        let ingredients: Vec<ResolvedIngredient> = ings.iter().map(|(slug, role, grams, kcal)| {
            ResolvedIngredient {
                product: Some(IngredientData {
                    slug: slug.to_string(),
                    name_en: slug.to_string(),
                    name_ru: slug.to_string(),
                    name_pl: slug.to_string(),
                    name_uk: slug.to_string(),
                    calories_per_100g: 100.0,
                    protein_per_100g: 10.0,
                    fat_per_100g: 5.0,
                    carbs_per_100g: 10.0,
                    image_url: None,
                    product_type: "other".into(),
                    density_g_per_ml: None, behaviors: vec![],
                }),
                slug_hint: slug.to_string(),
                resolved_slug: Some(slug.to_string()),
                state: "raw".into(),
                role: role.to_string(),
                gross_g: *grams,
                cleaned_net_g: *grams,
                cooked_net_g: *grams,
                kcal: *kcal,
                protein_g: 10.0,
                fat_g: 5.0,
                carbs_g: 10.0,
            }
        }).collect();

        let total_output: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
        let total_kcal: u32 = ingredients.iter().map(|i| i.kcal).sum();

        TechCard {
            dish_name: "test".into(),
            dish_name_local: None,
            display_name: None,
            dish_type: "default".into(),
            servings: 1,
            steps: vec![CookingStep { step: 1, text: "Cook".into(), time_min: Some(10), temp_c: None, tip: None }],
            total_output_g: total_output,
            total_gross_g: total_output,
            total_kcal,
            total_protein: 30.0,
            total_fat: 15.0,
            total_carbs: 30.0,
            per_serving_kcal: total_kcal,
            per_serving_protein: 30.0,
            per_serving_fat: 15.0,
            per_serving_carbs: 30.0,
            unresolved: vec![],
            removed_ingredients: vec![],
            complexity: "easy".into(),
            goal: "balanced".into(),
            allergens: vec![],
            tags: vec![],
            applied_constraints: vec![],
            adaptations: vec![],
            validation_warnings: vec![],
            auto_fixes: vec![], flavor_analysis: None,
            ingredients,
        }
    }

    #[test]
    fn fix_no_protein_adds_legumes_for_soup() {
        let mut tc = make_techcard(vec![
            ("beet", "side", 100.0, 40),
            ("potato", "side", 100.0, 80),
        ]);
        tc.dish_name = "tomato soup".into();
        let validation = ValidationReport {
            issues: vec![ValidationIssue {
                severity: Severity::Warning,
                code: "NO_PROTEIN",
                message: "no protein".into(),
            }],
        };

        let report = auto_fix(&mut tc, &validation, &balanced(), ChatLang::En);
        // Soup should get beans/lentils, NOT eggs
        let protein_slug = tc.ingredients.iter()
            .find(|i| i.role == "protein")
            .map(|i| i.slug_hint.as_str());
        assert!(
            protein_slug == Some("white-beans") || protein_slug == Some("lentils"),
            "soup should get legumes, got {:?}", protein_slug
        );
        assert!(!report.is_empty());
    }

    #[test]
    fn fix_no_protein_low_cal_soup_gets_lentils() {
        let mut tc = make_techcard(vec![
            ("beet", "side", 100.0, 40),
            ("potato", "side", 100.0, 80),
        ]);
        tc.dish_name = "tomato soup".into();
        let validation = ValidationReport {
            issues: vec![ValidationIssue {
                severity: Severity::Warning,
                code: "NO_PROTEIN",
                message: "no protein".into(),
            }],
        };

        let report = auto_fix(&mut tc, &validation, &low_cal(), ChatLang::Ru);
        let protein_slug = tc.ingredients.iter()
            .find(|i| i.role == "protein")
            .map(|i| i.slug_hint.as_str());
        assert_eq!(protein_slug, Some("lentils"), "low-cal soup → lentils");
        assert!(!report.is_empty());
    }

    #[test]
    fn fix_kcal_too_high_reduces_oil() {
        let mut tc = make_techcard(vec![
            ("chicken", "protein", 100.0, 800),
            ("olive-oil", "oil", 50.0, 442),
            ("butter", "condiment", 30.0, 300),
        ]);
        tc.per_serving_kcal = 1542;

        let validation = ValidationReport {
            issues: vec![ValidationIssue {
                severity: Severity::Warning,
                code: "KCAL_TOO_HIGH",
                message: "too high".into(),
            }],
        };

        let oil_before = tc.ingredients.iter().find(|i| i.slug_hint == "olive-oil").unwrap().gross_g;
        let report = auto_fix(&mut tc, &validation, &balanced(), ChatLang::En);
        let oil_after = tc.ingredients.iter().find(|i| i.slug_hint == "olive-oil").unwrap().gross_g;

        assert!(oil_after < oil_before, "oil should be reduced");
        assert!(!report.is_empty());
    }

    #[test]
    fn fix_no_steps_generates_basic() {
        let mut tc = make_techcard(vec![
            ("chicken", "protein", 100.0, 200),
        ]);
        tc.steps.clear();

        let validation = ValidationReport {
            issues: vec![ValidationIssue {
                severity: Severity::Warning,
                code: "NO_STEPS",
                message: "no steps".into(),
            }],
        };

        auto_fix(&mut tc, &validation, &balanced(), ChatLang::En);
        assert!(tc.steps.len() >= 2);
    }

    #[test]
    fn no_fixes_for_valid_recipe() {
        let mut tc = make_techcard(vec![
            ("chicken", "protein", 100.0, 200),
            ("beet", "side", 100.0, 40),
        ]);
        let validation = ValidationReport { issues: vec![] };

        let report = auto_fix(&mut tc, &validation, &balanced(), ChatLang::En);
        assert!(report.is_empty());
    }

    #[test]
    fn recalculate_after_fix() {
        let mut tc = make_techcard(vec![
            ("beet", "side", 50.0, 20),
            ("potato", "side", 50.0, 40),
        ]);
        let old_kcal = tc.total_kcal;

        let validation = ValidationReport {
            issues: vec![ValidationIssue {
                severity: Severity::Warning,
                code: "NO_PROTEIN",
                message: "no protein".into(),
            }],
        };

        auto_fix(&mut tc, &validation, &balanced(), ChatLang::En);
        assert!(tc.total_kcal > old_kcal, "total kcal should increase after adding protein");
    }

    #[test]
    fn target_driven_protein_scales_up() {
        // High-protein goal requires 40g+ protein per serving.
        // Default lentils add ~7g — not enough. System should scale up.
        let high_protein = profile_for(HealthModifier::HighProtein);

        let mut tc = make_techcard(vec![
            ("beet", "side", 100.0, 40),
            ("potato", "side", 100.0, 80),
        ]);
        tc.dish_name = "tomato soup".into();
        tc.servings = 1;
        // protein_g per ingredient = 10.0 by default → total = 20g
        // high_protein goal wants 40g+, so system must add + scale protein

        let validation = ValidationReport {
            issues: vec![ValidationIssue {
                severity: Severity::Warning,
                code: "NO_PROTEIN",
                message: "no protein".into(),
            }],
        };

        let report = auto_fix(&mut tc, &validation, &high_protein, ChatLang::En);
        recalculate_totals(&mut tc);
        let per_serving_protein = tc.total_protein / tc.servings.max(1) as f32;

        // Should have added protein AND scaled it up toward target
        assert!(per_serving_protein >= 35.0,
            "high-protein goal should drive protein up, got {:.1}g", per_serving_protein);
        // Should have at least 2 fix actions: add + scale
        assert!(report.fixes.len() >= 2,
            "expected add + scale fixes, got {} fixes", report.fixes.len());
    }
}
