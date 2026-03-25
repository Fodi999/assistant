//! Culinary Rules — pre-filter + scoring boost for candidate pool.
//!
//! Sits between raw DB pairings and suggestion_engine:
//!
//!   DB → flavor → pairings → candidates
//!                                ↓
//!                      🔥 culinary_rules::apply()   ← HERE
//!                                ↓
//!                        suggestion_engine
//!                                ↓
//!                         recipe_builder
//!
//! Design principles:
//! - No hard blocks — only penalties (score -= N) for bad pairings.
//!   Hard blocks destroy valid edge-cases (salmon + cream sauce, tuna melt).
//! - Category detection uses `product_type` field first (stable DB value),
//!   then falls back to slug keyword matching as a safety net.
//! - Safety fallback: if penalties reduce pool below MIN_POOL, return
//!   the original candidates unchanged so recipe_builder always has material.
//! - meal_type + diet add contextual intelligence (breakfast → eggs boost,
//!   vegan → filter meat/dairy, etc.)
//!
//! Pure deterministic functions — no DB, no AI, no HTTP.
//! One public function: `apply(main, candidates, extras, state, meal_type, diet) -> Vec<Candidate>`

use crate::domain::tools::suggestion_engine::Candidate;
use super::context::{MealType, Diet};

/// Minimum number of candidates we must keep after filtering.
/// If result drops below this, return original pool unchanged.
const MIN_POOL: usize = 3;

// ── Public API ───────────────────────────────────────────────────────────────

/// Apply culinary rules to a raw candidate pool:
/// 1. Filter by diet (hard constraint — remove incompatible)
/// 2. Penalise bad pairings (not hard-block)
/// 3. Penalise liquids that can't be dish sides
/// 4. Boost contextually excellent pairings
/// 5. Boost by meal_type context (breakfast → eggs/oats, dinner → protein)
/// 6. Drop only candidates with score ≤ 0 after adjustments
/// 7. Safety fallback: if too few remain, return original pool
///
/// `state` — cooking state of the main ingredient ("raw", "grilled", "baked", …).
/// `meal_type` — meal occasion (breakfast, lunch, dinner, snack, dessert).
/// `diet` — dietary restriction (vegan, vegetarian, gluten_free, etc.).
pub fn apply(
    main: &str,
    candidates: &[Candidate],
    extras: &[String],
    state: Option<&str>,
    meal_type: Option<MealType>,
    diet: Diet,
) -> Vec<Candidate> {
    let adjusted: Vec<Candidate> = candidates
        .iter()
        // ── Diet hard filter (before scoring) ────────────────────────────────
        .filter(|c| passes_diet_filter(c, diet))
        .map(|c| {
            let mut c = c.clone();
            apply_penalties(main, &mut c, state);
            apply_boosts(main, &mut c, extras);
            apply_meal_type_boost(&mut c, meal_type);
            c.pair_score = c.pair_score.min(10.0).max(0.0);
            c
        })
        .filter(|c| c.pair_score > 0.0)
        .collect();

    // Safety fallback: never starve recipe_builder of candidates
    if adjusted.len() < MIN_POOL {
        return candidates.to_vec();
    }

    adjusted
}

// ── Penalties (replace hard blocks) ─────────────────────────────────────────
//
// Penalties instead of removal because:
//   salmon + cream = classic ✅  →  would be removed by hard fish+dairy block
//   tuna melt      = classic ✅  →  same
//   fish chowder   = classic ✅  →  same
// A penalty keeps these but makes them rank lower than genuinely good pairs.

