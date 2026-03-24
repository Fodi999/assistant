//! Recipe Builder v2 — deterministic dish-level variant generator.
//!
//! Given a main ingredient + pool of candidates (from pairings / feedback loop),
//! produces 3 dish variants with **culinary structure**:
//!
//! 1. **Healthy** — low-cal, high-protein, high-fiber picks (salad / bowl feel)
//! 2. **Balanced** — best flavor balance, fills weak dimensions (main-course feel)
//! 3. **Heavy / Tasty** — high-fat, umami boosters, sauces (sauce-based / rich)
//!
//! v2 improvements over v1:
//! - `IngredientRole` inference (base/side/sauce/aromatic/fat)
//! - Role-constrained assembly (max 1 fat, 1 sauce, 2 sides, 1 aromatic)
//! - Real flavor-balance recomputation via `analyze_balance`
//! - Conflict detection (no duplicate fats, no clashing flavors)
//! - `DishType` detection (salad / bowl / main-course / sauce-based)
//! - Appetizing variant-aware titles
//!
//! No AI, no DB, no HTTP — pure deterministic functions.
//! Target: <5ms for 3 variants.

use crate::domain::tools::flavor_graph::{
    self, FlavorBalance, FlavorIngredient, FlavorVector,
};
use crate::domain::tools::suggestion_engine::Candidate;
use crate::shared::Language;

use super::response::{
    DishType, IngredientRole, RecipeVariant, VariantIngredient,
};

// ── Constants ────────────────────────────────────────────────────────────────

/// Role slot limits per variant
const MAX_SIDES: usize = 2;
const MAX_SAUCES: usize = 1;
const MAX_AROMATICS: usize = 1;
const MAX_FATS: usize = 1;

// ── Public API ───────────────────────────────────────────────────────────────

/// Build 3 recipe variants from the main ingredient + candidate pool.
pub fn build_variants(
    main_slug: &str,
    main_name: &str,
    main_image: Option<&str>,
    main_cal_per_100g: f64,
    main_prot_per_100g: f64,
    main_fat_per_100g: f64,
    main_fiber_per_100g: f64,
    main_typical_g: f64,
    main_product_type: Option<&str>,
    candidates: &[Candidate],
    balance: &FlavorBalance,
    lang: Language,
) -> Vec<RecipeVariant> {
    let main_category = product_type_category(main_product_type);

    // Filter out candidates without nutrition data AND bad combos with main
    let usable: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.nutrition.calories > 0.0 || c.nutrition.protein_g > 0.0)
        .filter(|c| !is_bad_combo(main_category, &c.slug, c.product_type.as_deref()))
        .filter(|c| !is_liquid_ingredient(&c.slug, c.product_type.as_deref()))
        // Extra cross-check: reject candidates that conflict with each other
        // (e.g. fish+fruit, raw meat+raw fish)
        .filter(|c| !is_cross_conflict(main_slug, &c.slug))
        .collect();

    // If no usable candidates after filtering, inject hardcoded defaults
    // so every variant always has a plausible dish.
    let default_pool;
    let classified: Vec<(&Candidate, IngredientRole)> = if usable.is_empty() {
        default_pool = default_sides_for(main_slug, main_product_type);
        default_pool
            .iter()
            .map(|c| (c, infer_role(c)))
            .collect()
    } else {
        usable.iter().map(|c| (*c, infer_role(c))).collect()
    };

    // Main ingredient info bundle
    let main_info = MainInfo {
        slug: main_slug,
        name: main_name,
        image: main_image,
        cal: main_cal_per_100g,
        _prot: main_prot_per_100g,
        _fat: main_fat_per_100g,
        _fiber: main_fiber_per_100g,
        typical_g: main_typical_g,
    };

    vec![
        build_healthy(&main_info, &classified, balance, lang),
        build_balanced(&main_info, &classified, balance, lang),
        build_heavy(&main_info, &classified, balance, lang),
    ]
}

// ── Internal types ───────────────────────────────────────────────────────────

struct MainInfo<'a> {
    slug: &'a str,
    name: &'a str,
    image: Option<&'a str>,
    cal: f64,
    _prot: f64,
    _fat: f64,
    _fiber: f64,
    typical_g: f64,
}

impl<'a> MainInfo<'a> {
    fn to_variant_ingredient(&self) -> VariantIngredient {
        VariantIngredient {
            slug: self.slug.to_string(),
            name: self.name.to_string(),
            image_url: self.image.map(|s| s.to_string()),
            role: IngredientRole::Base,
            grams: self.typical_g,
            calories: round1(self.cal * self.typical_g / 100.0),
        }
    }

    fn flavor_ingredient(&self) -> FlavorIngredient {
        // Main flavor vector not available here — use zero, rely on candidates.
        FlavorIngredient {
            slug: self.slug.to_string(),
            grams: self.typical_g,
            flavor: FlavorVector::zero(),
        }
    }
}

// ── Role inference ───────────────────────────────────────────────────────────

