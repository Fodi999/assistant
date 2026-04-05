//! Recipe Rule Engine — domain layer
//!
//! Deterministic rule-based recipe diagnostics:
//! - Flavor balance rules (low acidity, missing fat, etc.)
//! - Nutrition rules (too many carbs, low protein, etc.)
//! - Ingredient dominance rules (one ingredient > 60%)
//! - Culinary structure rules (missing roles: acid, fat, aroma, etc.)
//!
//! Returns issues + actionable fixes with impact scores. No AI, fast and predictable.

use serde::Serialize;
use crate::domain::tools::flavor_graph::FlavorVector;
use crate::domain::tools::unit_converter as uc;

// ── Output types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct RuleIssue {
    /// Category: "flavor", "nutrition", "dominance", "structure"
    pub category: String,
    /// Severity: "info", "warning", "critical"
    pub severity: String,
    /// Machine-readable type
    pub rule: String,
    /// i18n title key
    pub title_key: String,
    /// i18n description key
    pub description_key: String,
    /// Suggested fix slugs (ingredients to add)
    pub fix_slugs: Vec<String>,
    /// Suggested fix description keys
    pub fix_keys: Vec<String>,
    /// Actual value that triggered the rule
    pub value: Option<f64>,
    /// Threshold that was violated
    pub threshold: Option<f64>,
    /// Estimated score improvement if fixed (0–20)
    pub impact: u8,
}

/// Per-category score breakdown (0–100 each)
#[derive(Debug, Serialize)]
pub struct CategoryScores {
    pub flavor: u8,
    pub nutrition: u8,
    pub dominance: u8,
    pub structure: u8,
}

