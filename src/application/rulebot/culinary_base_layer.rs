//! Culinary Base Layer — the "physics of cooking".
//!
//! Every dish needs a foundation: salt, fat, sometimes acid and pepper.
//! This layer ensures recipes feel real and professional by auto-inserting
//! fundamental ingredients that any cook would add without thinking.
//!
//! Called after `auto_insert_implicit` and before constraint_policy.
//!
//! Design:
//!   1. ALWAYS: salt + fat (unless raw/salad with existing dressing)
//!   2. BY DISH TYPE: pepper, garlic, onion, acid, sugar
//!   3. FLAVOR BALANCE: sweet dishes get salt pinch, savory get pepper
//!
//! Pure deterministic — no AI, no DB queries (cache only).

use crate::infrastructure::IngredientCache;
use crate::infrastructure::ingredient_cache::IngredientData;
use super::recipe_engine::{ResolvedIngredient, DishType};
use super::response_builder::HealthGoal;
use super::ingredient_resolver::resolve_slug;

// ── Public API ───────────────────────────────────────────────────────────────

/// Apply culinary basics to a resolved ingredient list.
/// Ensures every dish has salt, fat, and type-appropriate seasonings.
pub async fn apply_culinary_basics(
    ingredients: &mut Vec<ResolvedIngredient>,
    dish_type: DishType,
    cache: &IngredientCache,
    goal: HealthGoal,
) {
    // ── 1. ALWAYS: salt ──────────────────────────────────────────────────
    if !has_slug(ingredients, "salt") {
        let grams = match dish_type {
            DishType::Salad | DishType::Raw => 1.0,
            DishType::Soup | DishType::Stew => 3.0,
            _ => 2.0,
        };
        ensure_ingredient(ingredients, cache, "salt", grams, "seasoning", "raw").await;
    }

    // ── 2. ALWAYS: fat (unless Raw) ─────────────────────────────────────
    if !matches!(dish_type, DishType::Raw) && !has_any_oil(ingredients) {
        let (slug, grams) = match (dish_type, goal) {
            (DishType::Salad, _) => ("olive-oil", 10.0_f32),
            (DishType::Grill, HealthGoal::LowCalorie) => ("sunflower-oil", 5.0),
            (DishType::Grill, _) => ("sunflower-oil", 10.0),
            (DishType::StirFry, HealthGoal::LowCalorie) => ("sunflower-oil", 8.0),
            (DishType::StirFry, _) => ("sunflower-oil", 15.0),
            (DishType::Bake, HealthGoal::LowCalorie) => ("sunflower-oil", 5.0),
            (DishType::Bake, _) => ("sunflower-oil", 10.0),
            (DishType::Pasta, _) => ("olive-oil", 10.0),
            (_, HealthGoal::LowCalorie) => ("sunflower-oil", 5.0),
            (_, _) => ("sunflower-oil", 10.0),
        };
        ensure_ingredient(ingredients, cache, slug, grams, "oil", "raw").await;
    }

    // ── 3. BY DISH TYPE: pepper ─────────────────────────────────────────
    let needs_pepper = matches!(
        dish_type,
        DishType::Soup | DishType::Stew | DishType::StirFry |
        DishType::Grill | DishType::Bake | DishType::Pasta | DishType::Default
    );
    if needs_pepper && !has_slug(ingredients, "black-pepper") {
        ensure_ingredient(ingredients, cache, "black-pepper", 1.0, "seasoning", "raw").await;
    }

    // ── 4. BY DISH TYPE: garlic ─────────────────────────────────────────
    let needs_garlic = matches!(
        dish_type,
        DishType::StirFry | DishType::Pasta | DishType::Stew | DishType::Grill
    );
    if needs_garlic && !has_slug(ingredients, "garlic") {
        ensure_ingredient(ingredients, cache, "garlic", 5.0, "aromatic", "raw").await;
    }

    // ── 5. BY DISH TYPE: onion (soups, stews, stir-fry) ────────────────
    let needs_onion = matches!(
        dish_type,
        DishType::Soup | DishType::Stew | DishType::StirFry
    );
    if needs_onion && !has_any_onion(ingredients) {
        ensure_ingredient(ingredients, cache, "onion", 50.0, "aromatic", "sauteed").await;
    }

    // ── 6. Salad: acid (lemon or vinegar) ───────────────────────────────
    if matches!(dish_type, DishType::Salad) && !has_acid(ingredients) {
        ensure_ingredient(ingredients, cache, "lemon", 15.0, "acid", "raw").await;
    }

    // ── 7. Flavor balance: sweet ingredient → ensure salt+pepper ────────
    if has_sweet_ingredient(ingredients) {
        // Already added salt above; just bump pepper slightly
        if let Some(pepper) = ingredients.iter_mut().find(|i| slug_matches(i, "black-pepper")) {
            if pepper.gross_g < 0.5 {
                pepper.gross_g = 0.5;
                pepper.cleaned_net_g = 0.5;
                pepper.cooked_net_g = 0.5;
            }
        } else {
            ensure_ingredient(ingredients, cache, "black-pepper", 0.5, "seasoning", "raw").await;
        }
    }

    // ── 8. Soup: bay leaf ───────────────────────────────────────────────
    if matches!(dish_type, DishType::Soup | DishType::Stew) && !has_slug(ingredients, "bay-leaf") {
        ensure_ingredient(ingredients, cache, "bay-leaf", 1.0, "seasoning", "raw").await;
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn has_slug(ingredients: &[ResolvedIngredient], slug: &str) -> bool {
    ingredients.iter().any(|i| {
        i.resolved_slug.as_deref() == Some(slug)
            || i.slug_hint == slug
    })
}

fn slug_matches(i: &ResolvedIngredient, slug: &str) -> bool {
    i.resolved_slug.as_deref() == Some(slug) || i.slug_hint == slug
}

fn has_any_oil(ingredients: &[ResolvedIngredient]) -> bool {
    ingredients.iter().any(|i| {
        i.role == "oil"
            || slug_matches(i, "sunflower-oil")
            || slug_matches(i, "olive-oil")
            || slug_matches(i, "rapeseed-oil")
            || slug_matches(i, "butter")
            || slug_matches(i, "coconut-oil")
            || i.slug_hint.contains("oil")
            || i.slug_hint.contains("butter")
    })
}

fn has_any_onion(ingredients: &[ResolvedIngredient]) -> bool {
    ingredients.iter().any(|i| {
        slug_matches(i, "onion")
            || i.slug_hint.contains("onion")
            || i.slug_hint.contains("shallot")
            || i.slug_hint.contains("leek")
    })
}

fn has_acid(ingredients: &[ResolvedIngredient]) -> bool {
    ingredients.iter().any(|i| {
        i.role == "acid"
            || slug_matches(i, "lemon")
            || slug_matches(i, "lime")
            || slug_matches(i, "vinegar")
            || slug_matches(i, "wine-vinegar")
            || slug_matches(i, "balsamic-vinegar")
            || i.slug_hint.contains("lemon")
            || i.slug_hint.contains("lime")
            || i.slug_hint.contains("vinegar")
    })
}

fn has_sweet_ingredient(ingredients: &[ResolvedIngredient]) -> bool {
    let sweet_hints = [
        "apple", "pear", "peach", "apricot", "mango", "banana",
        "berry", "strawberry", "raspberry", "blueberry", "cherry",
        "honey", "sugar", "maple", "date", "fig", "plum",
        "pineapple", "orange", "melon", "grape", "kiwi",
    ];
    ingredients.iter().any(|i| {
        let h = i.slug_hint.to_lowercase();
        sweet_hints.iter().any(|s| h.contains(s))
    })
}

/// Add an ingredient if not already present, resolving from cache for proper КБЖУ.
async fn ensure_ingredient(
    ingredients: &mut Vec<ResolvedIngredient>,
    cache: &IngredientCache,
    slug: &str,
    grams: f32,
    role: &str,
    state: &str,
) {
    // Don't duplicate
    if has_slug(ingredients, slug) {
        return;
    }

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
            // Fallback: insert with zero nutrition (still shows in recipe)
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