/// Classify a candidate into a culinary role using nutrition + flavor + product_type heuristics.
fn infer_role(c: &Candidate) -> IngredientRole {
    let n = &c.nutrition;
    let f = &c.flavor;
    let slug = c.slug.to_lowercase();
    let pt = c.product_type.as_deref().unwrap_or("").to_lowercase();

    // 1. Fat detection
    if is_fat_slug(&slug) {
        return IngredientRole::Fat;
    }
    if n.fat_g > 60.0 && n.protein_g < 10.0 && n.carbs_g < 10.0 {
        return IngredientRole::Fat;
    }

    // 2. Sauce detection
    if is_sauce_slug(&slug) {
        return IngredientRole::Sauce;
    }
    if f.acidity > 6.0 && f.umami > 3.0 && n.calories < 150.0 {
        return IngredientRole::Sauce;
    }

    // 3. Aromatic detection
    if is_aromatic_slug(&slug) {
        return IngredientRole::Aromatic;
    }
    if n.calories < 40.0 && f.aroma > 5.0 {
        return IngredientRole::Aromatic;
    }

    // 4. Product-type-aware side detection
    if pt == "vegetable" || pt == "greens" || pt == "grain"
        || pt == "cereal" || pt == "pasta" || pt == "legume"
        || pt == "beans" || pt == "bread"
    {
        return IngredientRole::Side;
    }

    // 5. Nutrition-based side detection
    if n.carbs_g > 15.0 && n.protein_g < 15.0 {
        return IngredientRole::Side;
    }
    if n.fiber_g > 3.0 && n.calories < 100.0 {
        return IngredientRole::Side;
    }
    if n.calories < 80.0 && n.fat_g < 5.0 {
        return IngredientRole::Side;
    }

    // 6. Default: Side
    IngredientRole::Side
}

fn is_fat_slug(slug: &str) -> bool {
    const FATS: &[&str] = &[
        "olive-oil", "sunflower-oil", "coconut-oil", "sesame-oil",
        "avocado-oil", "butter", "ghee", "lard", "oil",
        "margarine", "cream-cheese", "sour-cream", "smetana",
    ];
    FATS.iter().any(|k| slug.contains(k))
}

fn is_sauce_slug(slug: &str) -> bool {
    const SAUCES: &[&str] = &[
        "soy-sauce", "teriyaki", "ponzu", "sriracha", "mayo",
        "mayonnaise", "ketchup", "mustard", "pesto", "hummus",
        "tahini", "balsamic", "vinegar", "dressing", "sauce",
        "wasabi", "gochujang", "chimichurri", "salsa",
    ];
    SAUCES.iter().any(|k| slug.contains(k))
}

fn is_aromatic_slug(slug: &str) -> bool {
    const AROMATICS: &[&str] = &[
        "garlic", "ginger", "lemon", "lime", "dill", "parsley",
        "cilantro", "basil", "rosemary", "thyme", "oregano",
        "cumin", "paprika", "chili", "pepper", "turmeric",
        "cinnamon", "mint", "scallion", "green-onion", "shallot",
        "lemongrass", "star-anise", "bay-leaf", "coriander",
    ];
    AROMATICS.iter().any(|k| slug.contains(k))
}

// ── Culinary rules — bad combo & liquid detection ────────────────────────────

/// Broad food category derived from `product_type` or slug heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FoodCategory {
    Fish,
    Dairy,
    Meat,
    Grain,
    Vegetable,
    Fruit,
    Legume,
    Other,
}

fn product_type_category(pt: Option<&str>) -> FoodCategory {
    match pt.unwrap_or("").to_lowercase().as_str() {
        "fish" | "seafood" => FoodCategory::Fish,
        "dairy" | "milk" | "cheese" => FoodCategory::Dairy,
        "meat" | "poultry" => FoodCategory::Meat,
        "grain" | "cereal" | "pasta" | "bread" => FoodCategory::Grain,
        "vegetable" | "greens" => FoodCategory::Vegetable,
        "fruit" | "berry" => FoodCategory::Fruit,
        "legume" | "beans" => FoodCategory::Legume,
        _ => FoodCategory::Other,
    }
}

fn slug_category(slug: &str) -> FoodCategory {
    let s = slug.to_lowercase();
    // Fish / seafood
    if ["salmon", "tuna", "cod", "trout", "shrimp", "prawn", "crab",
        "lobster", "mackerel", "sardine", "anchovy", "sea-bass",
        "tilapia", "halibut", "swordfish", "catfish", "squid",
        "octopus", "mussel", "clam", "oyster", "scallop", "fish"]
        .iter().any(|k| s.contains(k)) {
        return FoodCategory::Fish;
    }
    // Dairy
    if ["milk", "cheese", "yogurt", "kefir", "cream", "curd", "ricotta",
        "mozzarella", "parmesan", "feta", "brie", "cheddar", "cottage"]
        .iter().any(|k| s.contains(k)) {
        return FoodCategory::Dairy;
    }
    // Meat
    if ["chicken", "beef", "pork", "lamb", "turkey", "duck", "veal",
        "ham", "bacon", "sausage", "steak"]
        .iter().any(|k| s.contains(k)) {
        return FoodCategory::Meat;
    }
    FoodCategory::Other
}

/// Returns true if candidate is a bad culinary match for the main ingredient.
fn is_bad_combo(main_cat: FoodCategory, cand_slug: &str, cand_pt: Option<&str>) -> bool {
    let cand_cat = if cand_pt.is_some() {
        product_type_category(cand_pt)
    } else {
        slug_category(cand_slug)
    };

    // Rule 1: Fish + Dairy = ❌ (both directions)
    if main_cat == FoodCategory::Fish && cand_cat == FoodCategory::Dairy {
        // Exception: parmesan & cream are sometimes OK, but safer to ban
        return true;
    }
    if main_cat == FoodCategory::Dairy && cand_cat == FoodCategory::Fish {
        return true;
    }

    // Rule 2: Fish + Meat = ❌ (surf-n-turf is very niche)
    if main_cat == FoodCategory::Fish && cand_cat == FoodCategory::Meat {
        return true;
    }
    if main_cat == FoodCategory::Meat && cand_cat == FoodCategory::Fish {
        return true;
    }

    false
}

