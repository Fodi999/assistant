//! Ingredient Resolver — slug resolution + implicit ingredient insertion.
//!
//! Responsibilities:
//!   - `resolve_slug`: map a Gemini slug hint → real DB product
//!     (exact → plural/singular → aliases → substring)
//!   - `auto_insert_implicit`: add water (soup), oil (sauté) when missing
//!
//! Pure data-lookup, no business logic about portions or КБЖУ.

use crate::infrastructure::IngredientCache;
use crate::infrastructure::ingredient_cache::IngredientData;
use super::response_builder::HealthGoal;
use super::recipe_engine::{ResolvedIngredient, DishType};

// ── Slug Resolution ──────────────────────────────────────────────────────────

pub async fn resolve_slug(cache: &IngredientCache, hint: &str) -> Option<IngredientData> {
    // Normalise: lowercase, spaces/underscores → dashes
    let h = hint.to_lowercase().replace(' ', "-").replace('_', "-");

    // 1. Exact match
    if let Some(p) = cache.get(&h).await { return Some(p); }

    // 2. Plural / singular variants
    //    "noodle" → try "noodles";  "tomatoes" → try "tomato"
    let singular = h.trim_end_matches('s');
    let plural   = format!("{h}s");
    if singular != h {
        if let Some(p) = cache.get(singular).await { return Some(p); }
    }
    if let Some(p) = cache.get(&plural).await { return Some(p); }
    // -es ending: "tomatoes" → "tomato", "potatoes" → "potato"
    if h.ends_with("es") {
        let stem = &h[..h.len() - 2];
        if let Some(p) = cache.get(stem).await { return Some(p); }
    }
    // -ies → -y:  "cherries" → "cherry"
    if h.ends_with("ies") {
        let stem = format!("{}y", &h[..h.len() - 3]);
        if let Some(p) = cache.get(&stem).await { return Some(p); }
    }

    // 3. Synonym / alias rewrites (EXACT slug → slug)
    //    Only for slugs that DON'T exist in the DB themselves.
    //    If the real product IS in the DB, step 1/2 already found it.
    let aliases: &[(&str, &str)] = &[
        // Vegetables
        ("beet",          "beetroot"),
        ("beets",         "beetroot"),
        ("green-onion",   "onion"),
        ("spring-onion",  "onion"),
        ("scallion",      "onion"),
        ("shallot",       "onion"),
        ("celery-root",   "celery"),
        ("celeriac",      "celery"),
        ("bell-pepper",   "bell-pepper"),
        ("sweet-pepper",  "bell-pepper"),
        ("chili",         "black-pepper"),
        ("chilli",        "black-pepper"),
        ("jalapeno",      "black-pepper"),
        // Proteins
        ("chicken",       "chicken-breast"),
        ("chicken-leg",   "chicken-thighs"),
        ("chicken-wing",  "chicken-thighs"),
        ("chicken-drumstick", "chicken-thighs"),
        ("minced-meat",   "ground-meat"),
        ("ground-beef",   "ground-meat"),
        ("mince",         "ground-meat"),
        // Dairy
        ("cream",         "sour-cream"),
        ("heavy-cream",   "sour-cream"),
        ("cream-cheese",  "cottage-cheese"),
        ("ricotta",       "cottage-cheese"),
        ("parmesan",      "hard-cheese"),
        ("cheddar",       "hard-cheese"),
        ("cheese",        "hard-cheese"),
        ("egg",           "chicken-eggs"),
        ("eggs",          "chicken-eggs"),
        // Grains & pasta
        ("vermicelli",    "noodles"),
        ("spaghetti",     "pasta"),
        ("penne",         "pasta"),
        ("macaroni",      "pasta"),
        ("fettuccine",    "pasta"),
        ("linguine",      "pasta"),
        ("flour",         "wheat-flour"),
        ("all-purpose-flour", "wheat-flour"),
        // Liquids & sauces
        ("stock",         "water"),
        ("broth",         "water"),
        ("chicken-stock", "water"),
        ("chicken-broth", "water"),
        ("vegetable-stock", "water"),
        ("vegetable-broth", "water"),
        ("bouillon",      "water"),
        ("mineral-water", "water"),
        // Oils
        ("oil",           "sunflower-oil"),
        ("vegetable-oil", "sunflower-oil"),
        ("canola-oil",    "rapeseed-oil"),
        // Spices & herbs
        ("cilantro",      "parsley"),
        ("coriander",     "parsley"),
        ("bay-leaves",    "bay-leaf"),
        ("peppercorn",    "black-pepper"),
        ("peppercorns",   "black-pepper"),
        ("allspice",      "black-pepper"),
        ("paprika",       "sweet-paprika"),
        ("sea-salt",      "salt"),
        ("kosher-salt",   "salt"),
        ("table-salt",    "salt"),
        ("cornstarch",    "wheat-flour"),
        ("corn-starch",   "wheat-flour"),
        ("baking-soda",   "baking-powder"),
        // Canned / processed
        ("tomato-paste",  "canned-tomatoes"),
        ("tomato-sauce",  "canned-tomatoes"),
        ("crushed-tomatoes", "canned-tomatoes"),
        ("diced-tomatoes", "canned-tomatoes"),
        // Mushrooms
        ("mushroom",      "button-mushroom"),
        ("mushrooms",     "button-mushroom"),
        ("champignon",    "button-mushroom"),
        ("portobello",    "button-mushroom"),
        ("shiitake",      "porcini-mushroom"),
        // Nuts & seeds
        ("almond",        "almonds"),
        ("walnut",        "walnuts"),
        ("hazelnut",      "hazelnuts"),
        ("peanut",        "almonds"),
        ("peanuts",       "almonds"),
        ("sesame",        "sesame-seeds"),
        // Drinks
        ("wine",          "red-wine"),
        ("white-wine",    "white-wine"),
    ];

    // Exact alias match first (fast path)
    for (from, to) in aliases {
        if h == *from {
            if let Some(p) = cache.get(to).await { return Some(p); }
        }
    }
    // Partial alias match (e.g. "chicken-stock-cube" contains "chicken-stock")
    for (from, to) in aliases {
        if h.contains(from) && h != *from {
            if let Some(p) = cache.get(to).await { return Some(p); }
        }
    }

    // 4. Substring match against all slugs and English names
    let all = cache.all().await;
    for p in &all {
        if p.slug.contains(&h) || h.contains(&p.slug) {
            return Some(p.clone());
        }
        if p.name_en.to_lowercase().contains(&h) {
            return Some(p.clone());
        }
    }

    None
}