#[derive(Debug, Serialize)]
pub struct RuleDiagnosis {
    pub health_score: u8,
    pub category_scores: CategoryScores,
    pub issues: Vec<RuleIssue>,
    pub critical_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

// ── Input types ──────────────────────────────────────────────────────────────

pub struct RecipeContext {
    pub flavor: FlavorVector,
    pub balance_score: u8,
    pub total_calories: f64,
    pub protein_pct: f64,
    pub fat_pct: f64,
    pub carbs_pct: f64,
    pub fiber_g: f64,
    pub sugar_g: f64,
    pub total_grams: f64,
    pub ingredients: Vec<(String, f64, Option<String>)>,
    /// Nutrition quality score from nutrition::nutrition_score() (0–100)
    pub nutrition_score: u8,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn sev_penalty(sev: &str) -> f64 {
    match sev { "critical" => 15.0, "warning" => 5.0, "info" => 2.0, _ => 0.0 }
}

fn sev_impact(sev: &str) -> u8 {
    match sev { "critical" => 15, "warning" => 7, "info" => 3, _ => 0 }
}

fn cat_score(issues: &[RuleIssue], cat: &str) -> u8 {
    let p: f64 = issues.iter().filter(|i| i.category == cat).map(|i| sev_penalty(&i.severity)).sum();
    ((100.0 - p).max(20.0).min(100.0)) as u8
}

// ── Rule Engine ──────────────────────────────────────────────────────────────

pub fn diagnose(ctx: &RecipeContext) -> RuleDiagnosis {
    let mut issues = Vec::new();
    check_flavor_rules(&ctx.flavor, &mut issues);
    check_nutrition_rules(ctx, &mut issues);
    check_dominance_rules(ctx, &mut issues);
    check_structure_rules(ctx, &mut issues);

    let critical_count = issues.iter().filter(|i| i.severity == "critical").count();
    let warning_count  = issues.iter().filter(|i| i.severity == "warning").count();
    let info_count     = issues.iter().filter(|i| i.severity == "info").count();

    let total_pen: f64 = issues.iter().map(|i| sev_penalty(&i.severity)).sum();
    let rule_compliance = ((100.0 - total_pen.min(80.0)).max(20.0)) as f64;

    // Composite health_score:  35% rule compliance + 35% nutrition + 30% flavor balance
    // This prevents "93 health_score but 27 nutrition" paradox.
    let nutrition = ctx.nutrition_score as f64;
    let balance   = ctx.balance_score as f64;
    let composite = rule_compliance * 0.35 + nutrition * 0.35 + balance * 0.30;
    let health_score = composite.clamp(20.0, 100.0).round() as u8;

    // Category scores: blend rule-based score with actual measured score
    let flavor_rule    = cat_score(&issues, "flavor") as f64;
    let nutrition_rule = cat_score(&issues, "nutrition") as f64;

    let category_scores = CategoryScores {
        // flavor: 50% rule-based + 50% actual balance_score
        flavor:    ((flavor_rule * 0.5 + balance * 0.5).clamp(20.0, 100.0).round()) as u8,
        // nutrition: 50% rule-based + 50% actual nutrition_score
        nutrition: ((nutrition_rule * 0.5 + nutrition * 0.5).clamp(20.0, 100.0).round()) as u8,
        dominance: cat_score(&issues, "dominance"),
        structure: cat_score(&issues, "structure"),
    };

    RuleDiagnosis { health_score, category_scores, issues, critical_count, warning_count, info_count }
}

// ── Flavor rules ─────────────────────────────────────────────────────────────

fn check_flavor_rules(flavor: &FlavorVector, issues: &mut Vec<RuleIssue>) {
    if flavor.acidity < 1.5 {
        let sev = if flavor.acidity < 0.5 { "critical" } else { "warning" };
        issues.push(RuleIssue {
            category: "flavor".into(), severity: sev.into(),
            rule: "low_acidity".into(),
            title_key: "rules.lowAcidity".into(),
            description_key: "rules.lowAcidityDesc".into(),
            fix_slugs: vec!["lemon".into(), "tomato".into(), "vinegar".into()],
            fix_keys: vec!["rules.fixAddAcid".into()],
            value: Some(uc::round_to(flavor.acidity, 2)), threshold: Some(1.5),
            impact: sev_impact(sev),
        });
    }

    if flavor.umami < 2.0 {
        let sev = if flavor.umami < 1.0 { "critical" } else { "warning" };
        issues.push(RuleIssue {
            category: "flavor".into(), severity: sev.into(),
            rule: "low_umami".into(),
            title_key: "rules.lowUmami".into(),
            description_key: "rules.lowUmamiDesc".into(),
            fix_slugs: vec!["hard-cheese".into(), "soy-sauce".into(), "tomato".into(), "button-mushroom".into()],
            fix_keys: vec!["rules.fixAddUmami".into()],
            value: Some(uc::round_to(flavor.umami, 2)), threshold: Some(2.0),
            impact: sev_impact(sev),
        });
    }

    if flavor.fat < 1.0 {
        issues.push(RuleIssue {
            category: "flavor".into(), severity: "warning".into(),
            rule: "low_fat".into(),
            title_key: "rules.lowFat".into(),
            description_key: "rules.lowFatDesc".into(),
            fix_slugs: vec!["butter".into(), "olive-oil".into(), "milk".into()],
            fix_keys: vec!["rules.fixAddFat".into()],
            value: Some(uc::round_to(flavor.fat, 2)), threshold: Some(1.0),
            impact: 7,
        });
    }

    if flavor.aroma < 2.0 {
        issues.push(RuleIssue {
            category: "flavor".into(), severity: "warning".into(),
            rule: "low_aroma".into(),
            title_key: "rules.lowAroma".into(),
            description_key: "rules.lowAromaDesc".into(),
            fix_slugs: vec!["garlic".into(), "basil".into(), "black-pepper".into(), "onion".into()],
            fix_keys: vec!["rules.fixAddAroma".into()],
            value: Some(uc::round_to(flavor.aroma, 2)), threshold: Some(2.0),
            impact: 7,
        });
    }

    // Info-level: low sweetness (often intentional in savory)
    if flavor.sweetness < 0.5 {
        issues.push(RuleIssue {
            category: "flavor".into(), severity: "info".into(),
            rule: "low_sweetness".into(),
            title_key: "rules.lowSweetness".into(),
            description_key: "rules.lowSweetnessDesc".into(),
            fix_slugs: vec!["honey".into(), "onion".into(), "carrot".into()],
            fix_keys: vec!["rules.fixAddSweet".into()],
            value: Some(uc::round_to(flavor.sweetness, 2)), threshold: Some(0.5),
            impact: 3,
        });
    }

    // Info-level: low bitterness
    if flavor.bitterness < 0.3 {
        issues.push(RuleIssue {
            category: "flavor".into(), severity: "info".into(),
            rule: "low_bitterness".into(),
            title_key: "rules.lowBitterness".into(),
            description_key: "rules.lowBitternessDesc".into(),
            fix_slugs: vec!["black-pepper".into(), "spinach".into(), "walnuts".into()],
            fix_keys: vec!["rules.fixAddBitter".into()],
            value: Some(uc::round_to(flavor.bitterness, 2)), threshold: Some(0.3),
            impact: 3,
        });
    }

    // Too much of one dimension
    for (name, val) in flavor.dimensions() {
        if val > 8.0 {
            issues.push(RuleIssue {
                category: "flavor".into(), severity: "info".into(),
                rule: format!("high_{}", name),
                title_key: format!("rules.high_{}", name),
                description_key: format!("rules.high_{}_desc", name),
                fix_slugs: vec![],
                fix_keys: vec![format!("rules.fixReduce_{}", name)],
                value: Some(uc::round_to(val, 2)), threshold: Some(8.0),
                impact: 3,
            });
        }
    }
}

// ── Nutrition rules ──────────────────────────────────────────────────────────

fn check_nutrition_rules(ctx: &RecipeContext, issues: &mut Vec<RuleIssue>) {
    if ctx.carbs_pct > 65.0 {
        let sev = if ctx.carbs_pct > 75.0 { "critical" } else { "warning" };
        issues.push(RuleIssue {
            category: "nutrition".into(), severity: sev.into(),
            rule: "high_carbs".into(),
            title_key: "rules.highCarbs".into(),
            description_key: "rules.highCarbsDesc".into(),
            fix_slugs: vec!["chicken-breast".into(), "salmon".into(), "chicken-eggs".into()],
            fix_keys: vec!["rules.fixReduceCarbs".into()],
            value: Some(uc::round_to(ctx.carbs_pct, 1)), threshold: Some(65.0),
            impact: sev_impact(sev),
        });
    }

    if ctx.protein_pct < 12.0 {
        let sev = if ctx.protein_pct < 8.0 { "critical" } else { "warning" };
        issues.push(RuleIssue {
            category: "nutrition".into(), severity: sev.into(),
            rule: "low_protein".into(),
            title_key: "rules.lowProtein".into(),
            description_key: "rules.lowProteinDesc".into(),
            fix_slugs: vec!["chicken-breast".into(), "chicken-eggs".into(), "hard-cheese".into(), "lentils".into()],
            fix_keys: vec!["rules.fixAddProtein".into()],
            value: Some(uc::round_to(ctx.protein_pct, 1)), threshold: Some(12.0),
            impact: sev_impact(sev),
        });
    }

    if ctx.fat_pct > 45.0 {
        issues.push(RuleIssue {
            category: "nutrition".into(), severity: "warning".into(),
            rule: "high_fat_ratio".into(),
            title_key: "rules.highFatRatio".into(),
            description_key: "rules.highFatRatioDesc".into(),
            fix_slugs: vec![],
            fix_keys: vec!["rules.fixReduceFat".into()],
            value: Some(uc::round_to(ctx.fat_pct, 1)), threshold: Some(45.0),
            impact: 7,
        });
    }

    if ctx.total_calories > 200.0 {
        let fiber_per_1000 = ctx.fiber_g / (ctx.total_calories / 1000.0);
        if fiber_per_1000 < 10.0 {
            let sev = if fiber_per_1000 < 5.0 { "warning" } else { "info" };
            issues.push(RuleIssue {
                category: "nutrition".into(), severity: sev.into(),
                rule: "low_fiber".into(),
                title_key: "rules.lowFiber".into(),
                description_key: "rules.lowFiberDesc".into(),
                fix_slugs: vec!["broccoli".into(), "lentils".into(), "oatmeal".into()],
                fix_keys: vec!["rules.fixAddFiber".into()],
                value: Some(uc::round_to(fiber_per_1000, 1)), threshold: Some(10.0),
                impact: sev_impact(sev),
            });
        }
    }

    if ctx.total_calories > 200.0 {
        let sugar_cal_pct = (ctx.sugar_g * 4.0 / ctx.total_calories) * 100.0;
        if sugar_cal_pct > 20.0 {
            let sev = if sugar_cal_pct > 35.0 { "critical" } else { "warning" };
            issues.push(RuleIssue {
                category: "nutrition".into(), severity: sev.into(),
                rule: "high_sugar".into(),
                title_key: "rules.highSugar".into(),
                description_key: "rules.highSugarDesc".into(),
                fix_slugs: vec![],
                fix_keys: vec!["rules.fixReduceSugar".into()],
                value: Some(uc::round_to(sugar_cal_pct, 1)), threshold: Some(20.0),
                impact: sev_impact(sev),
            });
        }
    }
}

// ── Dominance rules ──────────────────────────────────────────────────────────

fn check_dominance_rules(ctx: &RecipeContext, issues: &mut Vec<RuleIssue>) {
    if ctx.total_grams <= 0.0 || ctx.ingredients.len() < 2 { return; }

    for (_slug, grams, _pt) in &ctx.ingredients {
        let pct = grams / ctx.total_grams * 100.0;
        if pct > 60.0 {
            let sev = if pct > 75.0 { "critical" } else { "warning" };
            issues.push(RuleIssue {
                category: "dominance".into(), severity: sev.into(),
                rule: "ingredient_dominance".into(),
                title_key: "rules.dominance".into(),
                description_key: "rules.dominanceDesc".into(),
                fix_slugs: vec![],
                fix_keys: vec!["rules.fixAddVariety".into()],
                value: Some(uc::round_to(pct, 1)), threshold: Some(60.0),
                impact: sev_impact(sev),
            });
            break;
        }
    }
}

// ── Structure rules ──────────────────────────────────────────────────────────

fn check_structure_rules(ctx: &RecipeContext, issues: &mut Vec<RuleIssue>) {
    let types: Vec<&str> = ctx.ingredients.iter().filter_map(|(_, _, pt)| pt.as_deref()).collect();
    let has = |t: &str| types.iter().any(|&pt| pt == t);

    if !has("oil") && !has("fat") && !has("dairy") {
        issues.push(RuleIssue {
            category: "structure".into(), severity: "warning".into(),
            rule: "missing_fat_source".into(),
            title_key: "rules.missingFat".into(),
            description_key: "rules.missingFatDesc".into(),
            fix_slugs: vec!["olive-oil".into(), "butter".into()],
            fix_keys: vec!["rules.fixAddFatSource".into()],
            value: None, threshold: None, impact: 7,
        });
    }

    if !has("herb") && !has("spice") {
        issues.push(RuleIssue {
            category: "structure".into(), severity: "info".into(),
            rule: "missing_aromatics".into(),
            title_key: "rules.missingAromatics".into(),
            description_key: "rules.missingAromaticsDesc".into(),
            fix_slugs: vec!["garlic".into(), "basil".into(), "black-pepper".into()],
            fix_keys: vec!["rules.fixAddAromatics".into()],
            value: None, threshold: None, impact: 3,
        });
    }

    if !has("vegetable") && !has("fruit") {
        issues.push(RuleIssue {
            category: "structure".into(), severity: "warning".into(),
            rule: "missing_vegetables".into(),
            title_key: "rules.missingVegetables".into(),
            description_key: "rules.missingVegetablesDesc".into(),
            fix_slugs: vec!["tomato".into(), "onion".into(), "broccoli".into()],
            fix_keys: vec!["rules.fixAddVegetables".into()],
            value: None, threshold: None, impact: 7,
        });
    }

    let has_protein = has("meat") || has("fish") || has("seafood") || has("egg") || has("dairy") || has("legume");
    if !has_protein && has("grain") {
        issues.push(RuleIssue {
            category: "structure".into(), severity: "warning".into(),
            rule: "missing_protein_source".into(),
            title_key: "rules.missingProtein".into(),
            description_key: "rules.missingProteinDesc".into(),
            fix_slugs: vec!["chicken-breast".into(), "chicken-eggs".into(), "salmon".into()],
            fix_keys: vec!["rules.fixAddProteinSource".into()],
            value: None, threshold: None, impact: 7,
        });
    }

    let has_acid = types.iter().any(|&pt| pt == "fruit" || pt == "vegetable") && ctx.flavor.acidity > 2.0;
    if !has_acid && ctx.flavor.acidity < 2.0 {
        issues.push(RuleIssue {
            category: "structure".into(), severity: "info".into(),
            rule: "missing_acid_source".into(),
            title_key: "rules.missingAcid".into(),
            description_key: "rules.missingAcidDesc".into(),
            fix_slugs: vec!["lemon".into(), "tomato".into(), "vinegar".into()],
            fix_keys: vec!["rules.fixAddAcidSource".into()],
            value: None, threshold: None, impact: 3,
        });
    }
}
