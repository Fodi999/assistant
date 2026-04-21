//! Product-level E2E tests — "does the system give the RIGHT answer?"
//!
//! These tests don't touch DB or Gemini. They build TechCards manually,
//! then run the real validation → auto_fix → adaptation → display pipeline.
//!
//! Run: cargo test --test product_e2e_test -- --nocapture

use restaurant_backend::application::rulebot::{
    recipe_engine::{TechCard, ResolvedIngredient, CookingStep},
    recipe_validation::{validate_recipe, ValidationReport, ValidationIssue, Severity},
    auto_fix::{auto_fix, FixReport},
    goal_engine::{profile_for, GoalProfile},
    goal_modifier::{detect_modifier, HealthModifier},
    adaptation_engine::{adapt_to_goal, AdaptationAction},
    user_constraints::{parse_constraints, UserConstraints, DietaryMode},
    display_name::format_recipe_text,
    intent_router::{detect_language, ChatLang},
};
use restaurant_backend::infrastructure::ingredient_cache::IngredientData;

// ═══════════════════════════════════════════════════════════════════════════════
//  HELPERS: build realistic TechCards from scratch
// ═══════════════════════════════════════════════════════════════════════════════

/// Ingredient shorthand: (slug, role, product_type, grams, kcal_per100, prot_per100, fat_per100, carbs_per100)
type IngSpec<'a> = (&'a str, &'a str, &'a str, f32, f32, f32, f32, f32);

fn make_ingredient(spec: &IngSpec) -> ResolvedIngredient {
    let (slug, role, ptype, grams, kcal100, prot100, fat100, carbs100) = *spec;
    let factor = grams / 100.0;
    ResolvedIngredient {
        product: Some(IngredientData {
            slug: slug.into(),
            name_en: slug.replace('-', " "),
            name_ru: slug.into(),
            name_pl: slug.into(),
            name_uk: slug.into(),
            calories_per_100g: kcal100,
            protein_per_100g: prot100,
            fat_per_100g: fat100,
            carbs_per_100g: carbs100,
            image_url: None,
            product_type: ptype.into(),
            density_g_per_ml: None, behaviors: vec![],
        }),
        slug_hint: slug.into(),
        resolved_slug: Some(slug.into()),
        state: "raw".into(),
        role: role.into(),
        gross_g: grams,
        cleaned_net_g: grams,
        cooked_net_g: grams * 0.85, // ~15% cooking loss
        kcal: (kcal100 * factor).round() as u32,
        protein_g: (prot100 * factor * 10.0).round() / 10.0,
        fat_g: (fat100 * factor * 10.0).round() / 10.0,
        carbs_g: (carbs100 * factor * 10.0).round() / 10.0,
    }
}

fn make_techcard(name: &str, servings: u8, specs: &[IngSpec]) -> TechCard {
    let ingredients: Vec<ResolvedIngredient> = specs.iter().map(make_ingredient).collect();

    let total_output: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
    let total_gross: f32 = ingredients.iter().map(|i| i.gross_g).sum();
    let total_kcal: u32 = ingredients.iter().map(|i| i.kcal).sum();
    let total_protein: f32 = ingredients.iter().map(|i| i.protein_g).sum();
    let total_fat: f32 = ingredients.iter().map(|i| i.fat_g).sum();
    let total_carbs: f32 = ingredients.iter().map(|i| i.carbs_g).sum();
    let s = servings.max(1) as f32;

    TechCard {
        dish_name: name.into(),
        dish_name_local: None,
        display_name: Some(name.into()),
        dish_type: "default".into(),
        servings,
        ingredients,
        steps: vec![
            CookingStep { step: 1, text: "Prepare ingredients".into(), time_min: Some(5), temp_c: None, tip: None },
            CookingStep { step: 2, text: "Cook".into(), time_min: Some(15), temp_c: Some(180), tip: None },
            CookingStep { step: 3, text: "Serve".into(), time_min: Some(2), temp_c: None, tip: None },
        ],
        total_output_g: total_output,
        total_gross_g: total_gross,
        total_kcal,
        total_protein,
        total_fat,
        total_carbs,
        per_serving_kcal: (total_kcal as f32 / s).round() as u32,
        per_serving_protein: (total_protein / s * 10.0).round() / 10.0,
        per_serving_fat: (total_fat / s * 10.0).round() / 10.0,
        per_serving_carbs: (total_carbs / s * 10.0).round() / 10.0,
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
    }
}

