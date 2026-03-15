//! Recipe Rule Engine — domain layer
//!
//! Deterministic rule-based recipe diagnostics:
//! - Flavor balance rules (low acidity, missing fat, etc.)
//! - Nutrition rules (too many carbs, low protein, etc.)
//! - Ingredient dominance rules (one ingredient > 60%)
//! - Culinary structure rules (missing roles: acid, fat, aroma, etc.)
//!
//! Returns issues + actionable fixes. No AI, fast and predictable.

use serde::Serialize;
use crate::domain::tools::flavor_graph::FlavorVector;
use crate::domain::tools::unit_converter as uc;

// ── Output types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct RuleIssue {
    /// Category: "flavor", "nutrition", "dominance", "structure"
    pub category: String,
    /// Severity: "warning", "critical"
    pub severity: String,
    /// Machine-readable type: "low_acidity", "high_carbs", etc.
    pub rule: String,
    /// Human-readable title key (for i18n on frontend)
    pub title_key: String,
    /// Human-readable description key
    pub description_key: String,
    /// Suggested fix slugs (ingredients to add)
    pub fix_slugs: Vec<String>,
    /// Suggested fix descriptions (keys for i18n)
    pub fix_keys: Vec<String>,
    /// Numeric context (e.g., the actual value that triggered the rule)
    pub value: Option<f64>,
    /// Threshold that was violated
    pub threshold: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct RuleDiagnosis {
    /// Overall recipe health score (0-100)
    pub health_score: u8,
    /// List of issues found
    pub issues: Vec<RuleIssue>,
    /// Number of critical issues
    pub critical_count: usize,
    /// Number of warnings
    pub warning_count: usize,
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
    /// (slug, grams, product_type)
    pub ingredients: Vec<(String, f64, Option<String>)>,
}

// ── Rule Engine ──────────────────────────────────────────────────────────────

pub fn diagnose(ctx: &RecipeContext) -> RuleDiagnosis {
    let mut issues = Vec::new();

    // ── 1. Flavor balance rules ──────────────────────────────────────────
    check_flavor_rules(&ctx.flavor, &mut issues);

    // ── 2. Nutrition rules ───────────────────────────────────────────────
    check_nutrition_rules(ctx, &mut issues);

    // ── 3. Ingredient dominance rules ────────────────────────────────────
    check_dominance_rules(ctx, &mut issues);

    // ── 4. Culinary structure rules ──────────────────────────────────────
    check_structure_rules(ctx, &mut issues);

    // ── Calculate health score ───────────────────────────────────────────
    let critical_count = issues.iter().filter(|i| i.severity == "critical").count();
    let warning_count = issues.iter().filter(|i| i.severity == "warning").count();

    // Start at 100, subtract penalties
    let penalty = (critical_count as f64 * 15.0 + warning_count as f64 * 5.0).min(80.0);
    let health_score = ((100.0 - penalty).max(20.0)) as u8;

    RuleDiagnosis {
        health_score,
        issues,
        critical_count,
        warning_count,
    }
}

// ── Flavor rules ─────────────────────────────────────────────────────────────