fn apply_penalties(main: &str, c: &mut Candidate, state: Option<&str>) {
    let m_slug = main.to_lowercase();
    let c_slug = c.slug.to_lowercase();

    // State-aware main category:
    // salmon "raw" → RawFish (no acid fruit, no sweet fruit)
    // salmon "grilled/baked/…" → Fish (normal cooked fish)
    let is_cooked = matches!(
        state.unwrap_or("raw"),
        "grilled" | "baked" | "fried" | "steamed" | "boiled" | "roasted" | "smoked"
    );
    let m_cat = if is_cooked {
        // Promote RawFish → Fish when cooked (cream sauce is fine with cooked salmon)
        let raw_cat = category_of_slug(&m_slug, None);
        if raw_cat == Cat::RawFish { Cat::Fish } else { raw_cat }
    } else {
        category_of_slug(&m_slug, None)
    };
    let c_cat = category_of_slug(&c_slug, c.product_type.as_deref());

    // ── Culinary mismatch penalties ───────────────────────────────────────────

    // Fish + dairy: mismatch in most contexts (-3)
    // Exceptions: cream sauce, butter baste, fish chowder — still appear
    // because -3 won't push a high-scoring candidate below zero.
    if m_cat == Cat::Fish && c_cat == Cat::Dairy {
        c.pair_score -= 3.0;
    }
    if m_cat == Cat::Dairy && c_cat == Cat::Fish {
        c.pair_score -= 3.0;
    }

    // Fish + red meat: very niche (-4)
    if m_cat == Cat::Fish && c_cat == Cat::RedMeat {
        c.pair_score -= 4.0;
    }
    if m_cat == Cat::RedMeat && c_cat == Cat::Fish {
        c.pair_score -= 4.0;
    }

    // Raw fish + sweet fruit: bad except ceviche (-3)
    if m_cat == Cat::RawFish && c_cat == Cat::SweetFruit {
        c.pair_score -= 3.0;
    }
    if m_cat == Cat::SweetFruit && c_cat == Cat::RawFish {
        c.pair_score -= 3.0;
    }

    // Sweet fruit + strong alliums (-3)
    if m_cat == Cat::SweetFruit && c_cat == Cat::Allium {
        c.pair_score -= 3.0;
    }
    if m_cat == Cat::Allium && c_cat == Cat::SweetFruit {
        c.pair_score -= 3.0;
    }

    // Liquid milk + high-acid fruit: curdles (-4)
    if is_liquid_milk(&m_slug) && c_cat == Cat::HighAcidFruit {
        c.pair_score -= 4.0;
    }
    if is_liquid_milk(&c_slug) && m_cat == Cat::HighAcidFruit {
        c.pair_score -= 4.0;
    }

    // ── Liquid penalty — the main fix for "salmon + milk" ────────────────────
    //
    // Liquids (milk, broth, juice, water, stock) have calories and protein
    // so they pass all nutrition filters — but they can't be dish sides.
    // cream is excluded: it's a valid sauce/fat component.

    if is_cooking_liquid(&c_slug) {
        // Base liquid penalty: liquids are not dish components
        c.pair_score -= 2.5;

        // Extra context penalty: liquid is only acceptable when main is a grain
        // (porridge, risotto, pasta) or dessert — everywhere else it's wrong
        if m_cat != Cat::Grain {
            c.pair_score -= 2.0;
        }
    }
}

// ── Boosts ───────────────────────────────────────────────────────────────────

fn apply_boosts(main: &str, c: &mut Candidate, extras: &[String]) {
    let m = main.to_lowercase();
    let s = c.slug.to_lowercase();
    let m_cat = category_of_slug(&m, None);

    // Fish context
    if m_cat == Cat::Fish || m_cat == Cat::RawFish {
        if is_acid_slug(&s)  { c.pair_score += 2.0; } // lemon, lime, vinegar
        if is_herb_slug(&s)  { c.pair_score += 1.5; } // dill, parsley, cilantro
        if s.contains("rice") || s.contains("quinoa") { c.pair_score += 1.0; }
    }

    // Chicken
    if m.contains("chicken") {
        if s.contains("garlic")   { c.pair_score += 2.0; }
        if s.contains("lemon")    { c.pair_score += 1.5; }
        if s.contains("rosemary") || s.contains("thyme") { c.pair_score += 1.5; }
        if s.contains("potato")   { c.pair_score += 1.0; }
    }

    // Beef / red meat
    if m_cat == Cat::RedMeat {
        if s.contains("garlic")   { c.pair_score += 2.0; }
        if s.contains("rosemary") { c.pair_score += 1.5; }
        if s.contains("mushroom") { c.pair_score += 1.5; } // umami synergy
        if s.contains("potato")   { c.pair_score += 1.0; }
        if s.contains("red-wine") { c.pair_score += 2.0; }
    }

    // Grain / pasta
    if m_cat == Cat::Grain {
        if s.contains("tomato")    { c.pair_score += 1.5; }
        if s.contains("basil")     { c.pair_score += 1.5; }
        if s.contains("olive-oil") { c.pair_score += 1.0; }
        if s.contains("parmesan")  { c.pair_score += 1.0; }
    }

    // Vegetable
    if m_cat == Cat::Vegetable {
        if s.contains("lemon")     { c.pair_score += 1.5; }
        if s.contains("olive-oil") { c.pair_score += 1.0; }
        if s.contains("hummus")    { c.pair_score += 1.0; }
    }

    // Legume
    if m_cat == Cat::Legume {
        if s.contains("garlic")    { c.pair_score += 2.0; }
        if s.contains("cumin")     { c.pair_score += 1.5; }
        if s.contains("tomato")    { c.pair_score += 1.0; }
        if s.contains("olive-oil") { c.pair_score += 1.0; }
    }

    // Cross-ingredient synergies from extras already in recipe
    let has_extra = |keyword: &str| extras.iter().any(|e| e.to_lowercase().contains(keyword));

    if has_extra("garlic") && is_herb_slug(&s)  { c.pair_score += 0.5; } // garlic+herbs = ✅
    if has_extra("lemon")  && is_acid_slug(&s)  { c.pair_score -= 1.0; } // avoid double acid
    if has_extra("olive-oil") && is_fat_slug(&s) { c.pair_score -= 1.5; } // avoid double fat
}

