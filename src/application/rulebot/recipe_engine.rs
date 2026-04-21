//! Recipe Engine v2 — minimal AI, maximal backend control.
//!
//! Philosophy:
//!   Gemini = "what dish, which ingredients"  (50–100 tokens)
//!   Backend = everything else: state, role, grams, yield, КБЖУ
//!
//! Flow:
//!   1. Gemini → `{"dish":"borscht","items":["beet","cabbage","potato",…]}`
//!   2. Backend resolves slugs from IngredientCache
//!   3. Backend assigns role (meal_role), cooking method, portion grams
//!   4. Backend computes gross/net/yield/КБЖУ deterministically
//!   5. Response builder renders recipe-view or tech-card
//!
//! Extracted modules:
//!   - `dish_schema`         — Gemini call + JSON parsing
//!   - `ingredient_resolver` — slug resolution + implicit ingredients
//!   - `nutrition_math`      — portions, yields, КБЖУ, allergens, diet tags
//!   - `display_name`        — multilingual grammar + display names

use serde::Serialize;

use crate::infrastructure::IngredientCache;
use crate::infrastructure::ingredient_cache::IngredientData;
use super::intent_router::ChatLang;
use super::meal_builder::CookMethod;
use super::response_builder::HealthGoal;
use super::goal_modifier::HealthModifier;
use super::cooking_rules::{self, IngredientRole, StepType};
use super::food_pairing;
use super::user_constraints::UserConstraints;
use super::constraint_policy;
use super::goal_engine;
use super::adaptation_engine;
use super::recipe_validation;
use super::auto_fix;

// ── Re-exports from extracted modules ────────────────────────────────────────
// Callers still use `recipe_engine::ask_gemini_dish_schema`, etc.

pub use super::dish_schema::{DishSchema, ask_gemini_dish_schema};
pub use super::display_name::{format_recipe_text, state_label};

// ── Internal imports from extracted modules ──────────────────────────────────

use super::ingredient_resolver::{resolve_slug, auto_insert_implicit};
use super::culinary_base_layer;
use super::nutrition_math::{
    build_ingredient_for_dish,
    round1, compute_complexity, detect_allergens, detect_diet_tags,
};
use super::display_name::{
    build_display_name,
    instrumental_phrase, accusative_phrase,
    instrumental_phrase_pl, accusative_phrase_pl,
    instrumental_phrase_uk, accusative_phrase_uk,
};

// ── Dish Cooking Profile ─────────────────────────────────────────────────────

/// The type of dish determines how every ingredient is cooked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DishType {
    Soup,       // borscht, ramen, pho, minestrone…
    Stew,       // goulash, ragout, curry…
    Salad,      // caesar, greek…
    StirFry,    // wok, pad thai…
    Grill,      // bbq, steaks…
    Bake,       // casserole, lasagna, pizza…
    Pasta,      // spaghetti, carbonara…
    Raw,        // tartare, sashimi…
    Default,    // unknown → old behaviour
}

impl DishType {
    /// Detect dish type from the English dish name returned by Gemini.
    pub fn detect(dish: &str) -> Self {
        let d = dish.to_lowercase();
        // Soups
        if d.contains("soup") || d.contains("borscht") || d.contains("borsch")
            || d.contains("ramen") || d.contains("pho") || d.contains("minestrone")
            || d.contains("chowder") || d.contains("consomme") || d.contains("gazpacho")
            || d.contains("bouillon") || d.contains("broth") || d.contains("ukha")
            || d.contains("shchi") || d.contains("solyanka") || d.contains("rassolnik")
            || d.contains("kharcho") || d.contains("tom yum") || d.contains("laksa")
            || d.contains("miso") { return DishType::Soup; }
        // Stews
        if d.contains("stew") || d.contains("ragout") || d.contains("goulash")
            || d.contains("curry") || d.contains("chili con") || d.contains("tagine")
            || d.contains("casserole") || d.contains("pot roast")
            || d.contains("braised") { return DishType::Stew; }
        // Salads
        if d.contains("salad") || d.contains("ceviche")
            || d.contains("coleslaw") || d.contains("tabouleh") { return DishType::Salad; }
        // Stir-fry / wok
        if d.contains("stir") || d.contains("wok") || d.contains("pad thai")
            || d.contains("fried rice") || d.contains("chow mein") { return DishType::StirFry; }
        // Grill
        if d.contains("grill") || d.contains("bbq") || d.contains("kebab")
            || d.contains("shashlik") || d.contains("steak")
            || d.contains("burger") { return DishType::Grill; }
        // Bake
        if d.contains("bake") || d.contains("lasagna") || d.contains("pizza")
            || d.contains("quiche") || d.contains("pie")
            || d.contains("gratin") { return DishType::Bake; }
        // Pasta
        if d.contains("pasta") || d.contains("spaghetti") || d.contains("carbonara")
            || d.contains("penne") || d.contains("fettuccine")
            || d.contains("macaroni") || d.contains("noodle") { return DishType::Pasta; }
        // Raw
        if d.contains("tartare") || d.contains("sashimi")
            || d.contains("carpaccio") { return DishType::Raw; }
        DishType::Default
    }