fn check_flavor_rules(flavor: &FlavorVector, issues: &mut Vec<RuleIssue>) {
    // Low acidity
    if flavor.acidity < 1.5 {
        issues.push(RuleIssue {
            category: "flavor".into(),
            severity: if flavor.acidity < 0.5 { "critical" } else { "warning" }.into(),
            rule: "low_acidity".into(),
            title_key: "rules.lowAcidity".into(),
            description_key: "rules.lowAcidityDesc".into(),
            fix_slugs: vec!["lemon".into(), "tomato".into(), "white-wine-vinegar".into()],
            fix_keys: vec!["rules.fixAddAcid".into()],
            value: Some(uc::round_to(flavor.acidity, 2)),
            threshold: Some(1.5),
        });
    }

    // Low umami
    if flavor.umami < 2.0 {
        issues.push(RuleIssue {
            category: "flavor".into(),
            severity: if flavor.umami < 1.0 { "critical" } else { "warning" }.into(),
            rule: "low_umami".into(),
            title_key: "rules.lowUmami".into(),
            description_key: "rules.lowUmamiDesc".into(),
            fix_slugs: vec!["hard-cheese".into(), "soy-sauce".into(), "tomato".into(), "mushrooms".into()],
            fix_keys: vec!["rules.fixAddUmami".into()],
            value: Some(uc::round_to(flavor.umami, 2)),
            threshold: Some(2.0),
        });
    }

    // Low fat / richness
    if flavor.fat < 1.0 {
        issues.push(RuleIssue {
            category: "flavor".into(),
            severity: "warning".into(),
            rule: "low_fat".into(),
            title_key: "rules.lowFat".into(),
            description_key: "rules.lowFatDesc".into(),
            fix_slugs: vec!["butter".into(), "olive-oil".into(), "cream".into()],
            fix_keys: vec!["rules.fixAddFat".into()],
            value: Some(uc::round_to(flavor.fat, 2)),
            threshold: Some(1.0),
        });
    }

    // Low aroma
    if flavor.aroma < 2.0 {
        issues.push(RuleIssue {
            category: "flavor".into(),
            severity: "warning".into(),
            rule: "low_aroma".into(),
            title_key: "rules.lowAroma".into(),
            description_key: "rules.lowAromaDesc".into(),
            fix_slugs: vec!["garlic".into(), "basil".into(), "black-pepper".into(), "onion".into()],
            fix_keys: vec!["rules.fixAddAroma".into()],
            value: Some(uc::round_to(flavor.aroma, 2)),
            threshold: Some(2.0),
        });
    }

    // Low sweetness (only flag if very low and no sweetener present)
    if flavor.sweetness < 0.5 {
        issues.push(RuleIssue {
            category: "flavor".into(),
            severity: "warning".into(),
            rule: "low_sweetness".into(),
            title_key: "rules.lowSweetness".into(),
            description_key: "rules.lowSweetnessDesc".into(),
            fix_slugs: vec!["honey".into(), "onion".into(), "carrot".into()],
            fix_keys: vec!["rules.fixAddSweet".into()],
            value: Some(uc::round_to(flavor.sweetness, 2)),
            threshold: Some(0.5),
        });
    }

    // Low bitterness is usually fine — skip unless extreme
    if flavor.bitterness < 0.3 {
        issues.push(RuleIssue {
            category: "flavor".into(),
            severity: "warning".into(),
            rule: "low_bitterness".into(),
            title_key: "rules.lowBitterness".into(),
            description_key: "rules.lowBitternessDesc".into(),
            fix_slugs: vec!["black-pepper".into(), "arugula".into(), "coffee".into()],
            fix_keys: vec!["rules.fixAddBitter".into()],
            value: Some(uc::round_to(flavor.bitterness, 2)),
            threshold: Some(0.3),
        });
    }

    // Too much of one dimension (> 8.0)
    for (name, val) in flavor.dimensions() {
        if val > 8.0 {
            issues.push(RuleIssue {
                category: "flavor".into(),
                severity: "warning".into(),
                rule: format!("high_{}", name),
                title_key: format!("rules.high_{}", name),
                description_key: format!("rules.high_{}_desc", name),
                fix_slugs: vec![],
                fix_keys: vec![format!("rules.fixReduce_{}", name)],
                value: Some(uc::round_to(val, 2)),
                threshold: Some(8.0),
            });
        }
    }
}

// ── Nutrition rules ──────────────────────────────────────────────────────────

fn check_nutrition_rules(ctx: &RecipeContext, issues: &mut Vec<RuleIssue>) {
    // High carbs
    if ctx.carbs_pct > 65.0 {
        issues.push(RuleIssue {
            category: "nutrition".into(),
            severity: if ctx.carbs_pct > 75.0 { "critical" } else { "warning" }.into(),
            rule: "high_carbs".into(),
            title_key: "rules.highCarbs".into(),
            description_key: "rules.highCarbsDesc".into(),
            fix_slugs: vec!["chicken-breast".into(), "salmon".into(), "egg".into()],
            fix_keys: vec!["rules.fixReduceCarbs".into()],
            value: Some(uc::round_to(ctx.carbs_pct, 1)),
            threshold: Some(65.0),
        });
    }

    // Low protein
    if ctx.protein_pct < 12.0 {
        issues.push(RuleIssue {
            category: "nutrition".into(),
            severity: if ctx.protein_pct < 8.0 { "critical" } else { "warning" }.into(),
            rule: "low_protein".into(),
            title_key: "rules.lowProtein".into(),
            description_key: "rules.lowProteinDesc".into(),
            fix_slugs: vec!["chicken-breast".into(), "egg".into(), "hard-cheese".into(), "lentils".into()],
            fix_keys: vec!["rules.fixAddProtein".into()],
            value: Some(uc::round_to(ctx.protein_pct, 1)),
            threshold: Some(12.0),
        });
    }

    // Very high fat
    if ctx.fat_pct > 45.0 {
        issues.push(RuleIssue {
            category: "nutrition".into(),
            severity: "warning".into(),
            rule: "high_fat_ratio".into(),
            title_key: "rules.highFatRatio".into(),
            description_key: "rules.highFatRatioDesc".into(),
            fix_slugs: vec![],
            fix_keys: vec!["rules.fixReduceFat".into()],
            value: Some(uc::round_to(ctx.fat_pct, 1)),
            threshold: Some(45.0),
        });
    }

    // Low fiber (per 1000 kcal should be ~14g)
    if ctx.total_calories > 200.0 {
        let fiber_per_1000 = ctx.fiber_g / (ctx.total_calories / 1000.0);
        if fiber_per_1000 < 10.0 {
            issues.push(RuleIssue {
                category: "nutrition".into(),
                severity: "warning".into(),
                rule: "low_fiber".into(),
                title_key: "rules.lowFiber".into(),
                description_key: "rules.lowFiberDesc".into(),
                fix_slugs: vec!["broccoli".into(), "lentils".into(), "oats".into()],
                fix_keys: vec!["rules.fixAddFiber".into()],
                value: Some(uc::round_to(fiber_per_1000, 1)),
                threshold: Some(10.0),
            });
        }
    }

    // High sugar (> 20% of calories from sugar)
    if ctx.total_calories > 200.0 {
        let sugar_cal_pct = (ctx.sugar_g * 4.0 / ctx.total_calories) * 100.0;
        if sugar_cal_pct > 20.0 {
            issues.push(RuleIssue {
                category: "nutrition".into(),
                severity: "warning".into(),
                rule: "high_sugar".into(),
                title_key: "rules.highSugar".into(),
                description_key: "rules.highSugarDesc".into(),
                fix_slugs: vec![],
                fix_keys: vec!["rules.fixReduceSugar".into()],
                value: Some(uc::round_to(sugar_cal_pct, 1)),
                threshold: Some(20.0),
            });
        }
    }
}

