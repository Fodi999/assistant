//! Dish Context — food intelligence layer
//!
//! Classifies a recipe as Dessert / Savory / Neutral based on its ingredients,
//! and provides compatibility rules so we never suggest salmon for a dessert
//! or chocolate for a chicken dish.
//!
//! Pure functions, no DB, no HTTP.

use serde::Serialize;

// ── Dish Type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DishType {
    Dessert,
    Savory,
    Neutral,
}

/// Classify a recipe based on ingredient product_types, sugar ratio, and flavor.
///
/// Arguments:
/// - `ingredients`: slice of (slug, grams, Option<product_type>)
/// - `sugar_ratio`: sugar_calories / total_calories × 100 (0–100)
/// - `sweetness`: recipe's overall sweetness from FlavorVector (0–10)
/// - `umami`: recipe's overall umami from FlavorVector (0–10)
pub fn classify_dish(
    ingredients: &[(String, f64, Option<String>)],
    sugar_ratio: f64,
    sweetness: f64,
    umami: f64,
) -> DishType {
    let types: Vec<&str> = ingredients
        .iter()
        .filter_map(|(_, _, pt)| pt.as_deref())
        .collect();

    let has = |t: &str| types.iter().any(|&pt| pt == t);

    // ── Strong dessert signals ──
    let has_dessert_base = has("fruit") || has("berry") || has("chocolate") || has("sweetener");
    let has_meat_fish = has("meat") || has("fish") || has("seafood");
    let high_sugar = sugar_ratio > 15.0;
    let sweet_flavor = sweetness > 4.0;

    // ── Strong savory signals ──
    let has_savory_base = has("meat") || has("fish") || has("seafood") || has("vegetable");
    let high_umami = umami > 3.0;

    // Decision tree:
    // 1. If has meat/fish → always Savory (even with some sugar)
    if has_meat_fish {
        return DishType::Savory;
    }

    // 2. If high sugar + dessert ingredients + sweet flavor → Dessert
    if has_dessert_base && (high_sugar || sweet_flavor) {
        return DishType::Dessert;
    }

    // 3. If has savory base or high umami → Savory
    if has_savory_base || high_umami {
        return DishType::Savory;
    }

    // 4. If sugar-heavy with dairy (cottage cheese + honey, yogurt + fruit) → Dessert
    if high_sugar && (has("dairy") || has("egg")) {
        return DishType::Dessert;
    }

    // 5. Default → Neutral (oatmeal, rice, plain grain dishes)
    DishType::Neutral
}

// ── Compatibility ────────────────────────────────────────────────────────────

/// Product types that are COMPATIBLE with each dish type.
/// Returns a multiplier: 1.0 = fully compatible, 0.0 = hard block, 0.1 = soft penalty.
pub fn compatibility_score(dish_type: DishType, candidate_product_type: Option<&str>) -> f64 {
    let pt = match candidate_product_type {
        Some(t) => t,
        None => return 0.7, // unknown type → mild penalty
    };

    match dish_type {
        DishType::Dessert => match pt {
            // Fully compatible with dessert
            "fruit" | "berry" | "dairy" | "sweetener" | "chocolate" | "nut"
            | "seed" | "grain" | "egg" | "spice" => 1.0,
            // Oils are OK in small amounts (butter in pastry)
            "oil" | "fat" => 0.8,
            // Vegetables mostly wrong (except avocado, but that's rare)
            "vegetable" => 0.15,
            // Hard block: meat, fish, garlic-heavy things
            "meat" | "fish" | "seafood" | "legume" => 0.0,
            // Herb is borderline (mint OK, basil OK, but parsley/dill → no)
            "herb" => 0.3,
            _ => 0.5,
        },
        DishType::Savory => match pt {
            // Fully compatible with savory
            "meat" | "fish" | "seafood" | "vegetable" | "legume" | "grain"
            | "dairy" | "egg" | "oil" | "fat" | "herb" | "spice" | "nut" | "seed" => 1.0,
            // Sugar-heavy things → strong penalty
            "sweetener" | "chocolate" => 0.05,
            // Fruit is sometimes OK (lemon, tomato counted as veg, but apple → less)
            "fruit" | "berry" => 0.3,
            _ => 0.7,
        },
        DishType::Neutral => {
            // Neutral allows everything but with mild preferences
            match pt {
                "sweetener" | "chocolate" => 0.6,
                "meat" | "fish" | "seafood" => 0.8,
                _ => 1.0,
            }
        }
    }
}

// ── Category dedup limit ─────────────────────────────────────────────────────

/// Maximum suggestions per product_type category.
/// Prevents "add chicken + salmon + eggs" (3 proteins at once).
pub const MAX_PER_CATEGORY: usize = 1;

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ing(slug: &str, g: f64, pt: &str) -> (String, f64, Option<String>) {
        (slug.to_string(), g, Some(pt.to_string()))
    }

    #[test]
    fn cottage_cheese_strawberry_honey_is_dessert() {
        let ings = vec![
            ing("cottage-cheese", 100.0, "dairy"),
            ing("strawberry", 100.0, "fruit"),
            ing("honey", 30.0, "sweetener"),
        ];
        assert_eq!(classify_dish(&ings, 25.0, 5.0, 1.0), DishType::Dessert);
    }

    #[test]
    fn chicken_butter_is_savory() {
        let ings = vec![
            ing("chicken-breast", 200.0, "meat"),
            ing("butter", 20.0, "fat"),
        ];
        assert_eq!(classify_dish(&ings, 0.0, 0.5, 5.0), DishType::Savory);
    }

    #[test]
    fn oatmeal_is_neutral() {
        let ings = vec![
            ing("oatmeal", 80.0, "grain"),
        ];
        assert_eq!(classify_dish(&ings, 2.0, 1.0, 0.5), DishType::Neutral);
    }

    #[test]
    fn salmon_blocked_in_dessert() {
        assert_eq!(compatibility_score(DishType::Dessert, Some("fish")), 0.0);
        assert_eq!(compatibility_score(DishType::Dessert, Some("meat")), 0.0);
    }

    #[test]
    fn chocolate_blocked_in_savory() {
        assert!(compatibility_score(DishType::Savory, Some("chocolate")) < 0.1);
        assert!(compatibility_score(DishType::Savory, Some("sweetener")) < 0.1);
    }

    #[test]
    fn fruit_ok_in_dessert() {
        assert_eq!(compatibility_score(DishType::Dessert, Some("fruit")), 1.0);
        assert_eq!(compatibility_score(DishType::Dessert, Some("dairy")), 1.0);
    }

    #[test]
    fn meat_with_honey_still_savory() {
        // If there's meat, it's always savory — even with some honey
        let ings = vec![
            ing("chicken-breast", 200.0, "meat"),
            ing("honey", 20.0, "sweetener"),
        ];
        assert_eq!(classify_dish(&ings, 10.0, 3.0, 4.0), DishType::Savory);
    }
}
