//! Culinary Base Layer — the "physics of cooking".
//!
//! Every dish needs a foundation: salt, fat, sometimes acid and pepper.
//! This layer ensures recipes feel real and professional by auto-inserting
//! fundamental ingredients that any cook would add without thinking.
//!
//! Called after `auto_insert_implicit` and before constraint_policy.
//!
//! v2 Design (weight-proportional):
//!   1. Salt = ~0.8% of total food weight (industry standard)
//!   2. Fat  = proportional to protein mass + dish type
//!   3. Pepper/garlic/onion = context-aware (not added to delicate/sweet dishes)
//!   4. Cooking logic validation (does this dish make sense?)
//!
//! Pure deterministic — no AI, no DB queries (cache only).

use crate::infrastructure::IngredientCache;
use super::recipe_engine::{ResolvedIngredient, DishType};
use super::response_builder::HealthGoal;
use super::ingredient_resolver::resolve_slug;

// ── Weight Analysis ──────────────────────────────────────────────────────────

/// Collected metrics from existing ingredients to drive proportional calculations.
struct DishMetrics {
    food_weight_g: f32,
    protein_weight_g: f32,
    vegetable_weight_g: f32,
    has_sweet: bool,
    has_delicate_protein: bool,
    has_allium: bool,
    main_ingredient_count: usize,
}

fn analyze_dish(ingredients: &[ResolvedIngredient]) -> DishMetrics {
    let mut m = DishMetrics {
        food_weight_g: 0.0,
        protein_weight_g: 0.0,
        vegetable_weight_g: 0.0,
        has_sweet: false,
        has_delicate_protein: false,
        has_allium: false,
        main_ingredient_count: 0,
    };

    for i in ingredients {
        let slug = i.slug_hint.to_lowercase();
        let role = i.role.as_str();

        if role == "liquid" || slug == "water" {
            continue;
        }

        m.food_weight_g += i.gross_g;

        if role == "protein" || is_protein_slug(&slug) {
            m.protein_weight_g += i.gross_g;
            if is_delicate_protein(&slug) {
                m.has_delicate_protein = true;
            }
        }

        if role == "vegetable" || role == "aromatic" || is_vegetable_slug(&slug) {
            m.vegetable_weight_g += i.gross_g;
        }

        if is_sweet_slug(&slug) {
            m.has_sweet = true;
        }

        if is_allium_slug(&slug) {
            m.has_allium = true;
        }

        if !matches!(role, "seasoning" | "oil" | "liquid" | "acid") {
            m.main_ingredient_count += 1;
        }
    }

    m
}

// ── Public API ───────────────────────────────────────────────────────────────

pub async fn apply_culinary_basics(
    ingredients: &mut Vec<ResolvedIngredient>,
    dish_type: DishType,
    cache: &IngredientCache,
    goal: HealthGoal,
) {
    let metrics = analyze_dish(ingredients);

    if metrics.food_weight_g < 10.0 {
        return;
    }

    // ── 1. SALT — proportional to food weight ────────────────────────────
    if !has_slug(ingredients, "salt") {
        let salt_ratio = match dish_type {
            DishType::Salad | DishType::Raw => 0.005,
            DishType::Soup | DishType::Stew => 0.010,
            _ => 0.008,
        };
        let salt_g = (metrics.food_weight_g * salt_ratio).clamp(0.5, 8.0);
        ensure_ingredient(ingredients, cache, "salt", salt_g, "seasoning", "raw").await;
    }

    // ── 2. FAT — proportional to protein + dish type ─────────────────────
    if !matches!(dish_type, DishType::Raw) && !has_any_oil(ingredients) {
        let (slug, oil_g) = compute_oil(dish_type, goal, &metrics);
        ensure_ingredient(ingredients, cache, slug, oil_g, "oil", "raw").await;
    }

    // ── 3. BLACK PEPPER — skip for sweet+delicate combos ─────────────────
    let needs_pepper = matches!(
        dish_type,
        DishType::Soup | DishType::Stew | DishType::StirFry |
        DishType::Grill | DishType::Bake | DishType::Pasta | DishType::Default
    );
    let pepper_ok = !metrics.has_sweet || !metrics.has_delicate_protein;
    if needs_pepper && pepper_ok && !has_slug(ingredients, "black-pepper") {
        let pepper_g = (metrics.food_weight_g * 0.001).clamp(0.3, 2.0);
        ensure_ingredient(ingredients, cache, "black-pepper", pepper_g, "seasoning", "raw").await;
    }

    // ── 4. GARLIC — only robust dishes, no sweet/delicate ────────────────
    let garlic_ok = matches!(
        dish_type,
        DishType::StirFry | DishType::Pasta | DishType::Stew
    ) && !metrics.has_sweet && !metrics.has_delicate_protein;

    if garlic_ok && !has_slug(ingredients, "garlic") {
        let garlic_g = (metrics.food_weight_g * 0.01).clamp(3.0, 10.0);
        ensure_ingredient(ingredients, cache, "garlic", garlic_g, "aromatic", "raw").await;
    }

    // ── 5. ONION — soups/stews (proportional), stir-fry (smaller) ────────
    if !metrics.has_allium && !metrics.has_sweet {
        match dish_type {
            DishType::Soup | DishType::Stew => {
                let onion_g = (metrics.food_weight_g * 0.12).clamp(30.0, 120.0);
                ensure_ingredient(ingredients, cache, "onion", onion_g, "aromatic", "sauteed").await;
            }
            DishType::StirFry => {
                let onion_g = (metrics.food_weight_g * 0.08).clamp(20.0, 60.0);
                ensure_ingredient(ingredients, cache, "onion", onion_g, "aromatic", "sauteed").await;
            }
            _ => {}
        }
    }

    // ── 6. ACID — salad dressing ─────────────────────────────────────────
    if matches!(dish_type, DishType::Salad) && !has_acid(ingredients) {
        let acid_g = (metrics.food_weight_g * 0.03).clamp(5.0, 20.0);
        ensure_ingredient(ingredients, cache, "lemon", acid_g, "acid", "raw").await;
    }

    // ── 7. SWEET-SAVORY BALANCE: bump pepper for contrast ────────────────
    if metrics.has_sweet && !matches!(dish_type, DishType::Salad | DishType::Raw) {
        if let Some(pepper) = ingredients.iter_mut().find(|i| slug_matches(i, "black-pepper")) {
            if pepper.gross_g < 0.5 {
                pepper.gross_g = 0.5;
                pepper.cleaned_net_g = 0.5;
                pepper.cooked_net_g = 0.5;
            }
        }
    }

    // ── 8. BAY LEAF — long-cooked liquid dishes ─────────────────────────
    if matches!(dish_type, DishType::Soup | DishType::Stew) && !has_slug(ingredients, "bay-leaf") {
        ensure_ingredient(ingredients, cache, "bay-leaf", 1.0, "seasoning", "raw").await;
    }
}