    /// The cooking method for a given role in this dish type.
    /// Delegates to cooking_rules for DDD rule lookup.
    pub fn cook_method(&self, role: &str, slug: &str, _product_type: &str, _goal: HealthGoal) -> CookMethod {
        let rule = cooking_rules::load_rule(*self);
        let ingredient_role = IngredientRole::from_str_role(role, slug);
        cooking_rules::method_for_role(&rule, ingredient_role)
    }
}

// ── Cooking Steps ────────────────────────────────────────────────────────────

/// A simple cooking step (pure logic, no LLM).
#[derive(Debug, Clone, Serialize)]
pub struct CookingStep {
    pub step: u8,
    pub text: String,
    pub time_min: Option<u16>,
    /// Cooking temperature in °C if relevant (sear=200, bake=180, etc.)
    pub temp_c: Option<u16>,
    /// Short chef tip for this step (localized)
    pub tip: Option<String>,
}

// ── Types ────────────────────────────────────────────────────────────────────

/// Backend-resolved ingredient with full calculations.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedIngredient {
    #[serde(skip)]
    pub product: Option<IngredientData>,
    pub slug_hint: String,
    pub resolved_slug: Option<String>,
    pub state: String,
    pub role: String,
    pub gross_g: f32,
    pub cleaned_net_g: f32,
    pub cooked_net_g: f32,
    pub kcal: u32,
    pub protein_g: f32,
    pub fat_g: f32,
    pub carbs_g: f32,
}

/// Full resolved recipe / tech-card.
#[derive(Debug, Clone, Serialize)]
pub struct TechCard {
    pub dish_name: String,
    pub dish_name_local: Option<String>,
    /// Improved display name: "Классический борщ с говядиной"
    pub display_name: Option<String>,
    pub dish_type: String,
    pub servings: u8,
    pub ingredients: Vec<ResolvedIngredient>,
    pub steps: Vec<CookingStep>,
    pub total_output_g: f32,
    pub total_gross_g: f32,
    pub total_kcal: u32,
    pub total_protein: f32,
    pub total_fat: f32,
    pub total_carbs: f32,
    pub per_serving_kcal: u32,
    pub per_serving_protein: f32,
    pub per_serving_fat: f32,
    pub per_serving_carbs: f32,
    pub unresolved: Vec<String>,
    /// Ingredients removed by food pairing filter, e.g. [("ice-cream", "banned in Soup")]
    pub removed_ingredients: Vec<(String, String)>,
    // ── Dish context (v2) ──
    /// "easy" | "medium" | "hard"
    pub complexity: String,
    /// "balanced" | "high_protein" | "low_calorie"
    pub goal: String,
    /// Allergen/intolerance flags present in the dish, e.g. ["gluten", "lactose", "nuts"]
    pub allergens: Vec<String>,
    /// Diet tags, e.g. ["vegetarian", "vegan", "pescatarian"]
    pub tags: Vec<String>,
    /// Dietary constraints applied, e.g. ["lactose-free", "vegan diet"]
    pub applied_constraints: Vec<String>,
    /// Adaptation actions taken, e.g. [("added", "chickpeas", "protein substitute")]
    pub adaptations: Vec<adaptation_engine::AdaptationAction>,
    /// Post-build validation warnings
    pub validation_warnings: Vec<String>,
    /// Auto-fix actions taken, e.g. [("Added eggs", "2 eggs as protein source")]
    pub auto_fixes: Vec<String>,
    /// Flavor/texture analysis from culinary behaviors DSL
    pub flavor_analysis: Option<super::flavor_engine::FlavorAnalysis>,
}

// ── Backend Intelligence: resolve, assign roles, portions, cook methods ──────