// ── Auto-insert Implicit Ingredients ─────────────────────────────────────────

/// Automatically add implicit ingredients that a recipe logically needs but
/// Gemini doesn't include (water for soup, oil for sauté).
pub async fn auto_insert_implicit(
    ingredients: &mut Vec<ResolvedIngredient>,
    dish_type: DishType,
    cache: &IngredientCache,
    goal: HealthGoal,
) {
    let has_liquid = ingredients.iter().any(|i| i.role == "liquid");
    let has_oil = ingredients.iter().any(|i| i.role == "oil");
    let has_oil_slug = ingredients.iter().any(|i| {
        i.resolved_slug.as_deref() == Some("sunflower-oil")
            || i.resolved_slug.as_deref() == Some("olive-oil")
            || i.slug_hint == "sunflower-oil"
            || i.slug_hint == "olive-oil"
    });
    let has_saute = ingredients.iter().any(|i| i.state == "sauteed");

    // ── Soup/Stew: add water (300ml per serving) if no liquid ───────────
    if matches!(dish_type, DishType::Soup | DishType::Stew) && !has_liquid {
        let water_product = resolve_slug(cache, "water").await;
        ingredients.push(ResolvedIngredient {
            product: water_product,
            slug_hint: "water".into(),
            resolved_slug: Some("water".into()),
            state: "boiled".into(),
            role: "liquid".into(),
            gross_g: 300.0,
            cleaned_net_g: 300.0,
            cooked_net_g: 300.0,
            kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
        });
    }

    // ── Sauté dishes: add cooking oil if none present ───────────────────
    if has_saute && !has_oil && !has_oil_slug {
        let portion = match goal {
            HealthGoal::LowCalorie => 5.0_f32,
            _ => 15.0_f32,
        };
        if let Some(oil) = resolve_slug(cache, "sunflower-oil").await {
            ingredients.push(ResolvedIngredient {
                product: Some(oil.clone()),
                slug_hint: "sunflower-oil".into(),
                resolved_slug: Some(oil.slug.clone()),
                state: "raw".into(),
                role: "oil".into(),
                gross_g: portion,
                cleaned_net_g: portion,
                cooked_net_g: portion,
                kcal: oil.kcal_for(portion),
                protein_g: oil.protein_for(portion),
                fat_g: oil.fat_for(portion),
                carbs_g: oil.carbs_for(portion),
            });
        } else {
            let fallback_kcal = (portion * 9.0) as u32;
            ingredients.push(ResolvedIngredient {
                product: None,
                slug_hint: "sunflower-oil".into(),
                resolved_slug: Some("sunflower-oil".into()),
                state: "raw".into(),
                role: "oil".into(),
                gross_g: portion,
                cleaned_net_g: portion,
                cooked_net_g: portion,
                kcal: fallback_kcal, protein_g: 0.0, fat_g: portion, carbs_g: 0.0,
            });
        }
    }
}