// ── Cooking Logic Validation ─────────────────────────────────────────────────

pub fn validate_cooking_logic(
    ingredients: &[ResolvedIngredient],
    dish_type: DishType,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let metrics = analyze_dish(ingredients);

    // 1. Cooking technique check
    let has_cooked = ingredients.iter().any(|i| !matches!(i.state.as_str(), "raw" | ""));
    if !has_cooked && !matches!(dish_type, DishType::Salad | DishType::Raw) {
        warnings.push("No cooking technique detected — all ingredients are raw".into());
    }

    // 2. Protein check for non-salad dishes
    if metrics.protein_weight_g < 1.0
        && !matches!(dish_type, DishType::Salad | DishType::Raw)
        && metrics.main_ingredient_count > 1
    {
        warnings.push("No protein source — dish may not be filling".into());
    }

    // 3. Suspicious combo: garlic + sweet fruit + delicate protein
    if has_slug(ingredients, "garlic") && metrics.has_sweet && metrics.has_delicate_protein {
        warnings.push("Garlic + sweet fruit + delicate protein — unusual combo".into());
    }

    // 4. Aromatic overload
    let aromatic_count = ingredients.iter().filter(|i| i.role == "aromatic").count();
    if aromatic_count > metrics.main_ingredient_count && metrics.main_ingredient_count > 0 {
        warnings.push("More aromatics than main ingredients — may overpower".into());
    }

    // 5. Very low total weight
    if metrics.food_weight_g < 50.0 && metrics.main_ingredient_count >= 2 {
        warnings.push(format!(
            "Total food weight {:.0}g seems too low for {} ingredients",
            metrics.food_weight_g, metrics.main_ingredient_count
        ));
    }

    warnings
}

// ── Oil Calculation ──────────────────────────────────────────────────────────

fn compute_oil(dish_type: DishType, goal: HealthGoal, metrics: &DishMetrics) -> (&'static str, f32) {
    let low_cal = matches!(goal, HealthGoal::LowCalorie);

    match dish_type {
        DishType::Salad => {
            let g = (metrics.vegetable_weight_g * 0.05).clamp(5.0, 15.0);
            ("olive-oil", g)
        }
        DishType::StirFry => {
            let base = (metrics.protein_weight_g * 0.05).clamp(8.0, 20.0);
            ("sunflower-oil", if low_cal { base * 0.6 } else { base })
        }
        DishType::Grill => {
            let base = (metrics.protein_weight_g * 0.03).clamp(5.0, 15.0);
            ("sunflower-oil", if low_cal { base * 0.5 } else { base })
        }
        DishType::Bake => {
            let base = (metrics.food_weight_g * 0.03).clamp(5.0, 15.0);
            ("sunflower-oil", if low_cal { base * 0.5 } else { base })
        }
        DishType::Pasta => {
            let g = (metrics.food_weight_g * 0.03).clamp(5.0, 15.0);
            ("olive-oil", g)
        }
        DishType::Soup | DishType::Stew => {
            let base = (metrics.vegetable_weight_g * 0.05).clamp(5.0, 15.0);
            ("sunflower-oil", if low_cal { base * 0.6 } else { base })
        }
        _ => {
            let base = (metrics.food_weight_g * 0.03).clamp(5.0, 15.0);
            ("sunflower-oil", if low_cal { base * 0.5 } else { base })
        }
    }
}