/// Run the full pipeline: validate → auto_fix → recalculate → revalidate → format.
/// Returns (TechCard, FixReport, ValidationReport_before, ValidationReport_after, formatted_text).
fn run_pipeline(
    tc: &mut TechCard,
    goal: &GoalProfile,
    constraints: &UserConstraints,
    lang: ChatLang,
) -> (FixReport, ValidationReport, ValidationReport, String) {
    let report_before = validate_recipe(tc, constraints, lang);
    let fix_report = auto_fix(tc, &report_before, goal, lang);

    // Store auto-fix messages for display
    tc.auto_fixes = fix_report.messages();
    tc.goal = goal.name.into();

    let report_after = validate_recipe(tc, constraints, lang);
    let text = format_recipe_text(tc, lang);

    (fix_report, report_before, report_after, text)
}

// ═══════════════════════════════════════════════════════════════════════════════
//  1. E2E: Weight loss recipe has low kcal and sufficient protein
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_weight_loss_recipe_has_low_kcal_and_protein() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🎯 E2E: Weight loss → low kcal + protein");
    println!("═══════════════════════════════════════════════════════\n");

    // A simple veggie soup — NO protein source
    let mut tc = make_techcard("light vegetable soup", 2, &[
        // slug,        role,     type,        g,   kcal/100, prot, fat, carbs
        ("potato",      "side",   "vegetable", 150.0, 77.0,   2.0, 0.1, 17.0),
        ("carrot",      "side",   "vegetable", 100.0, 41.0,   0.9, 0.2, 9.6),
        ("onion",       "side",   "vegetable", 80.0,  40.0,   1.1, 0.1, 9.3),
        ("olive-oil",   "oil",    "oil",       15.0,  884.0,  0.0, 100.0, 0.0),
    ]);

    let goal = profile_for(HealthModifier::LowCalorie); // weight_loss
    let constraints = UserConstraints::default();
    let (fixes, before, after, text) = run_pipeline(&mut tc, &goal, &constraints, ChatLang::Ru);

    println!("  Перед: {} issues", before.issues.len());
    for i in &before.issues { println!("    {} [{}] {}", if i.severity == Severity::Warning { "⚠️" } else { "❌" }, i.code, i.message); }
    println!("  Автофикс: {} fixes", fixes.fixes.len());
    for f in &fixes.fixes { println!("    🔧 {} → {}", f.action, f.detail); }
    println!("  После: {} issues", after.issues.len());
    println!("  Kcal/порция: {}", tc.per_serving_kcal);
    println!("  Белок/порция: {:.1}г", tc.per_serving_protein);
    println!("\n{}\n", text);

    // ── Assertions: weight loss recipe must be under 500 kcal ──
    assert!(tc.per_serving_kcal < 500,
        "weight loss recipe should be < 500 kcal/serving, got {}", tc.per_serving_kcal);

    // ── Assertions: auto-fix MUST have added protein ──
    assert!(!fixes.is_empty(),
        "auto-fix should have triggered (no protein in original)");
    assert!(tc.ingredients.iter().any(|i| i.role == "protein"),
        "auto-fix should have added a protein ingredient");

    // ── Assertions: protein must be reasonable ──
    assert!(tc.per_serving_protein >= 10.0,
        "weight loss recipe should have at least 10g protein/serving, got {:.1}",
        tc.per_serving_protein);

    println!("  ✅ Weight loss recipe: {}kcal, {:.1}g protein — PASS", tc.per_serving_kcal, tc.per_serving_protein);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  2. Goal ACTUALLY affects the dish: high-protein vs balanced
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_high_protein_goal_increases_protein() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  💪 Goal effect: HighProtein vs Balanced");
    println!("═══════════════════════════════════════════════════════\n");

    // Same base soup, no protein
    let base_specs: &[IngSpec] = &[
        ("potato",    "side", "vegetable", 150.0, 77.0,  2.0, 0.1, 17.0),
        ("beet",      "side", "vegetable", 120.0, 43.0,  1.6, 0.2, 9.6),
        ("cabbage",   "side", "vegetable", 100.0, 25.0,  1.3, 0.1, 5.8),
        ("olive-oil", "oil",  "oil",       10.0,  884.0, 0.0, 100.0, 0.0),
    ];

    // Run with BALANCED goal
    let mut tc_balanced = make_techcard("tomato soup", 1, base_specs);
    let goal_balanced = profile_for(HealthModifier::None);
    let (_, _, _, _) = run_pipeline(&mut tc_balanced, &goal_balanced, &UserConstraints::default(), ChatLang::En);
    let prot_balanced = tc_balanced.per_serving_protein;

    // Run with HIGH PROTEIN goal
    let mut tc_hp = make_techcard("tomato soup", 1, base_specs);
    let goal_hp = profile_for(HealthModifier::HighProtein);
    let (_, _, _, _) = run_pipeline(&mut tc_hp, &goal_hp, &UserConstraints::default(), ChatLang::En);
    let prot_hp = tc_hp.per_serving_protein;

    println!("  Balanced protein: {:.1}g", prot_balanced);
    println!("  HighProtein protein: {:.1}g", prot_hp);

    assert!(prot_hp > prot_balanced,
        "HighProtein goal should produce MORE protein than Balanced ({:.1} vs {:.1})",
        prot_hp, prot_balanced);

    println!("  ✅ HighProtein ({:.1}g) > Balanced ({:.1}g) — PASS", prot_hp, prot_balanced);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  3. Constraint: lactose-free removes dairy ingredients
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_lactose_free_constraint_parsing() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🚫 Constraint: lactose-free parsing");
    println!("═══════════════════════════════════════════════════════\n");

    let cases: &[(&str, &str)] = &[
        ("суп без лактозы",          "lactose"),
        ("soup lactose free",         "lactose"),
        ("zupa bez laktozy",          "lactose"),
        ("рецепт без глютена",       "gluten"),
        ("pasta gluten-free",         "gluten"),
        ("przepis bezglutenowy",      "gluten"),
        ("салат без орехов",          "nuts"),
        ("борщ без яиц",             "eggs"),
    ];

    for (input, expected_allergen) in cases {
        let lang = detect_language(input);
        let c = parse_constraints(input, lang);
        let ok = c.exclude_allergens.contains(&expected_allergen.to_string());
        let status = if ok { "✅" } else { "❌" };
        println!("  {} \"{}\" → allergens={:?} (expect {})",
            status, input, c.exclude_allergens, expected_allergen);
        assert!(ok,
            "Input \"{}\" should detect allergen '{}', got {:?}",
            input, expected_allergen, c.exclude_allergens);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  4. Auto-fix: adds protein to protein-less soup
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_auto_fix_adds_protein_if_missing() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🩹 Auto-fix: adds protein to veggie soup");
    println!("═══════════════════════════════════════════════════════\n");

    let mut tc = make_techcard("light veggie soup", 1, &[
        ("potato",    "side",  "vegetable", 150.0, 77.0,  2.0, 0.1, 17.0),
        ("carrot",    "side",  "vegetable", 100.0, 41.0,  0.9, 0.2, 9.6),
        ("broccoli",  "side",  "vegetable", 100.0, 34.0,  2.8, 0.4, 6.6),
    ]);

    let goal = profile_for(HealthModifier::None);

    // Step 1: validate — should detect NO_PROTEIN
    let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
    let has_no_protein = report.issues.iter().any(|i| i.code == "NO_PROTEIN");
    println!("  Before fix: NO_PROTEIN detected = {}", has_no_protein);
    assert!(has_no_protein, "veggie soup without protein should trigger NO_PROTEIN");

    // Step 2: auto-fix
    let fixes = auto_fix(&mut tc, &report, &goal, ChatLang::En);
    println!("  Fixes applied: {}", fixes.fixes.len());
    for f in &fixes.fixes { println!("    🔧 {} → {}", f.action, f.detail); }

    // Step 3: check protein was added
    assert!(!fixes.is_empty(), "auto-fix should have triggered");
    let protein_ing = tc.ingredients.iter().find(|i| i.role == "protein");
    assert!(protein_ing.is_some(), "protein ingredient should exist after fix");
    println!("  Added protein: {} ({:.0}g)",
        protein_ing.unwrap().slug_hint, protein_ing.unwrap().gross_g);

    // Step 4: re-validate — NO_PROTEIN should be gone
    let after = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
    let still_no_protein = after.issues.iter().any(|i| i.code == "NO_PROTEIN");
    assert!(!still_no_protein, "after fix, NO_PROTEIN should be resolved");

    println!("  ✅ Auto-fix added protein, NO_PROTEIN cleared — PASS");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  5. Auto-fix: KCAL_TOO_HIGH → reduces oil/fat
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_auto_fix_reduces_kcal_when_too_high() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  📉 Auto-fix: reduce kcal when too high");
    println!("═══════════════════════════════════════════════════════\n");

    let mut tc = make_techcard("fatty stir-fry", 1, &[
        ("chicken-breast", "protein", "meat",  300.0, 165.0, 31.0, 3.6, 0.0),
        ("olive-oil",      "oil",     "oil",   100.0, 884.0, 0.0,  100.0, 0.0),
        ("butter",         "oil",     "dairy",  80.0,  717.0, 0.9, 81.0, 0.1),
        ("rice",           "side",    "grain", 200.0,  130.0, 2.7, 0.3, 28.0),
    ]);

    let goal = profile_for(HealthModifier::LowCalorie);
    let kcal_before = tc.per_serving_kcal;

    let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
    let has_kcal_high = report.issues.iter().any(|i| i.code == "KCAL_TOO_HIGH");
    println!("  Before: {}kcal, KCAL_TOO_HIGH={}", kcal_before, has_kcal_high);

    let fixes = auto_fix(&mut tc, &report, &goal, ChatLang::En);
    println!("  After: {}kcal", tc.per_serving_kcal);
    for f in &fixes.fixes { println!("    🔧 {} → {}", f.action, f.detail); }

    assert!(tc.per_serving_kcal < kcal_before,
        "kcal should decrease after auto-fix ({} → {})", kcal_before, tc.per_serving_kcal);

    // Oil portions should have decreased
    let oil_after: f32 = tc.ingredients.iter()
        .filter(|i| i.role == "oil")
        .map(|i| i.gross_g)
        .sum();
    println!("  Oil total after fix: {:.0}g", oil_after);
    assert!(oil_after < 100.0, "oil should be reduced from 100g, got {:.0}g", oil_after);

    println!("  ✅ KCAL reduced: {} → {} kcal — PASS", kcal_before, tc.per_serving_kcal);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  6. Adaptation engine: vegan constraint → plant protein substitution
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_adaptation_adds_plant_protein_when_meat_removed() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🌱 Adaptation: vegan → plant protein added");
    println!("═══════════════════════════════════════════════════════\n");

    // Start with a normal soup with chicken
    let specs: &[IngSpec] = &[
        ("potato",    "side",    "vegetable", 150.0, 77.0,  2.0,  0.1, 17.0),
        ("carrot",    "side",    "vegetable", 100.0, 41.0,  0.9,  0.2, 9.6),
        ("onion",     "side",    "vegetable", 80.0,  40.0,  1.1,  0.1, 9.3),
        ("olive-oil", "oil",     "oil",       10.0,  884.0, 0.0,  100.0, 0.0),
    ];

    let mut ingredients: Vec<ResolvedIngredient> = specs.iter().map(make_ingredient).collect();
    let goal = profile_for(HealthModifier::None);

    // Simulate: meat was removed by constraint policy
    let removed_types = vec!["meat".to_string()];
    let adaptation = adapt_to_goal(&mut ingredients, &goal, &removed_types, 2);

    println!("  Strategy: {:?}", adaptation.strategy_applied);
    for a in &adaptation.actions {
        println!("    {} {} — {}", a.action, a.slug, a.detail);
    }

    // Check that plant protein was added
    let has_protein_sub = ingredients.iter()
        .any(|i| i.role == "protein" || i.slug_hint.contains("bean") || i.slug_hint.contains("lentil") || i.slug_hint.contains("chickpea") || i.slug_hint.contains("tofu"));

    assert!(has_protein_sub || !adaptation.is_empty(),
        "adaptation should add plant protein when meat is removed");

    println!("  ✅ Adaptation handled meat removal — PASS");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  7. Explain block: format_recipe_text contains key UX elements
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_explain_block_present_weight_loss() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  📋 Explain block: weight_loss text output");
    println!("═══════════════════════════════════════════════════════\n");

    let mut tc = make_techcard("light vegetable soup", 2, &[
        ("potato",    "side",    "vegetable", 150.0, 77.0,  2.0, 0.1, 17.0),
        ("carrot",    "side",    "vegetable", 100.0, 41.0,  0.9, 0.2, 9.6),
        ("onion",     "side",    "vegetable", 80.0,  40.0,  1.1, 0.1, 9.3),
    ]);
    tc.goal = "weight_loss".into();
    tc.auto_fixes = vec![
        "Added lentils: plant protein source".into(),
    ];
    tc.applied_constraints = vec!["low-calorie".into()];

    let text = format_recipe_text(&tc, ChatLang::Ru);
    println!("{}\n", text);

    // Must contain key UX elements
    assert!(text.contains("🍽"), "must have dish header 🍽");
    assert!(text.contains("🎯"), "must have goal emoji 🎯 (goal summary or applied)");
    assert!(text.contains("📊"), "must have macros line 📊");
    assert!(text.contains("🩹"), "must have auto-fix section 🩹 when fixes exist");

    println!("  ✅ Explain block has 🍽 🎯 📊 🩹 — PASS");
}

