//! Food Pairing Rules — DDD ingredient compatibility filter.
//!
//! Prevents absurd combinations that Gemini sometimes returns:
//!   ❌ cherry with fish
//!   ❌ chocolate with beef stew
//!   ❌ banana in borscht
//!
//! Two mechanisms:
//!   1. `BANNED_WITH` — hard bans: "never combine X with Y"
//!   2. `BANNED_IN_DISH` — dish-level: "never put X in soup"
//!
//! Called BEFORE ingredient resolution, filters the slug list from Gemini.

use super::recipe_engine::DishType;

// ── Banned Pairs ─────────────────────────────────────────────────────────────

/// Hard-banned ingredient pairs: (slug_contains_A, slug_contains_B).
/// If BOTH appear in the ingredient list, remove the SECOND one.
const BANNED_PAIRS: &[(&str, &str)] = &[
    // ── Ice cream / desserts with savory proteins ──
    ("ice-cream",  "chicken"),
    ("ice-cream",  "beef"),
    ("ice-cream",  "pork"),
    ("ice-cream",  "fish"),
    ("ice-cream",  "cod"),
    ("ice-cream",  "salmon"),
    ("ice_cream",  "chicken"),
    ("ice_cream",  "beef"),
    ("ice_cream",  "pork"),
    ("ice_cream",  "fish"),
    ("gelato",     "chicken"),
    ("gelato",     "beef"),
    ("gelato",     "fish"),
    // ── Fruit + fish/seafood (unless specifically paired like lemon) ──
    ("cherry",     "fish"),
    ("cherry",     "seafood"),
    ("cherry",     "cod"),
    ("cherry",     "salmon"),
    ("cherry",     "tuna"),
    ("banana",     "fish"),
    ("banana",     "seafood"),
    ("banana",     "cod"),
    ("banana",     "beef"),
    ("grape",      "fish"),
    ("grape",      "chicken"),
    ("watermelon", "fish"),
    ("watermelon", "meat"),
    ("kiwi",       "fish"),
    ("kiwi",       "beef"),
    // ── Chocolate + savory proteins ──
    ("chocolate",  "beef"),
    ("chocolate",  "chicken"),
    ("chocolate",  "fish"),
    ("chocolate",  "pork"),
    // ── Dairy + fish (except butter/cream which are okay) ──
    ("yogurt",     "fish"),
    ("kefir",      "fish"),
    ("milk",       "fish"),
];

// ── Banned in Dish Type ──────────────────────────────────────────────────────

/// Ingredients that should NEVER appear in certain dish types.
/// (slug_contains, banned_dish_type)
const BANNED_IN_DISH: &[(&str, DishType)] = &[
    // ── Desserts / sweets: NEVER in savory dishes ──
    ("ice-cream",  DishType::Soup),
    ("ice_cream",  DishType::Soup),
    ("ice-cream",  DishType::Stew),
    ("ice_cream",  DishType::Stew),
    ("ice-cream",  DishType::StirFry),
    ("ice_cream",  DishType::StirFry),
    ("ice-cream",  DishType::Grill),
    ("ice_cream",  DishType::Grill),
    ("ice-cream",  DishType::Pasta),
    ("ice_cream",  DishType::Pasta),
    ("gelato",     DishType::Soup),
    ("gelato",     DishType::Stew),
    ("sorbet",     DishType::Soup),
    ("sorbet",     DishType::Stew),
    ("candy",      DishType::Soup),
    ("candy",      DishType::Stew),
    ("candy",      DishType::StirFry),
    ("caramel",    DishType::Soup),
    ("caramel",    DishType::Stew),
    ("marshmallow", DishType::Soup),
    ("marshmallow", DishType::Stew),
    ("whipped-cream", DishType::Soup),
    ("cookie",     DishType::Soup),
    ("cookie",     DishType::Stew),
    ("wafer",      DishType::Soup),
    ("wafer",      DishType::Stew),
    ("jam",        DishType::Soup),
    ("jam",        DishType::Stew),
    ("syrup",      DishType::Soup),
    ("syrup",      DishType::Stew),
    // ── Chocolate: only in Bake (mole sauce aside, Gemini won't pick it) ──
    ("chocolate",  DishType::Soup),
    ("chocolate",  DishType::Stew),
    ("chocolate",  DishType::Salad),
    ("chocolate",  DishType::StirFry),
    ("chocolate",  DishType::Grill),
    ("chocolate",  DishType::Pasta),
    // ── Sweet fruits don't belong in soups / stews ──
    ("cherry",     DishType::Soup),
    ("banana",     DishType::Soup),
    ("grape",      DishType::Soup),
    ("watermelon", DishType::Soup),
    ("kiwi",       DishType::Soup),
    ("mango",      DishType::Soup),
    ("pineapple",  DishType::Soup),
    ("cherry",     DishType::Stew),
    ("banana",     DishType::Stew),
    ("grape",      DishType::Stew),
    ("watermelon", DishType::Stew),
    ("pineapple",  DishType::Stew),
    // ── Raw fish exclusions ──
    ("chicken",    DishType::Raw), // chicken tartare = dangerous
    ("pork",       DishType::Raw), // pork tartare = dangerous
];