/// Resolve a minimal dish schema into a full TechCard.
/// ALL intelligence (roles, grams, states, yields) lives here — not in Gemini.
pub async fn resolve_dish(
    cache: &IngredientCache,
    schema: &DishSchema,
    goal: HealthGoal,
    lang: ChatLang,
    constraints: &UserConstraints,
    modifier: HealthModifier,
) -> TechCard {
    let dish_type = DishType::detect(&schema.dish);
    let rule = cooking_rules::load_rule(dish_type);
    tracing::info!("🍳 DishType: {:?} for '{}'", dish_type, schema.dish);

    // ── 1. Food Pairing Filter: remove absurd combinations ──────────────
    let (filtered_items, removed) = food_pairing::filter_ingredients(&schema.items, dish_type);
    if !removed.is_empty() {
        tracing::warn!("🚫 Removed ingredients: {:?}", removed);
    }

    let mut ingredients = Vec::new();
    let mut unresolved = Vec::new();

    for slug_hint in &filtered_items {
        match resolve_slug(cache, slug_hint).await {
            Some(product) => {
                let resolved = build_ingredient_for_dish(&product, slug_hint, goal, dish_type);
                ingredients.push(resolved);
            }
            None => {
                unresolved.push(slug_hint.clone());
                ingredients.push(ResolvedIngredient {
                    product: None,
                    slug_hint: slug_hint.clone(),
                    resolved_slug: None,
                    state: "raw".into(),
                    role: "other".into(),
                    gross_g: 0.0, cleaned_net_g: 0.0, cooked_net_g: 0.0,
                    kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
                });
            }
        }
    }

    // ── 2. Auto-insert implicit ingredients (Liquid for soup, Oil for sauté) ──
    auto_insert_implicit(&mut ingredients, dish_type, cache, goal).await;

    // ── 2a. Culinary Base Layer: salt, fat, pepper, aromatics ────────────
    culinary_base_layer::apply_culinary_basics(&mut ingredients, dish_type, cache, goal).await;

    // ── 2b. Dietary Constraint Policy: remove/substitute per user preferences ──
    let constraint_report = constraint_policy::apply_dietary_constraints(&mut ingredients, constraints);
    if !constraint_report.removed.is_empty() {
        tracing::info!("🥗 Dietary constraints removed: {:?}", constraint_report.removed);
    }
    let mut all_removed = removed;
    for (slug, reason) in &constraint_report.removed {
        all_removed.push((slug.clone(), reason.clone()));
    }

    // ── 2c. Adaptation Engine: smart rebalancing per goal profile ─────────
    let goal_profile = goal_engine::profile_for(modifier);
    let removed_types: Vec<String> = constraint_report.removed.iter()
        .filter_map(|(slug, _)| {
            // Try to infer product_type from removed slug
            // (the actual product was already removed, so we check the reason)
            None::<String> // We pass dietary mode types instead
        })
        .collect();
    // Collect broad removed categories from dietary mode
    let removed_categories: Vec<String> = match constraints.dietary_mode {
        Some(super::user_constraints::DietaryMode::Vegan) =>
            vec!["meat".into(), "fish".into(), "seafood".into(), "dairy".into()],
        Some(super::user_constraints::DietaryMode::Vegetarian) =>
            vec!["meat".into(), "fish".into(), "seafood".into()],
        Some(super::user_constraints::DietaryMode::Pescatarian) =>
            vec!["meat".into()],
        None => vec![],
    };

    let adapt_servings = {
        let total: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
        let target = match dish_type {
            DishType::Soup | DishType::Stew => 350.0_f32,
            DishType::Salad | DishType::Raw    => 250.0,
            _                                   => 300.0,
        };
        ((total / target).round() as u8).max(1)
    };

    let adaptation_report = adaptation_engine::adapt_to_goal(
        &mut ingredients,
        &goal_profile,
        &removed_categories,
        adapt_servings,
    );
    if !adaptation_report.is_empty() {
        tracing::info!("🔄 Adaptations: {:?}", adaptation_report.actions);
    }

    // ── 3. Constraint Engine: enforce culinary laws ─────────────────────
    let servings_estimate = {
        let total: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
        let target = match dish_type {
            DishType::Soup | DishType::Stew => 350.0_f32,
            DishType::Salad | DishType::Raw    => 250.0,
            _                                   => 300.0,
        };
        ((total / target).round() as u8).max(1)
    };

    let mut snapshots: Vec<cooking_rules::IngredientSnapshot> = ingredients.iter().map(|i| {
        let slug = i.resolved_slug.as_deref().unwrap_or(&i.slug_hint);
        cooking_rules::IngredientSnapshot {
            slug: slug.to_string(),
            role: IngredientRole::from_str_role(&i.role, slug),
            gross_g: i.gross_g,
            fat_g: i.fat_g,
            protein_g: i.protein_g,
            kcal: i.kcal,
        }
    }).collect();

    let violations = cooking_rules::apply_constraints(&rule, &mut snapshots, servings_estimate);

    // Apply constraint fixes back to actual ingredients
    if !violations.is_empty() {
        for v in &violations {
            if v.auto_fixed {
                tracing::info!("🔧 Constraint auto-fix: {}", v.message);
            } else {
                tracing::warn!("⚠️ Constraint warning: {}", v.message);
            }
        }

        // Sync oil/fat changes back from snapshots
        for (ing, snap) in ingredients.iter_mut().zip(snapshots.iter()) {
            let slug = ing.resolved_slug.as_deref().unwrap_or(&ing.slug_hint);
            let role = IngredientRole::from_str_role(&ing.role, slug);
            if role == IngredientRole::Oil && (ing.gross_g - snap.gross_g).abs() > 0.1 {
                ing.gross_g = snap.gross_g;
                ing.cleaned_net_g = snap.gross_g;
                ing.cooked_net_g = snap.gross_g;
                ing.fat_g = snap.fat_g;
                ing.kcal = snap.kcal;
            }
        }

        // If constraint engine added water (RequiresLiquid auto-fix),
        // check if it's already in ingredients (auto_insert_implicit may have added it)
        let has_liquid = ingredients.iter().any(|i| i.role == "liquid");
        let snap_has_water = snapshots.iter().any(|s| s.slug == "water" && s.role == IngredientRole::Liquid);
        if snap_has_water && !has_liquid {
            let water_product = resolve_slug(cache, "water").await;
            ingredients.push(ResolvedIngredient {
                product: water_product,
                slug_hint: "water".into(),
                resolved_slug: Some("water".into()),
                state: "boiled".into(),
                role: "liquid".into(),
                gross_g: 300.0, cleaned_net_g: 300.0, cooked_net_g: 300.0,
                kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
            });
        }
    }

    // ── 3b. Deduplicate ingredients (merge water+water, etc.) ──────────
    merge_duplicate_ingredients(&mut ingredients);

    // ── 4. Compute totals ───────────────────────────────────────────────
    let total_gross: f32 = ingredients.iter().map(|i| i.gross_g).sum();
    let total_output: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
    let total_kcal: u32 = ingredients.iter().map(|i| i.kcal).sum();
    let total_protein: f32 = ingredients.iter().map(|i| i.protein_g).sum();
    let total_fat: f32 = ingredients.iter().map(|i| i.fat_g).sum();
    let total_carbs: f32 = ingredients.iter().map(|i| i.carbs_g).sum();

    // ── 5. Generate cooking steps ───────────────────────────────────────
    let steps = generate_steps(&ingredients, dish_type, lang);

    // Build improved display name (with goal prefix)
    let display_name = build_display_name(schema, &ingredients, dish_type, goal, lang);

    // ── 5b. Dish context: complexity, goal label, allergens, tags ────────
    let complexity = compute_complexity(&steps);
    let goal_label = match goal {
        HealthGoal::HighProtein => "high_protein",
        HealthGoal::LowCalorie  => "low_calorie",
        HealthGoal::Balanced    => "balanced",
    }.to_string();
    let allergens = detect_allergens(&ingredients);
    let tags = detect_diet_tags(&ingredients);

    // ── 6. Auto-portion: split into realistic servings (~300–400g each) ──
    let portion_target = match dish_type {
        DishType::Soup | DishType::Stew => 350.0_f32,
        DishType::Salad | DishType::Raw    => 250.0,
        _                                   => 300.0,
    };
    let servings = ((total_output / portion_target).round() as u8).max(1);
    let per_kcal = (total_kcal as f32 / servings as f32).round() as u32;
    let per_prot = round1(total_protein / servings as f32);
    let per_fat  = round1(total_fat / servings as f32);
    let per_carb = round1(total_carbs / servings as f32);

    let mut tech_card = TechCard {
        dish_name: schema.dish.clone(),
        dish_name_local: schema.dish_local.clone(),
        display_name: Some(display_name),
        dish_type: format!("{:?}", dish_type).to_lowercase(),
        servings,
        ingredients,
        steps,
        total_output_g: total_output,
        total_gross_g: total_gross,
        total_kcal,
        total_protein,
        total_fat,
        total_carbs,
        per_serving_kcal: per_kcal,
        per_serving_protein: per_prot,
        per_serving_fat: per_fat,
        per_serving_carbs: per_carb,
        unresolved,
        removed_ingredients: all_removed,
        complexity,
        goal: goal_label,
        allergens,
        tags,
        applied_constraints: constraint_report.messages,
        adaptations: adaptation_report.actions,
        validation_warnings: vec![], // filled below
        auto_fixes: vec![],          // filled below
        flavor_analysis: None,       // filled below
    };

    // ── 6b. Flavor/texture analysis from culinary behaviors ────────────────
    let flavor = super::flavor_engine::analyze_dish(&tech_card.ingredients);
    if !flavor.suggestions.is_empty() {
        tracing::info!("🎨 Flavor analysis: dominant={:?}, balance={:.2}, suggestions={:?}",
            flavor.dominant, flavor.balance_score, flavor.suggestions);
    }
    tech_card.flavor_analysis = Some(flavor);

    // ── 7. Post-build validation ────────────────────────────────────────
    let validation = recipe_validation::validate_recipe(&tech_card, constraints, lang);
    if !validation.issues.is_empty() {
        for issue in &validation.issues {
            match issue.severity {
                recipe_validation::Severity::Error =>
                    tracing::warn!("❌ Validation error: {}", issue.message),
                recipe_validation::Severity::Warning =>
                    tracing::info!("⚠️ Validation warning: {}", issue.message),
            }
        }

        // ── 8. Auto-fix: repair what we can (goal-aware + localized) ────
        let fix_report = auto_fix::auto_fix(&mut tech_card, &validation, &goal_profile, lang);
        if !fix_report.is_empty() {
            tracing::info!("🔧 Auto-fixes applied: {:?}", fix_report.messages());
            tech_card.auto_fixes = fix_report.messages();
        }

        // ── 9. Revalidation: clear warnings that were fixed ─────────────
        let revalidation = recipe_validation::validate_recipe(&tech_card, constraints, lang);
        let mut all_warnings = revalidation.warning_messages();

        // ── 9b. Culinary logic validation ────────────────────────────────
        let culinary_warnings = culinary_base_layer::validate_cooking_logic(&tech_card.ingredients, dish_type);
        all_warnings.extend(culinary_warnings);

        tech_card.validation_warnings = all_warnings;

        if revalidation.issues.is_empty() {
            tracing::info!("✅ All validation issues resolved by auto-fix");
        } else {
            for issue in &revalidation.issues {
                tracing::info!("⚠️ Post-fix: {}", issue.message);
            }
        }
    }

    tech_card
}

