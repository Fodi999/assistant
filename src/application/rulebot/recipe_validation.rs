//! Recipe Validation — post-build coherence checks for rulebot recipes.
//!
//! Runs AFTER the full TechCard is built. Catches logical issues like:
//!   - Recipe has no protein source
//!   - Too few ingredients (might be broken after constraint removal)
//!   - Steps reference ingredients that were removed
//!   - Calorie count is unreasonably low/high
//!
//! Returns `ValidationReport` with warnings (non-fatal) and errors (should block).

use super::recipe_engine::{TechCard, ResolvedIngredient, CookingStep};
use super::user_constraints::UserConstraints;
use super::intent_router::ChatLang;

/// Severity of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// A single validation issue.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

/// Full validation report for a TechCard.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Warning).collect()
    }

    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error).collect()
    }

    pub fn warning_messages(&self) -> Vec<String> {
        self.warnings().iter().map(|w| w.message.clone()).collect()
    }
}

/// Validate a completed TechCard for coherence.
pub fn validate_recipe(tech_card: &TechCard, constraints: &UserConstraints, lang: ChatLang) -> ValidationReport {
    let mut report = ValidationReport::default();

    check_minimum_ingredients(tech_card, lang, &mut report);
    check_has_protein(tech_card, constraints, lang, &mut report);
    check_calorie_range(tech_card, lang, &mut report);
    check_steps_not_empty(tech_card, lang, &mut report);
    check_unresolved_ratio(tech_card, lang, &mut report);
    check_total_weight(tech_card, lang, &mut report);

    report
}

// ── Individual checks ────────────────────────────────────────────────────────

/// Recipe should have at least 2 resolved ingredients.
fn check_minimum_ingredients(tc: &TechCard, lang: ChatLang, report: &mut ValidationReport) {
    let resolved_count = tc.ingredients.iter()
        .filter(|i| i.resolved_slug.is_some())
        .count();

    if resolved_count < 2 {
        let msg = match lang {
            ChatLang::Ru => format!("Только {} ингредиент(ов) — нужно минимум 2", resolved_count),
            ChatLang::En => format!("Only {} resolved ingredient(s) — needs at least 2", resolved_count),
            ChatLang::Pl => format!("Tylko {} składnik(ów) — potrzeba minimum 2", resolved_count),
            ChatLang::Uk => format!("Лише {} інгредієнт(ів) — потрібно мінімум 2", resolved_count),
        };
        report.issues.push(ValidationIssue {
            severity: Severity::Error,
            code: "TOO_FEW_INGREDIENTS",
            message: msg,
        });
    }
}

/// Non-vegan recipes should have a protein source (meat, fish, dairy, legume, egg).
/// Vegan recipes may use legumes/tofu as protein — warn if none found.
fn check_has_protein(tc: &TechCard, constraints: &UserConstraints, lang: ChatLang, report: &mut ValidationReport) {
    let has_protein_role = tc.ingredients.iter().any(|i| i.role == "protein");

    if !has_protein_role {
        let has_plant_protein = tc.ingredients.iter().any(|i| {
            let slug = slug_lower(i);
            slug.contains("tofu") || slug.contains("tempeh")
                || slug.contains("lentil") || slug.contains("chickpea")
                || slug.contains("bean") || slug.contains("edamame")
                || i.product.as_ref().map(|p| p.product_type.as_str()) == Some("legume")
        });

        if has_plant_protein {
            // OK — plant protein found, no issue
        } else if constraints.dietary_mode.is_some() {
            let msg = match lang {
                ChatLang::Ru => "Нет источника белка — диетические ограничения могли его убрать",
                ChatLang::En => "No protein source — dietary constraints may have removed it",
                ChatLang::Pl => "Brak źródła białka — ograniczenia dietetyczne mogły je usunąć",
                ChatLang::Uk => "Немає джерела білка — дієтичні обмеження могли його прибрати",
            };
            report.issues.push(ValidationIssue {
                severity: Severity::Warning,
                code: "NO_PROTEIN_DIETARY",
                message: msg.into(),
            });
        } else {
            let msg = match lang {
                ChatLang::Ru => "Нет источника белка — рекомендуется добавить мясо, рыбу или бобовые",
                ChatLang::En => "No protein source — consider adding meat, fish, or legumes",
                ChatLang::Pl => "Brak źródła białka — zalecamy dodać mięso, ryby lub rośliny strączkowe",
                ChatLang::Uk => "Немає джерела білка — рекомендуємо додати м'ясо, рибу або бобові",
            };
            report.issues.push(ValidationIssue {
                severity: Severity::Warning,
                code: "NO_PROTEIN",
                message: msg.into(),
            });
        }
    }
}