// ── Public API ───────────────────────────────────────────────────────────────

/// Filter a list of ingredient slugs, removing incompatible ones.
///
/// Returns: (filtered_slugs, removed_slugs_with_reasons).
///
/// ```text
/// Input:  ["cod", "potato", "cherry", "onion"]  dish=Soup
/// Output: (["cod", "potato", "onion"], [("cherry", "banned in Soup")])
/// ```
pub fn filter_ingredients(
    slugs: &[String],
    dish_type: DishType,
) -> (Vec<String>, Vec<(String, String)>) {
    let mut kept: Vec<String> = Vec::new();
    let mut removed: Vec<(String, String)> = Vec::new();

    for slug in slugs {
        let slug_lower = slug.to_lowercase();

        // Check dish-level bans
        if is_banned_in_dish(&slug_lower, dish_type) {
            removed.push((slug.clone(), format!("banned in {:?}", dish_type)));
            continue;
        }

        // Check pair bans: is this slug incompatible with anything already kept?
        let mut pair_banned = false;
        for existing in &kept {
            let existing_lower: String = existing.to_string().to_lowercase();
            if is_banned_pair(&slug_lower, &existing_lower) {
                removed.push((slug.clone(), format!("incompatible with {}", existing)));
                pair_banned = true;
                break;
            }
        }

        if !pair_banned {
            kept.push(slug.clone());
        }
    }

    if !removed.is_empty() {
        tracing::info!(
            "🚫 food_pairing filtered: {:?}",
            removed.iter().map(|(s, r)| format!("{s} ({r})")).collect::<Vec<_>>()
        );
    }

    (kept, removed)
}

/// Check if a slug is banned from a dish type.
fn is_banned_in_dish(slug: &str, dish_type: DishType) -> bool {
    BANNED_IN_DISH.iter().any(|(banned, dt)| {
        *dt == dish_type && slug.contains(banned)
    })
}

/// Check if two slugs form a banned pair.
fn is_banned_pair(a: &str, b: &str) -> bool {
    BANNED_PAIRS.iter().any(|(x, y)| {
        (a.contains(x) && b.contains(y)) || (a.contains(y) && b.contains(x))
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cherry_removed_from_fish_soup() {
        let slugs = vec!["cod".into(), "potato".into(), "cherry".into(), "onion".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Soup);
        assert!(!kept.contains(&"cherry".to_string()));
        assert_eq!(removed.len(), 1);
        assert!(removed[0].0 == "cherry");
    }

    #[test]
    fn banana_removed_from_soup() {
        let slugs = vec!["chicken".into(), "banana".into(), "carrot".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Soup);
        assert!(!kept.contains(&"banana".to_string()));
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn chocolate_removed_from_stew() {
        let slugs = vec!["beef".into(), "chocolate".into(), "potato".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Stew);
        assert!(!kept.contains(&"chocolate".to_string()));
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn cherry_banned_with_salmon() {
        let slugs = vec!["salmon".into(), "cherry".into(), "spinach".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Grill);
        assert!(!kept.contains(&"cherry".to_string()));
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn normal_ingredients_pass_through() {
        let slugs = vec!["beef".into(), "potato".into(), "onion".into(), "carrot".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Soup);
        assert_eq!(kept.len(), 4);
        assert!(removed.is_empty());
    }

    #[test]
    fn chicken_banned_raw() {
        let slugs = vec!["chicken".into(), "tomato".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Raw);
        assert!(!kept.contains(&"chicken".to_string()));
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn lemon_allowed_with_fish() {
        // Lemon is NOT in the banned list — it's a classic fish pairing
        let slugs = vec!["salmon".into(), "lemon".into(), "dill".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Grill);
        assert_eq!(kept.len(), 3);
        assert!(removed.is_empty());
    }

    #[test]
    fn salad_allows_fruit() {
        // Fruits in salad are fine
        let slugs = vec!["spinach".into(), "strawberry".into(), "walnut".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Salad);
        assert_eq!(kept.len(), 3);
        assert!(removed.is_empty());
    }

    #[test]
    fn ice_cream_removed_from_soup() {
        let slugs = vec!["chicken".into(), "tomato".into(), "ice-cream".into(), "carrot".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Soup);
        assert!(!kept.contains(&"ice-cream".to_string()));
        assert_eq!(removed.len(), 1);
        assert!(removed[0].0 == "ice-cream");
    }

    #[test]
    fn ice_cream_underscore_removed_from_soup() {
        let slugs = vec!["chicken".into(), "ice_cream".into(), "onion".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Soup);
        assert!(!kept.contains(&"ice_cream".to_string()));
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn chocolate_removed_from_pasta() {
        let slugs = vec!["pasta".into(), "chocolate".into(), "cream".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Pasta);
        assert!(!kept.contains(&"chocolate".to_string()));
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn gelato_banned_with_beef() {
        let slugs = vec!["beef".into(), "gelato".into(), "potato".into()];
        let (kept, removed) = filter_ingredients(&slugs, DishType::Stew);
        assert!(!kept.contains(&"gelato".to_string()));
        assert_eq!(removed.len(), 1);
    }
}