// ── Internal category enum ───────────────────────────────────────────────────
//
// Uses `product_type` (stable DB string) first, slug keywords as fallback.
// This is the fix for the brittle s.contains("milk") pattern.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cat {
    Fish,
    RawFish,
    Dairy,
    RedMeat,
    Grain,
    Vegetable,
    Legume,
    SweetFruit,
    HighAcidFruit,
    Allium,
    Other,
}

/// Resolve category from product_type (DB field) first, slug keywords second.
fn category_of_slug(slug: &str, product_type: Option<&str>) -> Cat {
    // ── product_type wins (stable, i18n-safe) ────────────────────────────────
    if let Some(pt) = product_type {
        let pt = pt.to_lowercase();
        match pt.as_str() {
            "fish" | "seafood"           => return Cat::Fish,
            "dairy" | "milk" | "cheese"  => return Cat::Dairy,
            "meat" | "poultry"           => return Cat::RedMeat,
            "grain" | "cereal" | "pasta" | "bread" => return Cat::Grain,
            "vegetable" | "greens"       => return Cat::Vegetable,
            "legume" | "beans"           => return Cat::Legume,
            "fruit" | "berry"            => return Cat::SweetFruit,
            _ => {}
        }
    }

    // ── Slug keyword fallback ────────────────────────────────────────────────
    let s = slug;

    // Raw fish (subset of fish — typically served uncooked)
    if ["salmon", "tuna", "mackerel", "sardine", "anchovy", "herring", "trout"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::RawFish;
    }

    // Cooked/generic fish
    if ["cod", "shrimp", "prawn", "crab", "lobster", "sea-bass", "tilapia",
        "halibut", "swordfish", "catfish", "squid", "octopus", "mussel",
        "clam", "oyster", "scallop", "fish", "seafood"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::Fish;
    }

    // Dairy
    if ["milk", "cheese", "yogurt", "kefir", "cream", "curd", "ricotta",
        "mozzarella", "parmesan", "feta", "brie", "cheddar", "cottage",
        "butter", "ghee", "smetana", "sour-cream"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::Dairy;
    }

    // Red meat
    if ["beef", "pork", "lamb", "veal", "steak", "venison", "bison"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::RedMeat;
    }

    // High-acid fruits (curdlers)
    if ["lemon", "lime", "orange", "grapefruit", "pineapple", "kiwi",
        "strawberr", "raspberry", "tamarind"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::HighAcidFruit;
    }

    // Sweet fruits (allium conflict)
    if ["apple", "mango", "banana", "blueberr", "peach", "pear", "plum",
        "grape", "melon", "watermelon", "cherry", "apricot", "fig", "date"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::SweetFruit;
    }

    // Alliums
    if ["garlic", "onion", "shallot", "leek", "chive", "scallion"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::Allium;
    }

    // Grain
    if ["rice", "quinoa", "pasta", "spaghetti", "penne", "fettuccine", "noodle",
        "buckwheat", "bulgur", "couscous", "farro", "barley", "oat", "wheat",
        "bread", "flour", "semolina"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::Grain;
    }

    // Vegetable
    if ["carrot", "broccoli", "zucchini", "eggplant", "pepper", "tomato",
        "cucumber", "celery", "cauliflower", "asparagus", "beetroot", "pumpkin",
        "squash", "cabbage", "lettuce", "spinach", "kale", "arugula", "corn",
        "artichoke", "fennel"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::Vegetable;
    }

    // Legume
    if ["lentil", "chickpea", "bean", "pea", "soy", "tofu", "edamame",
        "hummus", "falafel", "dal"]
        .iter().any(|k| s.contains(k))
    {
        return Cat::Legume;
    }

    Cat::Other
}