// ── Ingredient Deduplication ─────────────────────────────────────────────────

/// Merge duplicate ingredients by resolved_slug (or slug_hint fallback).
/// Water + Water → single Water with summed grams.
/// This prevents duplicates caused by Gemini schema + auto_insert_implicit + cooking_rules
/// all potentially adding the same ingredient (e.g. water for soup).
fn merge_duplicate_ingredients(ingredients: &mut Vec<ResolvedIngredient>) {
    use std::collections::HashMap;

    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut merged_indices: Vec<usize> = Vec::new();

    for i in 0..ingredients.len() {
        let key = ingredients[i].resolved_slug.as_deref()
            .unwrap_or(&ingredients[i].slug_hint)
            .to_lowercase();

        if let Some(&first_idx) = seen.get(&key) {
            // Merge into the first occurrence
            let donor_gross = ingredients[i].gross_g;
            let donor_clean = ingredients[i].cleaned_net_g;
            let donor_cooked = ingredients[i].cooked_net_g;
            let donor_kcal = ingredients[i].kcal;
            let donor_protein = ingredients[i].protein_g;
            let donor_fat = ingredients[i].fat_g;
            let donor_carbs = ingredients[i].carbs_g;

            let target = &mut ingredients[first_idx];
            target.gross_g += donor_gross;
            target.cleaned_net_g += donor_clean;
            target.cooked_net_g += donor_cooked;
            target.kcal += donor_kcal;
            target.protein_g += donor_protein;
            target.fat_g += donor_fat;
            target.carbs_g += donor_carbs;

            merged_indices.push(i);
        } else {
            seen.insert(key, i);
        }
    }

    // Remove merged duplicates in reverse order to preserve indices
    for &idx in merged_indices.iter().rev() {
        ingredients.remove(idx);
    }
}