/// Returns true if the candidate is a liquid that shouldn't be a dish component.
/// Milk, cream, juice, broth etc. can be used in cooking but not as a "side".
fn is_liquid_ingredient(slug: &str, product_type: Option<&str>) -> bool {
    let s = slug.to_lowercase();
    // Explicit liquid slugs
    let liquid_slugs = [
        "milk", "whole-milk", "skim-milk", "oat-milk", "almond-milk",
        "soy-milk", "coconut-milk", "cream", "heavy-cream",
        "whipping-cream", "half-and-half", "buttermilk",
        "orange-juice", "apple-juice", "grape-juice", "juice",
        "broth", "stock", "chicken-broth", "beef-broth",
        "vegetable-broth", "water",
    ];
    if liquid_slugs.iter().any(|k| s == *k || s.contains(k)) {
        return true;
    }
    // product_type based
    if let Some(pt) = product_type {
        let pt_lower = pt.to_lowercase();
        if pt_lower == "milk" || pt_lower == "liquid" || pt_lower == "beverage" || pt_lower == "juice" {
            return true;
        }
    }
    false
}

/// Additional slug-pair cross-conflict rules not covered by category logic.
/// Used to block niche but clearly wrong pairs (fruit + alliums, etc.)
fn is_cross_conflict(main_slug: &str, cand_slug: &str) -> bool {
    let a = main_slug.to_lowercase();
    let b = cand_slug.to_lowercase();

    // Fruit + strong alliums/fish = ❌
    let is_fruit = |s: &str| ["apple", "mango", "banana", "strawberr", "blueberr",
        "raspberry", "peach", "pear", "plum", "grape", "melon", "watermelon",
        "pineapple", "kiwi"].iter().any(|k| s.contains(k));
    let is_allium = |s: &str| ["garlic", "onion", "shallot", "leek"].iter().any(|k| s.contains(k));
    let is_raw_fish_slug = |s: &str| ["salmon", "tuna", "mackerel", "sardine",
        "anchovy", "herring"].iter().any(|k| s.contains(k));

    if is_fruit(&a) && is_allium(&b) { return true; }
    if is_fruit(&b) && is_allium(&a) { return true; }

    // Sweet fruit + raw fish (non-sushi context)
    if is_fruit(&a) && is_raw_fish_slug(&b) { return true; }
    if is_fruit(&b) && is_raw_fish_slug(&a) { return true; }

    false
}

/// Hardcoded fallback sides for when all candidates get filtered out.
/// Ensures every variant always has a minimal plausible dish structure.
fn default_sides_for(main_slug: &str, main_product_type: Option<&str>) -> Vec<Candidate> {
    let cat = product_type_category(main_product_type);
    let slug_cat = slug_category(main_slug);
    let effective_cat = if cat != FoodCategory::Other { cat } else { slug_cat };

    let mut defaults: Vec<(&str, &str, IngredientRole, f64, f64, f64, f64, f64)> = match effective_cat {
        // fish → lemon (aromatic) + rice (side) + olive-oil (fat)
        FoodCategory::Fish => vec![
            ("lemon",      "Lemon",      IngredientRole::Aromatic, 29.0,  1.1, 0.3, 2.8,  30.0),
            ("rice",       "Rice",       IngredientRole::Side,     130.0, 2.7, 0.3, 0.4, 150.0),
            ("olive-oil",  "Olive oil",  IngredientRole::Fat,      884.0, 0.0,100.0,0.0,  10.0),
        ],
        // meat/poultry → garlic (aromatic) + potato (side) + olive-oil (fat)
        FoodCategory::Meat => vec![
            ("garlic",     "Garlic",     IngredientRole::Aromatic,  149.0, 6.4, 0.5, 2.1,   5.0),
            ("potato",     "Potato",     IngredientRole::Side,       77.0, 2.0, 0.1, 2.2, 150.0),
            ("olive-oil",  "Olive oil",  IngredientRole::Fat,       884.0, 0.0,100.0,0.0,  10.0),
        ],
        // grain/pasta → tomato (side) + basil (aromatic) + olive-oil (fat)
        FoodCategory::Grain => vec![
            ("tomato",     "Tomato",     IngredientRole::Side,       18.0, 0.9, 0.2, 1.2, 120.0),
            ("basil",      "Basil",      IngredientRole::Aromatic,   23.0, 3.2, 0.6, 1.6,   5.0),
            ("olive-oil",  "Olive oil",  IngredientRole::Fat,       884.0, 0.0,100.0,0.0,  10.0),
        ],
        // vegetable → lemon (aromatic) + olive-oil (fat) + hummus (sauce)
        FoodCategory::Vegetable => vec![
            ("lemon",      "Lemon",      IngredientRole::Aromatic,   29.0, 1.1, 0.3, 2.8,  30.0),
            ("olive-oil",  "Olive oil",  IngredientRole::Fat,       884.0, 0.0,100.0,0.0,  10.0),
            ("hummus",     "Hummus",     IngredientRole::Sauce,      166.0, 7.9, 9.6, 6.0,  30.0),
        ],
        // legume → garlic (aromatic) + olive-oil (fat) + tomato (side)
        FoodCategory::Legume => vec![
            ("garlic",     "Garlic",     IngredientRole::Aromatic,  149.0, 6.4, 0.5, 2.1,   5.0),
            ("olive-oil",  "Olive oil",  IngredientRole::Fat,       884.0, 0.0,100.0,0.0,  10.0),
            ("tomato",     "Tomato",     IngredientRole::Side,       18.0, 0.9, 0.2, 1.2, 120.0),
        ],
        // default → lemon + olive-oil + garlic
        _ => vec![
            ("lemon",      "Lemon",      IngredientRole::Aromatic,   29.0, 1.1, 0.3, 2.8,  30.0),
            ("olive-oil",  "Olive oil",  IngredientRole::Fat,       884.0, 0.0,100.0,0.0,  10.0),
            ("garlic",     "Garlic",     IngredientRole::Aromatic,  149.0, 6.4, 0.5, 2.1,   5.0),
        ],
    };

    // Remove any default that clashes with the main ingredient itself
    defaults.retain(|(slug, _, _, _, _, _, _, _)| {
        !is_cross_conflict(main_slug, slug)
    });

    defaults
        .into_iter()
        .map(|(slug, name, _role, cal, prot, fat, fiber, typical_g)| {
            use crate::domain::tools::nutrition::NutritionBreakdown;
            Candidate {
                slug: slug.to_string(),
                name: name.to_string(),
                image_url: None,
                pair_score: 5.0,
                typical_g,
                nutrition: NutritionBreakdown {
                    calories: cal,
                    protein_g: prot,
                    fat_g: fat,
                    carbs_g: 0.0,
                    fiber_g: fiber,
                    sugar_g: 0.0,
                    salt_g: 0.0,
                    sodium_mg: 0.0,
                },
                flavor: FlavorVector::zero(),
                product_type: None,
            }
        })
        .collect()
}