// ── Slug-only helpers (used only for boosts, not for category logic) ─────────

/// True only for liquid milk variants — not cheese, yogurt, cream, butter.
fn is_liquid_milk(s: &str) -> bool {
    ["whole-milk", "skim-milk", "oat-milk", "almond-milk", "soy-milk",
     "coconut-milk", "buttermilk", "milk"]
    .iter().any(|k| s == *k || s.ends_with(k))
}

/// True for liquids that have calories/protein but cannot be dish components.
/// cream/sour-cream/kefir are excluded — they are valid sauce/fat ingredients.
fn is_cooking_liquid(s: &str) -> bool {
    // Exact or suffix match for milk variants
    if is_liquid_milk(s) {
        return true;
    }
    // Other cooking liquids — broth, stock, juice, plain water
    ["broth", "stock", "chicken-stock", "beef-stock", "vegetable-stock",
     "chicken-broth", "beef-broth", "vegetable-broth",
     "orange-juice", "apple-juice", "grape-juice", "juice",
     "water"]
    .iter().any(|k| s == *k || s.contains(k))
}

fn is_acid_slug(s: &str) -> bool {
    ["lemon", "lime", "vinegar", "balsamic", "tamarind", "sumac"]
    .iter().any(|k| s.contains(k))
}

fn is_herb_slug(s: &str) -> bool {
    ["dill", "parsley", "cilantro", "basil", "rosemary", "thyme",
     "oregano", "mint", "chive", "tarragon", "sage", "bay-leaf"]
    .iter().any(|k| s.contains(k))
}

fn is_fat_slug(s: &str) -> bool {
    ["olive-oil", "sunflower-oil", "coconut-oil", "sesame-oil", "avocado-oil",
     "butter", "ghee", "lard", "oil"]
    .iter().any(|k| s.contains(k))
}

// ── Diet filter (hard constraint) ────────────────────────────────────────────
//
// Unlike penalties, diet is a HARD FILTER — incompatible candidates are removed.
// This runs BEFORE scoring so diet-incompatible items never appear.

fn passes_diet_filter(c: &Candidate, diet: Diet) -> bool {
    if diet == Diet::None {
        return true;
    }

    let s = c.slug.to_lowercase();
    let cat = category_of_slug(&s, c.product_type.as_deref());

    match diet {
        Diet::None => true,

        Diet::Vegan => {
            // No animal products at all
            !matches!(cat, Cat::Fish | Cat::RawFish | Cat::Dairy | Cat::RedMeat)
                && !is_animal_product(&s)
        }

        Diet::Vegetarian => {
            // No meat/fish, dairy is OK
            !matches!(cat, Cat::Fish | Cat::RawFish | Cat::RedMeat)
                && !is_meat_slug(&s)
        }

        Diet::Pescatarian => {
            // No red meat / poultry, fish + dairy OK
            cat != Cat::RedMeat && !is_poultry_slug(&s)
        }

        Diet::GlutenFree => {
            // No wheat, barley, rye, regular pasta, bread
            !is_gluten_slug(&s)
        }

        Diet::DairyFree => {
            cat != Cat::Dairy && !is_dairy_slug(&s)
        }

        Diet::Paleo => {
            // No grains, no legumes, no dairy, no processed
            !matches!(cat, Cat::Grain | Cat::Legume | Cat::Dairy)
                && !is_processed_slug(&s)
        }

        Diet::Mediterranean => {
            // Everything OK — Mediterranean is not restrictive, just preferential
            // (boosts handled in apply_meal_type_boost)
            true
        }
    }
}

fn is_animal_product(s: &str) -> bool {
    ["egg", "honey", "chicken", "turkey", "duck", "goose"]
    .iter().any(|k| s.contains(k))
}

fn is_meat_slug(s: &str) -> bool {
    ["beef", "pork", "lamb", "veal", "steak", "chicken", "turkey",
     "duck", "goose", "venison", "bison", "bacon", "ham", "sausage"]
    .iter().any(|k| s.contains(k))
}

fn is_poultry_slug(s: &str) -> bool {
    ["chicken", "turkey", "duck", "goose"]
    .iter().any(|k| s.contains(k))
}

fn is_gluten_slug(s: &str) -> bool {
    ["wheat", "barley", "rye", "spelt", "bread", "pasta", "spaghetti",
     "penne", "fettuccine", "noodle", "flour", "couscous", "bulgur",
     "semolina", "cracker", "tortilla"]
    .iter().any(|k| s.contains(k))
}