// ── Cooking Steps Generation (pure logic, no LLM) ───────────────────────────

/// Generate cooking steps driven by DishRule (DDD: rules as data).
/// Iterates the rule's step sequence; for each step, collects matching ingredients,
/// skips the step if no ingredients match, otherwise generates text.
fn generate_steps(ingredients: &[ResolvedIngredient], dish_type: DishType, lang: ChatLang) -> Vec<CookingStep> {
    let rule = cooking_rules::load_rule(dish_type);
    let lang_code = lang.code();

    // ── Classify ingredients by DDD role ─────────────────────────────────
    let classify = |ing: &ResolvedIngredient| -> IngredientRole {
        let slug = ing.resolved_slug.as_deref().unwrap_or(&ing.slug_hint);
        IngredientRole::from_str_role(&ing.role, slug)
    };

    let by_role = |target: IngredientRole| -> Vec<&ResolvedIngredient> {
        ingredients.iter().filter(|i| classify(i) == target).collect()
    };

    // Helper: pick ingredient name by language + apply grammar
    // `case` controls declension: accusative for most steps, instrumental for Dress
    let name_of_case = |ing: &ResolvedIngredient, case: &str| -> String {
        ing.product.as_ref()
            .map(|p| {
                let raw = match lang {
                    ChatLang::En => &p.name_en,
                    ChatLang::Pl => &p.name_pl,
                    ChatLang::Uk => &p.name_uk,
                    ChatLang::Ru => &p.name_ru,
                };
                match lang {
                    ChatLang::Ru => match case {
                        "instr" => instrumental_phrase(raw),
                        _       => accusative_phrase(raw),
                    },
                    ChatLang::Pl => match case {
                        "instr" => instrumental_phrase_pl(raw),
                        _       => accusative_phrase_pl(raw),
                    },
                    ChatLang::Uk => match case {
                        "instr" => instrumental_phrase_uk(raw),
                        _       => accusative_phrase_uk(raw),
                    },
                    ChatLang::En => raw.to_lowercase(),
                }
            })
            .unwrap_or_else(|| ing.slug_hint.clone())
    };

    let name_of = |ing: &ResolvedIngredient| -> String {
        name_of_case(ing, "acc")
    };

    let sep = match lang {
        ChatLang::Ru => " и ",
        ChatLang::En => " and ",
        ChatLang::Pl => " i ",
        ChatLang::Uk => " і ",
    };

    let names_of = |ings: &[&ResolvedIngredient], join: &str| -> String {
        ings.iter()
            .map(|i| name_of(i))
            .collect::<Vec<_>>()
            .join(join)
    };

    let names_of_case = |ings: &[&ResolvedIngredient], join: &str, case: &str| -> String {
        ings.iter()
            .map(|i| name_of_case(i, case))
            .collect::<Vec<_>>()
            .join(join)
    };

    // ── Walk the rule's step sequence ────────────────────────────────────
    let mut steps = Vec::new();
    let mut step_num: u8 = 0;
    let mut had_saute = false; // Track if SauteAromatics was emitted

    // Localized chef tips
    let tip_text = |key: &str| -> Option<String> {
        let t = match (key, lang) {
            ("foam", ChatLang::Ru) => "Снимайте пену для прозрачного бульона",
            ("foam", ChatLang::En) => "Skim foam for a clear broth",
            ("foam", ChatLang::Pl) => "Zbieraj pianę dla przejrzystego bulionu",
            ("foam", ChatLang::Uk) => "Знімайте піну для прозорого бульйону",
            ("golden", ChatLang::Ru) => "До золотистого цвета, не пережаривайте",
            ("golden", ChatLang::En) => "Until golden, don't over-brown",
            ("golden", ChatLang::Pl) => "Do złotego koloru, nie przypalaj",
            ("golden", ChatLang::Uk) => "До золотистого кольору, не пересмажуйте",
            ("sear_first", ChatLang::Ru) => "Обжарьте мясо до корочки перед тушением",
            ("sear_first", ChatLang::En) => "Sear meat before braising for depth",
            ("sear_first", ChatLang::Pl) => "Obsmaż mięso przed duszeniem",
            ("sear_first", ChatLang::Uk) => "Обсмажте м'ясо перед тушкуванням",
            ("smoking", ChatLang::Ru) => "Масло должно слегка дымиться",
            ("smoking", ChatLang::En) => "Oil should be lightly smoking",
            ("smoking", ChatLang::Pl) => "Olej powinien się lekko dymić",
            ("smoking", ChatLang::Uk) => "Олія повинна ледь димитися",
            ("no_move", ChatLang::Ru) => "Не двигайте — дайте корочке сформироваться",
            ("no_move", ChatLang::En) => "Don't move — let the crust form",
            ("no_move", ChatLang::Pl) => "Nie ruszaj — pozwól się zrumienić",
            ("no_move", ChatLang::Uk) => "Не рухайте — дайте скоринці сформуватись",
            ("rest_after", ChatLang::Ru) => "Дайте отдохнуть 5 мин перед нарезкой",
            ("rest_after", ChatLang::En) => "Rest 5 min before cutting",
            ("rest_after", ChatLang::Pl) => "Odczekaj 5 min przed krojeniem",
            ("rest_after", ChatLang::Uk) => "Дайте відпочити 5 хв перед нарізкою",
            ("al_dente", ChatLang::Ru) => "Al dente — варите на 1 мин меньше",
            ("al_dente", ChatLang::En) => "Al dente — cook 1 min less than package",
            ("al_dente", ChatLang::Pl) => "Al dente — gotuj 1 min krócej",
            ("al_dente", ChatLang::Uk) => "Al dente — варіть на 1 хв менше",
            ("check_color", ChatLang::Ru) => "Проверяйте готовность по цвету корочки",
            ("check_color", ChatLang::En) => "Check doneness by crust color",
            ("check_color", ChatLang::Pl) => "Sprawdzaj gotowość po kolorze skórki",
            ("check_color", ChatLang::Uk) => "Перевіряйте готовність за кольором скоринки",
            _ => return None,
        };
        Some(t.to_string())
    };

    let mut add = |text: String, time: Option<u16>, temp_c: Option<u16>, tip_key: Option<&str>| {
        step_num += 1;
        let tip = tip_key.and_then(|k| tip_text(k));
        steps.push(CookingStep { step: step_num, text, time_min: time, temp_c, tip });
    };

    for step_rule in &rule.steps {
        // Collect ingredients that match ANY of the step's roles
        let matching: Vec<&ResolvedIngredient> = step_rule.roles.iter()
            .flat_map(|r| by_role(*r))
            .collect();

        // For steps that need ingredients: skip if none
        let needs_ingredients = matches!(step_rule.step,
            StepType::BoilProtein | StepType::BraiseProtein | StepType::SearProtein
            | StepType::GrillProtein | StepType::MarinateProtein
            | StepType::SauteAromatics | StepType::AddRoots | StepType::AddVegetables
            | StepType::AddAromatics | StepType::BoilBase | StepType::AddBase
            | StepType::AddLiquid | StepType::AddSpices | StepType::ChopAll | StepType::Dress
        );

        if needs_ingredients && matching.is_empty() {
            continue;
        }

        // Special handling for AddRoots: split vegetables into root vs leafy
        if step_rule.step == StepType::AddRoots {
            let roots: Vec<&ResolvedIngredient> = matching.iter()
                .filter(|i| {
                    let slug = i.resolved_slug.as_deref().unwrap_or("");
                    cooking_rules::is_root_vegetable(slug)
                })
                .copied()
                .collect();
            if !roots.is_empty() {
                let names = names_of(&roots, ", ");
                add(cooking_rules::step_text(StepType::AddRoots, &names, lang_code), step_rule.time_min, step_rule.temp_c, step_rule.tip);
            }
            continue;
        }

        // Special handling for AddVegetables in soup/stew: only non-root vegetables
        if step_rule.step == StepType::AddVegetables
            && matches!(dish_type, DishType::Soup | DishType::Stew)
        {
            let leafy: Vec<&ResolvedIngredient> = matching.iter()
                .filter(|i| {
                    let slug = i.resolved_slug.as_deref().unwrap_or("");
                    !cooking_rules::is_root_vegetable(slug)
                })
                .copied()
                .collect();
            if !leafy.is_empty() {
                let names = names_of(&leafy, ", ");
                add(cooking_rules::step_text(StepType::AddVegetables, &names, lang_code), step_rule.time_min, step_rule.temp_c, step_rule.tip);
            }
            continue;
        }

        // SauteAromatics: join with localized "and" (not ", ")
        // Dress: use instrumental case ("заправить майонезом", not "заправить майонез")
        // AddAromatics: only emit if we previously had a SauteAromatics step
        let names = match step_rule.step {
            StepType::SauteAromatics => {
                had_saute = true;
                names_of(&matching, sep)
            },
            StepType::Dress          => names_of_case(&matching, ", ", "instr"),
            StepType::AddAromatics   => {
                if !had_saute {
                    // No sauté step → skip phantom "zasmażka" / "зажарка"
                    continue;
                }
                String::new() // text comes from step_text, not names
            },
            _                        => names_of(&matching, ", "),
        };

        add(cooking_rules::step_text(step_rule.step, &names, lang_code), step_rule.time_min, step_rule.temp_c, step_rule.tip);
    }

    steps
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::dish_schema;
    use super::super::nutrition_math;
    use super::super::display_name;
    use nutrition_math::build_ingredient;

    #[test]
    fn parse_minimal_schema() {
        let json = r#"{"dish":"borscht","dish_local":"Борщ","items":["beet","cabbage","potato","beef"]}"#;
        let s = dish_schema::parse_dish_schema(json).unwrap();
        assert_eq!(s.dish, "borscht");
        assert_eq!(s.items.len(), 4);
        assert_eq!(s.items[0], "beet");
    }

    #[test]
    fn parse_markdown_wrapped() {
        let raw = "```json\n{\"dish\":\"test\",\"items\":[\"a\",\"b\"]}\n```";
        let s = dish_schema::parse_dish_schema(raw).unwrap();
        assert_eq!(s.dish, "test");
        assert_eq!(s.items.len(), 2);
    }

    #[test]
    fn parse_unknown_dish_errors() {
        let json = r#"{"dish":"unknown","items":[]}"#;
        assert!(dish_schema::parse_dish_schema(json).is_err());
    }

    #[test]
    fn edible_yield_potato() {
        let y = nutrition_math::edible_yield_for("vegetable", "potato");
        assert!((y - 0.80).abs() < 0.01);
    }

    #[test]
    fn edible_yield_default() {
        let y = nutrition_math::edible_yield_for("dairy", "milk");
        assert!((y - 1.0).abs() < 0.01);
    }

    #[test]
    fn build_ingredient_beef() {
        let product = IngredientData {
            slug: "beef".into(),
            name_en: "Beef".into(),
            name_ru: "Говядина".into(),
            name_pl: "Wołowina".into(),
            name_uk: "Яловичина".into(),
            calories_per_100g: 250.0,
            protein_per_100g: 26.0,
            fat_per_100g: 15.0,
            carbs_per_100g: 0.0,
            image_url: None,
            product_type: "meat".into(),
            density_g_per_ml: None, behaviors: vec![], states: vec![],
        };
        let resolved = build_ingredient(&product, "beef", HealthGoal::Balanced);

        assert_eq!(resolved.role, "protein");
        assert!(resolved.state == "grilled" || resolved.state == "baked",
            "meat protein should be grilled/baked, got {}", resolved.state);
        assert!((resolved.cooked_net_g - 100.0).abs() < 1.0);
        assert!(resolved.gross_g > resolved.cooked_net_g);
        assert!(resolved.kcal > 0);
        assert!(resolved.protein_g > 20.0);
    }

    #[test]
    fn build_ingredient_vegetable_is_side() {
        let product = IngredientData {
            slug: "beet".into(),
            name_en: "Beet".into(),
            name_ru: "Свёкла".into(),
            name_pl: "Burak".into(),
            name_uk: "Буряк".into(),
            calories_per_100g: 43.0,
            protein_per_100g: 1.6,
            fat_per_100g: 0.2,
            carbs_per_100g: 9.6,
            image_url: None,
            product_type: "vegetable".into(),
            density_g_per_ml: None, behaviors: vec![], states: vec![],
        };
        let resolved = build_ingredient(&product, "beet", HealthGoal::Balanced);

        assert_eq!(resolved.role, "side");
        assert_eq!(resolved.cooked_net_g, 50.0);
        assert!(resolved.gross_g > 55.0);
    }

    #[test]
    fn recipe_portions_are_reasonable() {
        let meat = IngredientData {
            slug: "chicken-breast".into(), name_en: "Chicken".into(),
            name_ru: "".into(), name_pl: "".into(), name_uk: "".into(),
            calories_per_100g: 165.0, protein_per_100g: 31.0,
            fat_per_100g: 3.6, carbs_per_100g: 0.0, image_url: None,
            product_type: "meat".into(), density_g_per_ml: None, behaviors: vec![], states: vec![],
        };
        assert_eq!(nutrition_math::recipe_portion(&meat, "protein"), 100.0);

        let oil = IngredientData {
            slug: "olive-oil".into(), name_en: "Olive Oil".into(),
            name_ru: "".into(), name_pl: "".into(), name_uk: "".into(),
            calories_per_100g: 884.0, protein_per_100g: 0.0,
            fat_per_100g: 100.0, carbs_per_100g: 0.0, image_url: None,
            product_type: "oil".into(), density_g_per_ml: None, behaviors: vec![], states: vec![],
        };
        assert_eq!(nutrition_math::recipe_portion(&oil, "oil"), 15.0);
    }

    #[test]
    fn garlic_is_spice_not_side() {
        let garlic = IngredientData {
            slug: "garlic".into(), name_en: "Garlic".into(),
            name_ru: "Чеснок".into(), name_pl: "Czosnek".into(), name_uk: "Часник".into(),
            calories_per_100g: 149.0, protein_per_100g: 6.4,
            fat_per_100g: 0.5, carbs_per_100g: 33.0, image_url: None,
            product_type: "vegetable".into(), density_g_per_ml: None, behaviors: vec![], states: vec![],
        };
        let resolved = build_ingredient(&garlic, "garlic", HealthGoal::Balanced);
        assert_eq!(resolved.role, "spice", "garlic should be spice, not {}", resolved.role);
        assert_eq!(resolved.cooked_net_g, 5.0, "garlic should be 5g, not {}", resolved.cooked_net_g);
    }

    #[test]
    fn extract_json_from_markdown() {
        let raw = "Sure!\n```json\n{\"dish\":\"x\",\"items\":[]}\n```\nDone.";
        let j = dish_schema::extract_json(raw).unwrap();
        assert!(j.starts_with('{') && j.ends_with('}'));
    }

    #[test]
    fn ru_gender_feminine_soft_sign() {
        // Морковь, фасоль — feminine, ending in -ь
        assert_eq!(display_name::ru_gender("Морковь"), 'f');
        assert_eq!(display_name::ru_gender("Фасоль"), 'f');
        assert_eq!(display_name::ru_gender("Форель"), 'f');
        assert_eq!(display_name::ru_gender("Печень"), 'f');
        // Картофель, имбирь — masculine, ending in -ь
        assert_eq!(display_name::ru_gender("Картофель"), 'm');
        assert_eq!(display_name::ru_gender("Имбирь"), 'm');
    }

    #[test]
    fn state_label_morkov_feminine() {
        let label = display_name::state_label_ru("sauteed", "Морковь");
        assert_eq!(label, "пассерованная");
    }

    #[test]
    fn state_label_kartoshka_feminine() {
        // Картошка ends in -а → feminine
        let label = display_name::state_label_ru("boiled", "Картошка");
        assert_eq!(label, "варёная");
    }

    // ═══ Russian Grammar: compound names ═════════════════════════════════

    #[test]
    fn accusative_simple_nouns() {
        assert_eq!(display_name::accusative_word("Говядина"), "говядину");
        assert_eq!(display_name::accusative_word("Свинина"), "свинину");
        assert_eq!(display_name::accusative_word("Мука"), "муку");
        assert_eq!(display_name::accusative_word("Морковь"), "морковь");  // inanimate
        assert_eq!(display_name::accusative_word("Лук"), "лук");          // inanimate
        assert_eq!(display_name::accusative_word("Чеснок"), "чеснок");    // inanimate
    }

    #[test]
    fn accusative_adjective_aya() {
        // -ая → -ую
        assert_eq!(display_name::accusative_word("Пшеничная"), "пшеничную");
        assert_eq!(display_name::accusative_word("Каменная"), "каменную");
    }

    #[test]
    fn accusative_compound_names() {
        assert_eq!(accusative_phrase("Пшеничная мука"), "пшеничную муку");
        assert_eq!(accusative_phrase("Соль каменная"), "соль каменную");  // adj after noun
        assert_eq!(accusative_phrase("Куриные яйца"), "куриные яйца");   // inanimate pl → unchanged
        assert_eq!(accusative_phrase("Чёрный перец"), "чёрный перец");    // inanimate m → unchanged
        assert_eq!(accusative_phrase("Говядина"), "говядину");
    }

    #[test]
    fn instrumental_simple_nouns() {
        assert_eq!(display_name::instrumental_word("Говядина"), "говядиной");
        assert_eq!(display_name::instrumental_word("Майонез"), "майонезом");
        assert_eq!(display_name::instrumental_word("Масло"), "маслом");
        assert_eq!(display_name::instrumental_word("Морковь"), "морковью");
        assert_eq!(display_name::instrumental_word("Перец"), "перцем");
    }

    #[test]
    fn instrumental_adjectives() {
        assert_eq!(display_name::instrumental_word("Подсолнечное"), "подсолнечным");
        assert_eq!(display_name::instrumental_word("Куриные"), "куриными");
        assert_eq!(display_name::instrumental_word("Чёрный"), "чёрным");
        assert_eq!(display_name::instrumental_word("Пшеничная"), "пшеничной");
    }

    #[test]
    fn instrumental_compound_names() {
        assert_eq!(instrumental_phrase("Подсолнечное масло"), "подсолнечным маслом");
        assert_eq!(instrumental_phrase("Майонез"), "майонезом");
        assert_eq!(instrumental_phrase("Говядина"), "говядиной");
        assert_eq!(instrumental_phrase("Чёрный перец"), "чёрным перцем");
        assert_eq!(instrumental_phrase("Куриные яйца"), "куриными яйцами");
    }

    // ═══ Ingredient deduplication ════════════════════════════════════════

    #[test]
    fn merge_duplicate_water() {
        let mut ingredients = vec![
            ResolvedIngredient {
                product: None,
                slug_hint: "water".into(),
                resolved_slug: Some("water".into()),
                state: "boiled".into(),
                role: "liquid".into(),
                gross_g: 300.0, cleaned_net_g: 300.0, cooked_net_g: 300.0,
                kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
            },
            ResolvedIngredient {
                product: None,
                slug_hint: "water".into(),
                resolved_slug: Some("water".into()),
                state: "boiled".into(),
                role: "liquid".into(),
                gross_g: 300.0, cleaned_net_g: 300.0, cooked_net_g: 300.0,
                kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
            },
            ResolvedIngredient {
                product: None,
                slug_hint: "potato".into(),
                resolved_slug: Some("potato".into()),
                state: "boiled".into(),
                role: "side".into(),
                gross_g: 200.0, cleaned_net_g: 170.0, cooked_net_g: 170.0,
                kcal: 140, protein_g: 3.4, fat_g: 0.2, carbs_g: 31.0,
            },
        ];

        merge_duplicate_ingredients(&mut ingredients);

        assert_eq!(ingredients.len(), 2, "water should be merged into one");
        let water = ingredients.iter().find(|i| i.slug_hint == "water").unwrap();
        assert_eq!(water.gross_g, 600.0, "water grams should be summed");
        let potato = ingredients.iter().find(|i| i.slug_hint == "potato").unwrap();
        assert_eq!(potato.gross_g, 200.0, "potato should be unchanged");
    }
}