// ── Role-constrained assembly ────────────────────────────────────────────────

/// Pick candidates respecting role slot limits.
fn assemble_by_roles<'a>(
    scored: &[(&'a Candidate, IngredientRole, f64)],
    portion_strategy: PortionStrategy,
) -> Vec<(&'a Candidate, IngredientRole, f64)> {
    let mut sides = 0usize;
    let mut sauces = 0usize;
    let mut aromatics = 0usize;
    let mut fats = 0usize;
    let mut result: Vec<(&'a Candidate, IngredientRole, f64)> = Vec::new();
    let mut used_slugs: Vec<String> = Vec::new();

    for &(c, role, _score) in scored {
        // Skip duplicates
        if used_slugs.iter().any(|s| *s == c.slug) {
            continue;
        }

        // Check slot availability
        let fits = match role {
            IngredientRole::Base => false, // main is added externally
            IngredientRole::Side => sides < MAX_SIDES,
            IngredientRole::Sauce => sauces < MAX_SAUCES,
            IngredientRole::Aromatic => aromatics < MAX_AROMATICS,
            IngredientRole::Fat => fats < MAX_FATS,
        };
        if !fits {
            continue;
        }

        // Conflict check
        if has_conflict(&result, c, role) {
            continue;
        }

        let grams = portion_for(c.typical_g, role, portion_strategy);

        match role {
            IngredientRole::Side => sides += 1,
            IngredientRole::Sauce => sauces += 1,
            IngredientRole::Aromatic => aromatics += 1,
            IngredientRole::Fat => fats += 1,
            IngredientRole::Base => {}
        }

        used_slugs.push(c.slug.clone());
        result.push((c, role, grams));

        let total = sides + sauces + aromatics + fats;
        if total >= MAX_SIDES + MAX_SAUCES + MAX_AROMATICS + MAX_FATS {
            break;
        }
    }

    result
}

/// Detect conflicts between a new candidate and existing picks.
fn has_conflict(
    existing: &[(&Candidate, IngredientRole, f64)],
    new: &Candidate,
    new_role: IngredientRole,
) -> bool {
    for &(ex, ex_role, _) in existing {
        // Two fats = conflict
        if new_role == IngredientRole::Fat && ex_role == IngredientRole::Fat {
            return true;
        }
        // Two sauces = conflict
        if new_role == IngredientRole::Sauce && ex_role == IngredientRole::Sauce {
            return true;
        }
        // Same-suffix slugs in same role (e.g. olive-oil + sunflower-oil)
        if new_role == ex_role && slugs_similar(&ex.slug, &new.slug) {
            return true;
        }
    }
    false
}

/// Rough slug similarity: share a common suffix after the last hyphen.
fn slugs_similar(a: &str, b: &str) -> bool {
    let suffix_a = a.rsplit('-').next().unwrap_or("");
    let suffix_b = b.rsplit('-').next().unwrap_or("");
    !suffix_a.is_empty() && suffix_a == suffix_b && a != b
}

// ── Portion strategies ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum PortionStrategy {
    Light,    // healthy: 70% typical, sauces minimal
    Standard, // balanced: 100% typical
    Generous, // heavy: 130% typical, sauces generous
}

fn portion_for(typical_g: f64, role: IngredientRole, strategy: PortionStrategy) -> f64 {
    let base = match strategy {
        PortionStrategy::Light => match role {
            IngredientRole::Side => (typical_g * 0.7).max(20.0),
            IngredientRole::Sauce => 15.0,
            IngredientRole::Aromatic => 5.0,
            IngredientRole::Fat => 5.0,
            IngredientRole::Base => typical_g,
        },
        PortionStrategy::Standard => match role {
            IngredientRole::Side => typical_g,
            IngredientRole::Sauce => 25.0,
            IngredientRole::Aromatic => 8.0,
            IngredientRole::Fat => 10.0,
            IngredientRole::Base => typical_g,
        },
        PortionStrategy::Generous => match role {
            IngredientRole::Side => (typical_g * 1.3).max(40.0),
            IngredientRole::Sauce => 40.0,
            IngredientRole::Aromatic => 10.0,
            IngredientRole::Fat => 20.0,
            IngredientRole::Base => typical_g,
        },
    };
    round1(base)
}

// ── Variant 1: Healthy ───────────────────────────────────────────────────────

fn build_healthy(
    main: &MainInfo,
    classified: &[(&Candidate, IngredientRole)],
    _balance: &FlavorBalance,
    lang: Language,
) -> RecipeVariant {
    let mut scored: Vec<(&Candidate, IngredientRole, f64)> = classified
        .iter()
        .map(|&(c, role)| {
            let n = &c.nutrition;
            let protein_ratio = if n.calories > 0.0 {
                (n.protein_g * 4.0 / n.calories).min(1.0)
            } else {
                0.0
            };
            let fiber_bonus = (n.fiber_g / 10.0).min(1.0);
            let low_cal_bonus = if n.calories < 50.0 {
                1.0
            } else if n.calories < 100.0 {
                0.7
            } else if n.calories < 200.0 {
                0.3
            } else {
                0.0
            };
            // Prefer sides and aromatics for healthy
            let role_bonus = match role {
                IngredientRole::Side => 5.0,
                IngredientRole::Aromatic => 8.0,
                IngredientRole::Fat => -10.0,
                IngredientRole::Sauce => -3.0,
                IngredientRole::Base => 0.0,
            };
            let score = protein_ratio * 35.0
                + fiber_bonus * 25.0
                + low_cal_bonus * 20.0
                + c.pair_score.min(10.0)
                + role_bonus;
            (c, role, score)
        })
        .collect();

    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let picks = assemble_by_roles(&scored, PortionStrategy::Light);
    let (ingredients, total_cal, new_balance) = compose_dish(main, &picks);
    let dish_type = detect_dish_type(&ingredients);
    let score = compute_healthy_score(&ingredients, total_cal);

    RecipeVariant {
        variant_type: "healthy".to_string(),
        dish_type,
        title: build_title(&ingredients, "healthy", dish_type, lang),
        ingredients,
        total_calories: total_cal.round() as i32,
        score,
        balance_score: new_balance.balance_score,
        explanation: healthy_explanation(lang),
    }
}

// ── Variant 2: Balanced ──────────────────────────────────────────────────────

fn build_balanced(
    main: &MainInfo,
    classified: &[(&Candidate, IngredientRole)],
    balance: &FlavorBalance,
    lang: Language,
) -> RecipeVariant {
    let weak_dims: Vec<&str> = balance
        .weak_dimensions
        .iter()
        .map(|g| g.dimension.as_str())
        .collect();

    let mut scored: Vec<(&Candidate, IngredientRole, f64)> = classified
        .iter()
        .map(|&(c, role)| {
            let fv = &c.flavor;
            let gap_fill: f64 = weak_dims
                .iter()
                .map(|dim| dimension_value(fv, dim))
                .sum::<f64>()
                / (weak_dims.len().max(1) as f64);

            let pair_bonus = (c.pair_score / 10.0).min(1.0) * 25.0;

            let n = &c.nutrition;
            let macro_balance = if n.calories > 0.0 {
                let p = n.protein_g * 4.0 / n.calories;
                let f = n.fat_g * 9.0 / n.calories;
                let c_ratio = n.carbs_g * 4.0 / n.calories;
                (1.0 - ((p - 0.3).abs() + (f - 0.3).abs() + (c_ratio - 0.4).abs()) / 3.0)
                    .max(0.0)
            } else {
                0.0
            };

            // Role diversity bonus
            let role_bonus = match role {
                IngredientRole::Side => 3.0,
                IngredientRole::Sauce => 5.0,
                IngredientRole::Aromatic => 5.0,
                IngredientRole::Fat => 3.0,
                IngredientRole::Base => 0.0,
            };

            let score = gap_fill * 35.0 + pair_bonus + macro_balance * 20.0 + role_bonus;
            (c, role, score)
        })
        .collect();

    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let picks = assemble_by_roles(&scored, PortionStrategy::Standard);
    let (ingredients, total_cal, new_balance) = compose_dish(main, &picks);
    let dish_type = detect_dish_type(&ingredients);
    // Balanced score IS the real recomputed flavor balance
    let score = new_balance.balance_score;

    RecipeVariant {
        variant_type: "balanced".to_string(),
        dish_type,
        title: build_title(&ingredients, "balanced", dish_type, lang),
        ingredients,
        total_calories: total_cal.round() as i32,
        score,
        balance_score: new_balance.balance_score,
        explanation: balanced_explanation(lang),
    }
}

// ── Variant 3: Heavy / Tasty ─────────────────────────────────────────────────

fn build_heavy(
    main: &MainInfo,
    classified: &[(&Candidate, IngredientRole)],
    _balance: &FlavorBalance,
    lang: Language,
) -> RecipeVariant {
    let mut scored: Vec<(&Candidate, IngredientRole, f64)> = classified
        .iter()
        .map(|&(c, role)| {
            let fv = &c.flavor;
            let n = &c.nutrition;

            let fat_score = (n.fat_g / 20.0).min(1.0) * 20.0 + (fv.fat / 10.0) * 10.0;
            let umami_score = (fv.umami / 10.0) * 25.0;
            let aroma_score = (fv.aroma / 10.0) * 10.0;

            let density_bonus = if n.calories > 200.0 {
                12.0
            } else if n.calories > 100.0 {
                6.0
            } else {
                0.0
            };

            // Heavy loves sauces and fats
            let role_bonus = match role {
                IngredientRole::Sauce => 10.0,
                IngredientRole::Fat => 8.0,
                IngredientRole::Side => 2.0,
                IngredientRole::Aromatic => 3.0,
                IngredientRole::Base => 0.0,
            };

            let pair_bonus = (c.pair_score / 10.0).min(1.0) * 5.0;
            let score = fat_score + umami_score + aroma_score + density_bonus + role_bonus + pair_bonus;
            (c, role, score)
        })
        .collect();

    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let picks = assemble_by_roles(&scored, PortionStrategy::Generous);
    let (ingredients, total_cal, new_balance) = compose_dish(main, &picks);
    let dish_type = detect_dish_type(&ingredients);
    let score = compute_heavy_score(&picks);

    RecipeVariant {
        variant_type: "heavy".to_string(),
        dish_type,
        title: build_title(&ingredients, "heavy", dish_type, lang),
        ingredients,
        total_calories: total_cal.round() as i32,
        score,
        balance_score: new_balance.balance_score,
        explanation: heavy_explanation(lang),
    }
}

// ── Dish composition & balance recomputation ─────────────────────────────────

/// Compose the final ingredient list + compute real flavor balance.
fn compose_dish(
    main: &MainInfo,
    picks: &[(&Candidate, IngredientRole, f64)],
) -> (Vec<VariantIngredient>, f64, FlavorBalance) {
    let main_vi = main.to_variant_ingredient();
    let mut total_cal = main_vi.calories;
    let mut ingredients = vec![main_vi];

    let mut flavor_ings = vec![main.flavor_ingredient()];

    for &(c, role, grams) in picks {
        let cal = round1(c.nutrition.calories * grams / 100.0);
        total_cal += cal;
        ingredients.push(VariantIngredient {
            slug: c.slug.clone(),
            name: c.name.clone(),
            image_url: c.image_url.clone(),
            role,
            grams,
            calories: cal,
        });
        flavor_ings.push(FlavorIngredient {
            slug: c.slug.clone(),
            grams,
            flavor: c.flavor.clone(),
        });
    }

    // Real balance recomputation on the assembled dish
    let new_balance = flavor_graph::analyze_balance(&flavor_ings);

    (ingredients, total_cal, new_balance)
}

// ── Dish type detection ──────────────────────────────────────────────────────

fn detect_dish_type(ingredients: &[VariantIngredient]) -> DishType {
    let has_sauce = ingredients.iter().any(|i| i.role == IngredientRole::Sauce);
    let side_count = ingredients.iter().filter(|i| i.role == IngredientRole::Side).count();
    let has_fat = ingredients.iter().any(|i| i.role == IngredientRole::Fat);
    let total_cal: f64 = ingredients.iter().map(|i| i.calories).sum();

    // Slug-based detection (more specific than role-based)
    let slugs_contain = |keywords: &[&str]| {
        ingredients.iter().any(|i| {
            let s = i.slug.to_lowercase();
            keywords.iter().any(|k| s.contains(k))
        })
    };

    // Bowl: grain base detected
    if slugs_contain(&["rice", "quinoa", "buckwheat", "bulgur", "couscous", "farro",
                        "barley", "millet", "noodle", "soba", "udon", "ramen"]) {
        return DishType::Bowl;
    }

    // Salad: leafy greens or raw-salad base
    if slugs_contain(&["lettuce", "spinach", "arugula", "rocket", "kale", "cabbage",
                        "watercress", "endive", "radicchio", "mixed-greens"]) {
        return DishType::Salad;
    }

    // Sauce-based: explicit sauce + fat OR pasta with sauce
    if has_sauce && has_fat {
        return DishType::SauceBased;
    }
    if has_sauce && slugs_contain(&["pasta", "spaghetti", "penne", "fettuccine",
                                    "linguine", "rigatoni", "tagliatelle"]) {
        return DishType::SauceBased;
    }

    // Fallback: calorie-light multi-side = Salad
    if total_cal < 350.0 && side_count >= 2 {
        return DishType::Salad;
    }

    // Fallback: has sides but no sauce = Bowl
    if side_count >= 1 && !has_sauce {
        return DishType::Bowl;
    }

    DishType::MainCourse
}

// ── Appetizing titles ────────────────────────────────────────────────────────

fn build_title(
    ingredients: &[VariantIngredient],
    variant: &str,
    dish_type: DishType,
    lang: Language,
) -> String {
    if ingredients.is_empty() {
        return String::new();
    }
    let main_name = &ingredients[0].name;

    let sauce_name = ingredients.iter()
        .find(|i| i.role == IngredientRole::Sauce)
        .map(|i| i.name.as_str());

    let sides: Vec<&str> = ingredients
        .iter()
        .filter(|i| i.role == IngredientRole::Side)
        .map(|i| i.name.as_str())
        .collect();

    let aromatic = ingredients.iter()
        .find(|i| i.role == IngredientRole::Aromatic)
        .map(|i| i.name.as_str());

    match lang {
        Language::Ru => build_title_ru(main_name, variant, dish_type, sauce_name, &sides, aromatic),
        Language::Uk => build_title_uk(main_name, variant, dish_type, sauce_name, &sides, aromatic),
        Language::Pl => build_title_pl(main_name, variant, dish_type, sauce_name, &sides, aromatic),
        Language::En => build_title_en(main_name, variant, dish_type, sauce_name, &sides, aromatic),
    }
}

// ── RU titles ────────────────────────────────────────────────────────────────

fn build_title_ru(
    main_name: &str, variant: &str, dish_type: DishType,
    sauce: Option<&str>, sides: &[&str], aromatic: Option<&str>,
) -> String {
    let prefix = match variant {
        "healthy" => "Лёгкий",
        "heavy" => "Сытный",
        _ => "",
    };
    match dish_type {
        DishType::SauceBased => {
            if let Some(s) = sauce {
                if prefix.is_empty() {
                    format!("{} в соусе {}", main_name, s)
                } else {
                    format!("{} {} в соусе {}", prefix, main_name.to_lowercase(), s)
                }
            } else {
                fmt_companions_ru(prefix, main_name, sides, aromatic)
            }
        }
        DishType::Salad => {
            let p = if prefix.is_empty() { "Салат:".to_string() } else { format!("{} салат:", prefix) };
            fmt_companions_ru(&p, main_name, sides, aromatic)
        }
        DishType::Bowl => {
            let p = if prefix.is_empty() { "Боул:".to_string() } else { format!("{} боул:", prefix) };
            fmt_companions_ru(&p, main_name, sides, aromatic)
        }
        DishType::MainCourse => fmt_companions_ru(prefix, main_name, sides, aromatic),
    }
}

fn fmt_companions_ru(prefix: &str, main_name: &str, sides: &[&str], aromatic: Option<&str>) -> String {
    let mut parts: Vec<&str> = sides.to_vec();
    if let Some(a) = aromatic { parts.push(a); }
    let base = if prefix.is_empty() { main_name.to_string() } else { format!("{} {}", prefix, main_name.to_lowercase()) };
    if parts.is_empty() { base }
    else if parts.len() == 1 { format!("{} с {}", base, parts[0]) }
    else { let last = parts.pop().unwrap(); format!("{} с {} и {}", base, parts.join(", "), last) }
}

// ── UK titles ────────────────────────────────────────────────────────────────

fn build_title_uk(
    main_name: &str, variant: &str, dish_type: DishType,
    sauce: Option<&str>, sides: &[&str], aromatic: Option<&str>,
) -> String {
    let prefix = match variant { "healthy" => "Легкий", "heavy" => "Ситний", _ => "" };
    match dish_type {
        DishType::SauceBased => {
            if let Some(s) = sauce {
                if prefix.is_empty() { format!("{} у соусі {}", main_name, s) }
                else { format!("{} {} у соусі {}", prefix, main_name.to_lowercase(), s) }
            } else {
                fmt_companions_uk(prefix, main_name, sides, aromatic)
            }
        }
        _ => fmt_companions_uk(prefix, main_name, sides, aromatic),
    }
}

fn fmt_companions_uk(prefix: &str, main_name: &str, sides: &[&str], aromatic: Option<&str>) -> String {
    let mut parts: Vec<&str> = sides.to_vec();
    if let Some(a) = aromatic { parts.push(a); }
    let base = if prefix.is_empty() { main_name.to_string() } else { format!("{} {}", prefix, main_name.to_lowercase()) };
    if parts.is_empty() { base }
    else if parts.len() == 1 { format!("{} з {}", base, parts[0]) }
    else { let last = parts.pop().unwrap(); format!("{} з {} та {}", base, parts.join(", "), last) }
}

// ── PL titles ────────────────────────────────────────────────────────────────

fn build_title_pl(
    main_name: &str, variant: &str, dish_type: DishType,
    sauce: Option<&str>, sides: &[&str], aromatic: Option<&str>,
) -> String {
    let prefix = match variant { "healthy" => "Lekki", "heavy" => "Syty", _ => "" };
    match dish_type {
        DishType::SauceBased => {
            if let Some(s) = sauce {
                if prefix.is_empty() { format!("{} w sosie {}", main_name, s) }
                else { format!("{} {} w sosie {}", prefix, main_name.to_lowercase(), s) }
            } else {
                fmt_companions_pl(prefix, main_name, sides, aromatic)
            }
        }
        _ => fmt_companions_pl(prefix, main_name, sides, aromatic),
    }
}

fn fmt_companions_pl(prefix: &str, main_name: &str, sides: &[&str], aromatic: Option<&str>) -> String {
    let mut parts: Vec<&str> = sides.to_vec();
    if let Some(a) = aromatic { parts.push(a); }
    let base = if prefix.is_empty() { main_name.to_string() } else { format!("{} {}", prefix, main_name.to_lowercase()) };
    if parts.is_empty() { base }
    else if parts.len() == 1 { format!("{} z {}", base, parts[0]) }
    else { let last = parts.pop().unwrap(); format!("{} z {} i {}", base, parts.join(", "), last) }
}

// ── EN titles ────────────────────────────────────────────────────────────────

fn build_title_en(
    main_name: &str, variant: &str, dish_type: DishType,
    sauce: Option<&str>, sides: &[&str], aromatic: Option<&str>,
) -> String {
    let prefix = match variant { "healthy" => "Light", "heavy" => "Rich", _ => "" };
    match dish_type {
        DishType::SauceBased => {
            if let Some(s) = sauce {
                if prefix.is_empty() { format!("{} in {} sauce", main_name, s) }
                else { format!("{} {} in {} sauce", prefix, main_name.to_lowercase(), s) }
            } else {
                fmt_companions_en(prefix, main_name, sides, aromatic)
            }
        }
        DishType::Salad => {
            let p = if prefix.is_empty() { "Salad:".to_string() } else { format!("{} salad:", prefix) };
            fmt_companions_en(&p, main_name, sides, aromatic)
        }
        DishType::Bowl => {
            let p = if prefix.is_empty() { "Bowl:".to_string() } else { format!("{} bowl:", prefix) };
            fmt_companions_en(&p, main_name, sides, aromatic)
        }
        DishType::MainCourse => fmt_companions_en(prefix, main_name, sides, aromatic),
    }
}

fn fmt_companions_en(prefix: &str, main_name: &str, sides: &[&str], aromatic: Option<&str>) -> String {
    let mut parts: Vec<&str> = sides.to_vec();
    if let Some(a) = aromatic { parts.push(a); }
    let base = if prefix.is_empty() { main_name.to_string() } else { format!("{} {}", prefix, main_name.to_lowercase()) };
    if parts.is_empty() { base }
    else if parts.len() == 1 { format!("{} with {}", base, parts[0]) }
    else { let last = parts.pop().unwrap(); format!("{} with {} and {}", base, parts.join(", "), last) }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn dimension_value(fv: &FlavorVector, dim: &str) -> f64 {
    match dim {
        "sweetness"  => fv.sweetness,
        "acidity"    => fv.acidity,
        "bitterness" => fv.bitterness,
        "umami"      => fv.umami,
        "aroma"      => fv.aroma,
        "fat"        => fv.fat,
        _ => 0.0,
    }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

// ── Scoring ──────────────────────────────────────────────────────────────────

fn compute_healthy_score(ingredients: &[VariantIngredient], total_cal: f64) -> u8 {
    let cal_score = if total_cal < 300.0 {
        40.0
    } else if total_cal < 500.0 {
        30.0
    } else if total_cal < 700.0 {
        20.0
    } else {
        10.0
    };
    let variety = (ingredients.len() as f64 / 5.0).min(1.0) * 30.0;
    let roles: std::collections::HashSet<_> = ingredients.iter()
        .map(|i| std::mem::discriminant(&i.role)).collect();
    let role_diversity = (roles.len() as f64 / 4.0).min(1.0) * 10.0;
    let base = cal_score + variety + role_diversity + 15.0;
    base.clamp(0.0, 100.0).round() as u8
}

fn compute_heavy_score(picks: &[(&Candidate, IngredientRole, f64)]) -> u8 {
    if picks.is_empty() {
        return 50;
    }
    let avg_umami: f64 = picks.iter().map(|(c, _, _)| c.flavor.umami).sum::<f64>() / picks.len() as f64;
    let avg_fat: f64 = picks.iter().map(|(c, _, _)| c.flavor.fat).sum::<f64>() / picks.len() as f64;
    let has_sauce = picks.iter().any(|(_, r, _)| *r == IngredientRole::Sauce);
    let sauce_bonus = if has_sauce { 10.0 } else { 0.0 };
    let score = (avg_umami + avg_fat) / 20.0 * 50.0 + sauce_bonus + 30.0;
    score.clamp(0.0, 100.0).round() as u8
}

// ── Localized explanations ───────────────────────────────────────────────────

fn healthy_explanation(lang: Language) -> String {
    match lang {
        Language::Ru => "Лёгкий вариант: упор на белок и клетчатку, минимум калорий. Роли: гарнир + аромат.".to_string(),
        Language::Uk => "Легкий варіант: акцент на білок та клітковину, мінімум калорій.".to_string(),
        Language::Pl => "Lekka wersja: nacisk na białko i błonnik, minimum kalorii.".to_string(),
        Language::En => "Light variant: high protein & fiber, minimal calories. Roles: side + aromatic.".to_string(),
    }
}

fn balanced_explanation(lang: Language) -> String {
    match lang {
        Language::Ru => "Сбалансированный вариант: гармония вкуса и нутриентов. Все роли представлены.".to_string(),
        Language::Uk => "Збалансований варіант: гармонія смаку та нутрієнтів.".to_string(),
        Language::Pl => "Zbalansowana wersja: harmonia smaku i składników odżywczych.".to_string(),
        Language::En => "Balanced variant: flavor harmony and nutrient balance. All roles present.".to_string(),
    }
}

fn heavy_explanation(lang: Language) -> String {
    match lang {
        Language::Ru => "Сытный вариант: максимум вкуса, жиров и умами. Соус + жир для насыщенности.".to_string(),
        Language::Uk => "Ситний варіант: максимум смаку, жирів та умамі.".to_string(),
        Language::Pl => "Syta wersja: maksimum smaku, tłuszczu i umami.".to_string(),
        Language::En => "Rich variant: maximum flavor, fats, and umami. Sauce + fat for richness.".to_string(),
    }
}