// ── Dominance rules ──────────────────────────────────────────────────────────

fn check_dominance_rules(ctx: &RecipeContext, issues: &mut Vec<RuleIssue>) {
    if ctx.total_grams <= 0.0 || ctx.ingredients.len() < 2 {
        return;
    }

    for (slug, grams, _pt) in &ctx.ingredients {
        let pct = grams / ctx.total_grams * 100.0;
        if pct > 60.0 {
            issues.push(RuleIssue {
                category: "dominance".into(),
                severity: if pct > 75.0 { "critical" } else { "warning" }.into(),
                rule: "ingredient_dominance".into(),
                title_key: "rules.dominance".into(),
                description_key: "rules.dominanceDesc".into(),
                fix_slugs: vec![], // context-dependent
                fix_keys: vec!["rules.fixAddVariety".into()],
                value: Some(uc::round_to(pct, 1)),
                threshold: Some(60.0),
            });
            // Only report the most dominant ingredient
            break;
        }
    }
}

// ── Structure rules ──────────────────────────────────────────────────────────

fn check_structure_rules(ctx: &RecipeContext, issues: &mut Vec<RuleIssue>) {
    // Check which culinary roles are present
    let types: Vec<&str> = ctx.ingredients.iter()
        .filter_map(|(_, _, pt)| pt.as_deref())
        .collect();

    let has = |t: &str| types.iter().any(|&pt| pt == t);

    // Should have a fat source
    if !has("oil") && !has("fat") && !has("dairy") {
        issues.push(RuleIssue {
            category: "structure".into(),
            severity: "warning".into(),
            rule: "missing_fat_source".into(),
            title_key: "rules.missingFat".into(),
            description_key: "rules.missingFatDesc".into(),
            fix_slugs: vec!["olive-oil".into(), "butter".into()],
            fix_keys: vec!["rules.fixAddFatSource".into()],
            value: None,
            threshold: None,
        });
    }

    // Should have herbs/spices for aroma
    if !has("herb") && !has("spice") {
        issues.push(RuleIssue {
            category: "structure".into(),
            severity: "warning".into(),
            rule: "missing_aromatics".into(),
            title_key: "rules.missingAromatics".into(),
            description_key: "rules.missingAromaticsDesc".into(),
            fix_slugs: vec!["garlic".into(), "basil".into(), "black-pepper".into()],
            fix_keys: vec!["rules.fixAddAromatics".into()],
            value: None,
            threshold: None,
        });
    }

    // Should have a vegetable
    if !has("vegetable") && !has("fruit") {
        issues.push(RuleIssue {
            category: "structure".into(),
            severity: "warning".into(),
            rule: "missing_vegetables".into(),
            title_key: "rules.missingVegetables".into(),
            description_key: "rules.missingVegetablesDesc".into(),
            fix_slugs: vec!["tomato".into(), "onion".into(), "broccoli".into()],
            fix_keys: vec!["rules.fixAddVegetables".into()],
            value: None,
            threshold: None,
        });
    }

    // Should have a protein source (for savory dishes)
    let has_protein_source = has("meat") || has("fish") || has("seafood") || has("egg") || has("legume");
    let has_grain = has("grain");
    if !has_protein_source && has_grain {
        issues.push(RuleIssue {
            category: "structure".into(),
            severity: "warning".into(),
            rule: "missing_protein_source".into(),
            title_key: "rules.missingProtein".into(),
            description_key: "rules.missingProteinDesc".into(),
            fix_slugs: vec!["chicken-breast".into(), "egg".into(), "salmon".into()],
            fix_keys: vec!["rules.fixAddProteinSource".into()],
            value: None,
            threshold: None,
        });
    }

    // Should have an acid source
    let has_acid = types.iter().any(|&pt| pt == "fruit" || pt == "vegetable") && ctx.flavor.acidity > 2.0;
    if !has_acid && ctx.flavor.acidity < 2.0 {
        issues.push(RuleIssue {
            category: "structure".into(),
            severity: "warning".into(),
            rule: "missing_acid_source".into(),
            title_key: "rules.missingAcid".into(),
            description_key: "rules.missingAcidDesc".into(),
            fix_slugs: vec!["lemon".into(), "tomato".into(), "white-wine-vinegar".into()],
            fix_keys: vec!["rules.fixAddAcidSource".into()],
            value: None,
            threshold: None,
        });
    }
}