/// Per-serving calories should be in a reasonable range (50–1500 kcal).
fn check_calorie_range(tc: &TechCard, lang: ChatLang, report: &mut ValidationReport) {
    let per_serving = tc.per_serving_kcal;

    if per_serving < 50 && tc.ingredients.iter().any(|i| i.resolved_slug.is_some()) {
        let msg = match lang {
            ChatLang::Ru => format!("Калорийность подозрительно низкая: {} ккал/порция", per_serving),
            ChatLang::En => format!("Per-serving calories suspiciously low: {} kcal", per_serving),
            ChatLang::Pl => format!("Podejrzanie niska kaloryczność: {} kcal/porcja", per_serving),
            ChatLang::Uk => format!("Калорійність підозріло низька: {} ккал/порція", per_serving),
        };
        report.issues.push(ValidationIssue {
            severity: Severity::Warning,
            code: "KCAL_TOO_LOW",
            message: msg,
        });
    }

    if per_serving > 1500 {
        let msg = match lang {
            ChatLang::Ru => format!("Калорийность подозрительно высокая: {} ккал/порция", per_serving),
            ChatLang::En => format!("Per-serving calories suspiciously high: {} kcal", per_serving),
            ChatLang::Pl => format!("Podejrzanie wysoka kaloryczność: {} kcal/porcja", per_serving),
            ChatLang::Uk => format!("Калорійність підозріло висока: {} ккал/порція", per_serving),
        };
        report.issues.push(ValidationIssue {
            severity: Severity::Warning,
            code: "KCAL_TOO_HIGH",
            message: msg,
        });
    }
}

/// Recipe should have at least 1 step.
fn check_steps_not_empty(tc: &TechCard, lang: ChatLang, report: &mut ValidationReport) {
    if tc.steps.is_empty() {
        let msg = match lang {
            ChatLang::Ru => "Нет шагов приготовления",
            ChatLang::En => "Recipe has no cooking steps",
            ChatLang::Pl => "Brak kroków przygotowania",
            ChatLang::Uk => "Немає кроків приготування",
        };
        report.issues.push(ValidationIssue {
            severity: Severity::Warning,
            code: "NO_STEPS",
            message: msg.into(),
        });
    }
}

/// If more than half the ingredients are unresolved, that's suspicious.
fn check_unresolved_ratio(tc: &TechCard, lang: ChatLang, report: &mut ValidationReport) {
    let total = tc.ingredients.len();
    let unresolved = tc.unresolved.len();

    if total > 0 && unresolved as f32 / total as f32 > 0.5 {
        let msg = match lang {
            ChatLang::Ru => format!("{}/{} ингредиентов не найдены — рецепт может быть неточным", unresolved, total),
            ChatLang::En => format!("{}/{} ingredients unresolved — recipe may be inaccurate", unresolved, total),
            ChatLang::Pl => format!("{}/{} składników nieznalezionych — przepis może być niedokładny", unresolved, total),
            ChatLang::Uk => format!("{}/{} інгредієнтів не знайдено — рецепт може бути неточним", unresolved, total),
        };
        report.issues.push(ValidationIssue {
            severity: Severity::Warning,
            code: "HIGH_UNRESOLVED",
            message: msg,
        });
    }
}