// ── Slug Helpers ─────────────────────────────────────────────────────────────

fn has_slug(ingredients: &[ResolvedIngredient], slug: &str) -> bool {
    ingredients.iter().any(|i| i.resolved_slug.as_deref() == Some(slug) || i.slug_hint == slug)
}

fn slug_matches(i: &ResolvedIngredient, slug: &str) -> bool {
    i.resolved_slug.as_deref() == Some(slug) || i.slug_hint == slug
}

fn has_any_oil(ingredients: &[ResolvedIngredient]) -> bool {
    ingredients.iter().any(|i| {
        i.role == "oil"
            || slug_matches(i, "sunflower-oil") || slug_matches(i, "olive-oil")
            || slug_matches(i, "rapeseed-oil") || slug_matches(i, "butter")
            || slug_matches(i, "coconut-oil")
            || i.slug_hint.contains("oil") || i.slug_hint.contains("butter")
    })
}

fn is_allium_slug(slug: &str) -> bool {
    slug.contains("onion") || slug.contains("shallot")
        || slug.contains("leek") || slug.contains("scallion")
}

fn has_acid(ingredients: &[ResolvedIngredient]) -> bool {
    ingredients.iter().any(|i| {
        i.role == "acid"
            || slug_matches(i, "lemon") || slug_matches(i, "lime")
            || slug_matches(i, "vinegar") || slug_matches(i, "wine-vinegar")
            || slug_matches(i, "balsamic-vinegar")
            || i.slug_hint.contains("lemon") || i.slug_hint.contains("lime")
            || i.slug_hint.contains("vinegar")
    })
}

fn is_protein_slug(slug: &str) -> bool {
    ["chicken", "beef", "pork", "lamb", "turkey", "duck",
     "salmon", "tuna", "cod", "shrimp", "prawn", "squid",
     "egg", "tofu", "tempeh", "lentil", "chickpea", "bean",
     "ground-meat", "mince", "sausage", "bacon"]
        .iter().any(|k| slug.contains(k))
}

fn is_delicate_protein(slug: &str) -> bool {
    ["salmon", "tuna", "cod", "sole", "trout", "sea-bass",
     "shrimp", "prawn", "scallop", "crab", "lobster"]
        .iter().any(|k| slug.contains(k))
}

fn is_vegetable_slug(slug: &str) -> bool {
    ["tomato", "onion", "carrot", "pepper", "cabbage", "cucumber",
     "broccoli", "cauliflower", "spinach", "lettuce", "zucchini",
     "eggplant", "celery", "beet", "radish", "pea", "corn",
     "mushroom", "potato", "sweet-potato"]
        .iter().any(|k| slug.contains(k))
}

fn is_sweet_slug(slug: &str) -> bool {
    ["apple", "pear", "peach", "apricot", "mango", "banana",
     "berry", "strawberry", "raspberry", "blueberry", "cherry",
     "honey", "sugar", "maple", "date", "fig", "plum",
     "pineapple", "orange", "melon", "grape", "kiwi"]
        .iter().any(|s| slug.contains(s))
}

async fn ensure_ingredient(
    ingredients: &mut Vec<ResolvedIngredient>,
    cache: &IngredientCache,
    slug: &str,
    grams: f32,
    role: &str,
    state: &str,
) {
    if has_slug(ingredients, slug) { return; }

    match resolve_slug(cache, slug).await {
        Some(product) => {
            ingredients.push(ResolvedIngredient {
                product: Some(product.clone()),
                slug_hint: slug.to_string(),
                resolved_slug: Some(product.slug.clone()),
                state: state.to_string(),
                role: role.to_string(),
                gross_g: grams,
                cleaned_net_g: grams,
                cooked_net_g: grams,
                kcal: product.kcal_for(grams),
                protein_g: product.protein_for(grams),
                fat_g: product.fat_for(grams),
                carbs_g: product.carbs_for(grams),
            });
        }
        None => {
            ingredients.push(ResolvedIngredient {
                product: None,
                slug_hint: slug.to_string(),
                resolved_slug: Some(slug.to_string()),
                state: state.to_string(),
                role: role.to_string(),
                gross_g: grams,
                cleaned_net_g: grams,
                cooked_net_g: grams,
                kcal: 0,
                protein_g: 0.0,
                fat_g: if role == "oil" { grams } else { 0.0 },
                carbs_g: 0.0,
            });
        }
    }
}