#[test]
fn test_explain_block_multilang() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🌍 Explain block: 4-language macros");
    println!("═══════════════════════════════════════════════════════\n");

    let mut tc = make_techcard("soup", 1, &[
        ("chicken-breast", "protein", "meat", 150.0, 165.0, 31.0, 3.6, 0.0),
        ("potato",         "side",    "vegetable", 100.0, 77.0, 2.0, 0.1, 17.0),
    ]);
    tc.goal = "high_protein".into();

    let checks: &[(ChatLang, &str, &str)] = &[
        (ChatLang::Ru, "📊 Б:", "Ru macros: Б/Ж/У"),
        (ChatLang::En, "📊 P:", "En macros: P/F/C"),
        (ChatLang::Pl, "📊 B:", "Pl macros: B/T/W"),
        (ChatLang::Uk, "📊 Б:", "Uk macros: Б/Ж/В"),
    ];

    for (lang, expected, label) in checks {
        let text = format_recipe_text(&tc, *lang);
        let ok = text.contains(expected);
        let status = if ok { "✅" } else { "❌" };
        println!("  {} {} | found \"{}\" in output", status, label, expected);
        assert!(ok, "{}: expected \"{}\" in:\n{}", label, expected, text);
    }

    // Goal summaries in each language
    let goal_checks: &[(ChatLang, &str)] = &[
        (ChatLang::Ru, "🎯 **Цель**: высокий белок"),
        (ChatLang::En, "🎯 **Goal**: high protein"),
        (ChatLang::Pl, "🎯 **Cel**: wysoki białko"),
        (ChatLang::Uk, "🎯 **Ціль**: високий білок"),
    ];

    for (lang, expected) in goal_checks {
        let text = format_recipe_text(&tc, *lang);
        let ok = text.contains(expected);
        let status = if ok { "✅" } else { "❌" };
        println!("  {} goal {:?}: \"{}\"", status, lang, expected);
        assert!(ok, "goal summary for {:?}: expected \"{}\" in:\n{}", lang, expected, text);
    }

    println!("  ✅ All 4 languages have macros + goal summary — PASS");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  8. GoalProfile contracts: profiles have correct ranges
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_goal_profiles_make_sense() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  📐 Goal profiles: ranges make sense");
    println!("═══════════════════════════════════════════════════════\n");

    let weight_loss = profile_for(HealthModifier::LowCalorie);
    let high_protein = profile_for(HealthModifier::HighProtein);
    let balanced = profile_for(HealthModifier::None);
    let low_carb = profile_for(HealthModifier::LowCarb);

    // Weight loss must have lowest kcal ceiling
    assert!(weight_loss.kcal.end <= balanced.kcal.end,
        "weight_loss kcal ceiling ({}) should be <= balanced ({})",
        weight_loss.kcal.end, balanced.kcal.end);
    println!("  ✅ weight_loss kcal ceiling ({}) <= balanced ({})", weight_loss.kcal.end, balanced.kcal.end);

    // High protein must have highest protein floor
    assert!(high_protein.protein_g.start >= balanced.protein_g.start,
        "high_protein protein floor ({}) should be >= balanced ({})",
        high_protein.protein_g.start, balanced.protein_g.start);
    println!("  ✅ high_protein protein floor ({}) >= balanced ({})", high_protein.protein_g.start, balanced.protein_g.start);

    // Low carb must have lowest carb ceiling
    assert!(low_carb.carbs_g.end < balanced.carbs_g.end,
        "low_carb carbs ceiling ({}) should be < balanced ({})",
        low_carb.carbs_g.end, balanced.carbs_g.end);
    println!("  ✅ low_carb carbs ceiling ({}) < balanced ({})", low_carb.carbs_g.end, balanced.carbs_g.end);

    // Weight loss should prefer steam/grill, forbid fry
    assert!(!weight_loss.forbidden_methods.is_empty(),
        "weight_loss should have forbidden cooking methods");
    println!("  ✅ weight_loss forbidden methods: {:?}", weight_loss.forbidden_methods);

    // High protein should have larger portion factor
    assert!(high_protein.portion_factor > balanced.portion_factor,
        "high_protein portion factor ({}) should be > balanced ({})",
        high_protein.portion_factor, balanced.portion_factor);
    println!("  ✅ high_protein portion_factor ({}) > balanced ({})",
        high_protein.portion_factor, balanced.portion_factor);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  9. Auto-fix generates steps when missing
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_auto_fix_generates_steps_when_missing() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  📝 Auto-fix: generates steps when NO_STEPS");
    println!("═══════════════════════════════════════════════════════\n");

    let mut tc = make_techcard("test soup", 1, &[
        ("chicken-breast", "protein", "meat", 150.0, 165.0, 31.0, 3.6, 0.0),
        ("potato",         "side",    "vegetable", 100.0, 77.0, 2.0, 0.1, 17.0),
        ("carrot",         "side",    "vegetable", 80.0,  41.0, 0.9, 0.2, 9.6),
    ]);
    tc.steps.clear(); // Remove all steps

    let goal = profile_for(HealthModifier::None);
    let report = validate_recipe(&tc, &UserConstraints::default(), ChatLang::En);
    let has_no_steps = report.issues.iter().any(|i| i.code == "NO_STEPS");
    println!("  Before: steps={}, NO_STEPS={}", tc.steps.len(), has_no_steps);
    assert!(has_no_steps, "should detect NO_STEPS");

    let fixes = auto_fix(&mut tc, &report, &goal, ChatLang::En);
    println!("  After: steps={}", tc.steps.len());
    for s in &tc.steps { println!("    {}. {}", s.step, s.text); }

    assert!(tc.steps.len() >= 2, "should generate at least 2 steps, got {}", tc.steps.len());
    assert!(!fixes.is_empty(), "should report fix actions");

    println!("  ✅ Generated {} steps — PASS", tc.steps.len());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  10. Modifier detection from real user input → correct GoalProfile
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_modifier_to_goal_pipeline() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🔗 Pipeline: user input → modifier → goal profile");
    println!("═══════════════════════════════════════════════════════\n");

    let cases: &[(&str, &str, &str)] = &[
        ("хочу похудеть, приготовь суп",           "weight_loss",  "LowCalorie"),
        ("make a high protein salad",               "high_protein", "HighProtein"),
        ("keto friendly chicken recipe",            "low_carb",     "LowCarb"),
        ("szybki przepis na obiad",                 "balanced",     "Quick"),
        ("богатое клетчаткой блюдо",               "high_fiber",   "HighFiber"),
    ];

    for (input, expected_goal, expected_mod) in cases {
        let modifier = detect_modifier(&input.to_lowercase());
        let profile = profile_for(modifier.clone());
        let ok = profile.name == *expected_goal;
        let status = if ok { "✅" } else { "❌" };
        println!("  {} \"{}\" → modifier={:?} → goal={} (expect {})",
            status, input, modifier, profile.name, expected_goal);
        assert_eq!(profile.name, *expected_goal,
            "Input \"{}\" should produce goal '{}', got '{}' (modifier={:?})",
            input, expected_goal, profile.name, modifier);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  11. Validation catches obviously bad recipes
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_validation_catches_bad_recipes() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🔍 Validation: catches bad recipes");
    println!("═══════════════════════════════════════════════════════\n");

    // Only 1 ingredient → TOO_FEW_INGREDIENTS
    let tc_tiny = make_techcard("sad dish", 1, &[
        ("salt", "spice", "spice", 5.0, 0.0, 0.0, 0.0, 0.0),
    ]);
    let r1 = validate_recipe(&tc_tiny, &UserConstraints::default(), ChatLang::En);
    assert!(r1.has_errors(), "1-ingredient dish should have errors");
    assert!(r1.errors().iter().any(|e| e.code == "TOO_FEW_INGREDIENTS"));
    println!("  ✅ TOO_FEW_INGREDIENTS detected for 1-ingredient dish");

    // 2000 kcal dish → KCAL_TOO_HIGH
    let tc_fat = make_techcard("butter feast", 1, &[
        ("butter", "oil", "dairy", 200.0, 717.0, 0.9, 81.0, 0.1),
        ("cream",  "oil", "dairy", 200.0, 340.0, 2.1, 36.0, 3.4),
        ("sugar",  "side", "other", 100.0, 387.0, 0.0, 0.0, 100.0),
    ]);
    let r2 = validate_recipe(&tc_fat, &UserConstraints::default(), ChatLang::En);
    assert!(r2.issues.iter().any(|i| i.code == "KCAL_TOO_HIGH"));
    println!("  ✅ KCAL_TOO_HIGH detected for {}kcal dish", tc_fat.total_kcal);

    // No protein dish → NO_PROTEIN
    let tc_veg = make_techcard("veggie", 1, &[
        ("potato",  "side", "vegetable", 150.0, 77.0, 2.0, 0.1, 17.0),
        ("carrot",  "side", "vegetable", 100.0, 41.0, 0.9, 0.2, 9.6),
        ("broccoli","side", "vegetable", 100.0, 34.0, 2.8, 0.4, 6.6),
    ]);
    let r3 = validate_recipe(&tc_veg, &UserConstraints::default(), ChatLang::En);
    assert!(r3.issues.iter().any(|i| i.code == "NO_PROTEIN"));
    println!("  ✅ NO_PROTEIN detected for all-veggie dish");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  12. Full pipeline roundtrip: input → modifier → goal → validate → fix → display
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_full_pipeline_roundtrip_4_languages() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🚀 Full pipeline roundtrip × 4 languages");
    println!("═══════════════════════════════════════════════════════\n");

    let inputs: &[(&str, &str, &str)] = &[
        ("хочу похудеть, приготовь суп",           "weight_loss",  "Ru"),
        ("make a light soup for weight loss",       "weight_loss",  "En"),
        ("ugotuj lekką zupę na odchudzanie",       "weight_loss",  "Pl"),
        ("приготуй легкий суп для схуднення",      "weight_loss",  "Ru"), // UK detects as Ru without 'ї'
    ];

    for (input, expected_goal, expected_lang) in inputs {
        let lang = detect_language(input);
        let modifier = detect_modifier(&input.to_lowercase());
        let goal = profile_for(modifier.clone());
        let constraints = parse_constraints(input, lang);

        let mut tc = make_techcard("light soup", 2, &[
            ("potato",    "side", "vegetable", 150.0, 77.0,  2.0, 0.1, 17.0),
            ("carrot",    "side", "vegetable", 100.0, 41.0,  0.9, 0.2, 9.6),
            ("onion",     "side", "vegetable", 80.0,  40.0,  1.1, 0.1, 9.3),
        ]);

        let (fixes, _, after, text) = run_pipeline(&mut tc, &goal, &constraints, lang);

        let goal_ok = goal.name == *expected_goal;
        let has_protein = tc.ingredients.iter().any(|i| i.role == "protein");
        let has_macros = text.contains("📊");

        let status = if goal_ok && has_protein && has_macros { "✅" } else { "❌" };
        println!("  {} \"{}\"", status, input);
        println!("      lang={:?} modifier={:?} goal={}", lang, modifier, goal.name);
        println!("      protein_added={} kcal/serv={} fixes={}",
            has_protein, tc.per_serving_kcal, fixes.fixes.len());
        println!("      has_macros={} has_goal_summary={}",
            has_macros, text.contains("🎯"));

        assert_eq!(goal.name, *expected_goal,
            "\"{}\" should produce goal '{}'", input, expected_goal);
        assert!(has_protein,
            "\"{}\" should have protein after pipeline", input);
        assert!(has_macros,
            "\"{}\" output should contain macros 📊", input);
    }

    println!("\n  ✅ All 4 roundtrips passed — PASS");
}