/// Total output should be at least 100g (otherwise something is very wrong).
fn check_total_weight(tc: &TechCard, lang: ChatLang, report: &mut ValidationReport) {
    if tc.total_output_g < 100.0 && !tc.ingredients.is_empty() {
        let msg = match lang {
            ChatLang::Ru => format!("Общий выход всего {:.0}г — необычно мало", tc.total_output_g),
            ChatLang::En => format!("Total output is only {:.0}g — unusually low", tc.total_output_g),
            ChatLang::Pl => format!("Łączna masa to tylko {:.0}g — podejrzanie mało", tc.total_output_g),
            ChatLang::Uk => format!("Загальний вихід лише {:.0}г — незвично мало", tc.total_output_g),
        };
        report.issues.push(ValidationIssue {
            severity: Severity::Warning,
            code: "WEIGHT_TOO_LOW",
            message: msg,
        });
    }
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

    fn make_techcard(ingredients: Vec<(&str, &str, &str)>, kcal: u32) -> TechCard {
        use crate::infrastructure::ingredient_cache::IngredientData;

        let ings: Vec<ResolvedIngredient> = ingredients.iter().map(|(slug, role, ptype)| {
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
                    product_type: ptype.to_string(),
                    density_g_per_ml: None, behaviors: vec![], states: vec![],
                }),
                slug_hint: slug.to_string(),
                resolved_slug: Some(slug.to_string()),
                state: "raw".into(),
                role: role.to_string(),
                gross_g: 100.0,
                cleaned_net_g: 100.0,
                cooked_net_g: 100.0,
                kcal: kcal / ingredients.len().max(1) as u32,
                protein_g: 10.0,
                fat_g: 5.0,
                carbs_g: 10.0,
            }
        }).collect();

        TechCard {
            dish_name: "test".into(),
            dish_name_local: None,
            display_name: None,
            dish_type: "soup".into(),
            servings: 1,
            steps: vec![CookingStep {
                step: 1,
                text: "Cook".into(),
                time_min: Some(10),
                temp_c: None,
                tip: None,
            }],
            total_output_g: ings.iter().map(|i| i.cooked_net_g).sum(),
            total_gross_g: ings.iter().map(|i| i.gross_g).sum(),
            total_kcal: kcal,
            total_protein: 30.0,
            total_fat: 15.0,
            total_carbs: 30.0,
            per_serving_kcal: kcal,
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
            dish_image_url: None,
            ingredients: ings,
        }
    }

    #[test]
    fn valid_recipe_no_issues() {
        let tc = make_techcard(
            vec![
                ("chicken-breast", "protein", "meat"),
                ("beet", "side", "vegetable"),
                ("olive-oil", "oil", "oil"),
            ],
            450,
        );
        let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
        assert!(!report.has_errors());
        assert!(report.issues.is_empty());
    }

    #[test]
    fn too_few_ingredients() {
        let tc = make_techcard(vec![("salt", "spice", "spice")], 50);
        let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
        assert!(report.has_errors());
        assert!(report.errors().iter().any(|e| e.code == "TOO_FEW_INGREDIENTS"));
    }

    #[test]
    fn no_protein_warning() {
        let tc = make_techcard(
            vec![
                ("beet", "side", "vegetable"),
                ("potato", "side", "vegetable"),
                ("olive-oil", "oil", "oil"),
            ],
            300,
        );
        let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
        assert!(report.warnings().iter().any(|w| w.code == "NO_PROTEIN"));
    }

    #[test]
    fn vegan_no_protein_gives_dietary_warning() {
        use super::super::user_constraints::DietaryMode;

        let tc = make_techcard(
            vec![
                ("beet", "side", "vegetable"),
                ("potato", "side", "vegetable"),
            ],
            200,
        );
        let constraints = UserConstraints {
            dietary_mode: Some(DietaryMode::Vegan),
            ..Default::default()
        };
        let report = validate_recipe(&tc, &constraints, ChatLang::En);
        assert!(report.warnings().iter().any(|w| w.code == "NO_PROTEIN_DIETARY"));
    }

    #[test]
    fn high_kcal_warning() {
        let tc = make_techcard(
            vec![
                ("chicken-breast", "protein", "meat"),
                ("butter", "oil", "dairy"),
            ],
            2000,
        );
        let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
        assert!(report.warnings().iter().any(|w| w.code == "KCAL_TOO_HIGH"));
    }

    #[test]
    fn low_weight_warning() {
        let mut tc = make_techcard(
            vec![
                ("garlic", "spice", "vegetable"),
                ("salt", "spice", "spice"),
            ],
            50,
        );
        tc.total_output_g = 10.0; // override to be tiny
        tc.ingredients.iter_mut().for_each(|i| i.cooked_net_g = 5.0);
        let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
        assert!(report.warnings().iter().any(|w| w.code == "WEIGHT_TOO_LOW"));
    }

    #[test]
    fn high_unresolved_warning() {
        let mut tc = make_techcard(
            vec![
                ("beet", "side", "vegetable"),
                ("potato", "side", "vegetable"),
            ],
            300,
        );
        tc.unresolved = vec!["unknown1".into(), "unknown2".into(), "unknown3".into()];
        // 3 unresolved out of 5 total = 60%
        for slug in &tc.unresolved {
            tc.ingredients.push(ResolvedIngredient {
                product: None,
                slug_hint: slug.clone(),
                resolved_slug: None,
                state: "raw".into(),
                role: "other".into(),
                gross_g: 0.0, cleaned_net_g: 0.0, cooked_net_g: 0.0,
                kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
            });
        }
        let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
        assert!(report.warnings().iter().any(|w| w.code == "HIGH_UNRESOLVED"));
    }
}