fn is_dairy_slug(s: &str) -> bool {
    ["milk", "cheese", "yogurt", "cream", "butter", "ghee", "curd",
     "ricotta", "mozzarella", "parmesan", "feta", "brie", "cheddar",
     "cottage", "smetana", "sour-cream", "kefir"]
    .iter().any(|k| s.contains(k))
}

fn is_processed_slug(s: &str) -> bool {
    ["sugar", "candy", "soda", "syrup", "corn-syrup", "margarine",
     "sausage", "hot-dog", "chips", "cracker"]
    .iter().any(|k| s.contains(k))
}

// ── Meal-type contextual boosts ──────────────────────────────────────────────
//
// These shift scores based on what meal the user is cooking.
// Breakfast → eggs, oats, yogurt, toast, fruit boost.
// Dinner → protein, heavy sides boost.
// Snack → nuts, fruit, hummus boost.
// Dessert → sweet, chocolate, cream boost.

fn apply_meal_type_boost(c: &mut Candidate, meal_type: Option<MealType>) {
    let meal = match meal_type {
        Some(m) => m,
        None => return, // no meal context — skip
    };

    let s = c.slug.to_lowercase();
    let cat = category_of_slug(&s, c.product_type.as_deref());

    match meal {
        MealType::Breakfast => {
            // Breakfast staples get a boost
            if s.contains("egg")     { c.pair_score += 2.0; }
            if s.contains("oat")     { c.pair_score += 2.0; }
            if s.contains("yogurt")  { c.pair_score += 1.5; }
            if s.contains("honey")   { c.pair_score += 1.0; }
            if s.contains("banana")  { c.pair_score += 1.5; }
            if s.contains("berr")    { c.pair_score += 1.0; } // strawberry, blueberry
            if s.contains("toast") || s.contains("bread") { c.pair_score += 1.5; }
            if s.contains("avocado") { c.pair_score += 1.5; }
            // Penalize heavy dinner-style items at breakfast
            if cat == Cat::RedMeat   { c.pair_score -= 2.0; }
        }

        MealType::Lunch => {
            // Lunch: balanced, salad-friendly, grain sides
            if cat == Cat::Vegetable { c.pair_score += 1.0; }
            if cat == Cat::Grain     { c.pair_score += 1.0; }
            if cat == Cat::Legume    { c.pair_score += 1.0; }
            if s.contains("hummus")  { c.pair_score += 1.0; }
        }

        MealType::Dinner => {
            // Dinner: protein-forward, hearty
            if cat == Cat::RedMeat   { c.pair_score += 1.5; }
            if cat == Cat::Fish      { c.pair_score += 1.0; }
            if s.contains("potato")  { c.pair_score += 1.0; }
            if s.contains("mushroom"){ c.pair_score += 1.0; }
            if s.contains("garlic")  { c.pair_score += 0.5; }
            if s.contains("wine")    { c.pair_score += 1.0; }
        }

        MealType::Snack => {
            // Snack: light, portable, no heavy cooking
            if s.contains("nut") || s.contains("almond") || s.contains("walnut") { c.pair_score += 2.0; }
            if s.contains("hummus")  { c.pair_score += 1.5; }
            if cat == Cat::SweetFruit || cat == Cat::HighAcidFruit { c.pair_score += 1.5; }
            if s.contains("yogurt")  { c.pair_score += 1.0; }
            if s.contains("cracker") || s.contains("bread") { c.pair_score += 1.0; }
            // Penalize heavy items for snacks
            if cat == Cat::RedMeat   { c.pair_score -= 2.0; }
            if cat == Cat::Grain     { c.pair_score -= 1.0; }
        }

        MealType::Dessert => {
            // Dessert: sweet, creamy, chocolate, berries
            if s.contains("chocolate") { c.pair_score += 2.5; }
            if s.contains("cream")   { c.pair_score += 2.0; }
            if s.contains("sugar")   { c.pair_score += 1.0; }
            if s.contains("vanilla") { c.pair_score += 1.5; }
            if s.contains("honey")   { c.pair_score += 1.5; }
            if s.contains("berr")    { c.pair_score += 1.5; } // strawberry, etc.
            if s.contains("banana")  { c.pair_score += 1.0; }
            if cat == Cat::SweetFruit { c.pair_score += 1.0; }
            // Penalize savory items in dessert
            if cat == Cat::Fish || cat == Cat::RawFish { c.pair_score -= 3.0; }
            if cat == Cat::RedMeat   { c.pair_score -= 3.0; }
            if cat == Cat::Allium    { c.pair_score -= 2.0; }
        }
    }
}
