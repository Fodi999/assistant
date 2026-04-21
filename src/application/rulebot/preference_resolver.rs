//! Preference Resolver — map `user_preferences` text fields to product slugs.
//!
//! Transforms free-text allergy/dislike/like entries (e.g. "яйца", "peanuts",
//! "orzechy", "молочка") into concrete product slugs that exist in the
//! `IngredientCache`. Also expands allergen groups to their member slugs.
//!
//! Used by `chat_engine::handle_chat_personalized` to produce a hard-exclusion
//! list that flows through `SessionContext::excluded_slugs()` into every
//! product-selection pipeline.
//!
//! Design constraints:
//!   • Stateless, pure function over cache snapshot — O(n_prefs × n_cache).
//!   • Multilingual: scans user text against name_en / name_ru / name_pl / name_uk.
//!   • Allergen groups (e.g. "nuts", "dairy", "молочка") expand to all
//!     members of that product_type / allergen class.
//!   • Graceful: unresolved entries are logged and ignored, never error out.

use crate::domain::user_preferences::UserPreferences;
use crate::infrastructure::ingredient_cache::{IngredientCache, IngredientData};

/// Resolved personalization hints for the current turn.
pub struct PreferenceHints {
    /// Slugs to HARD-exclude: allergies ∪ intolerances ∪ dislikes.
    pub excludes: Vec<String>,
    /// Slugs the user explicitly likes (soft boost, not include).
    pub likes: Vec<String>,
}

/// Resolve `UserPreferences` text fields to product slugs via the ingredient cache.
///
/// Called on every turn when a `user_id` is present.
pub async fn resolve(prefs: &UserPreferences, cache: &IngredientCache) -> PreferenceHints {
    let all = cache.all().await;

    // ── Hard exclusions: allergies + intolerances + dislikes ──────────────
    let mut excludes = Vec::<String>::new();

    for entry in prefs.allergies.iter()
        .chain(prefs.intolerances.iter())
        .chain(prefs.dislikes.iter())
    {
        for slug in match_entry(entry, &all) {
            if !excludes.contains(&slug) {
                excludes.push(slug);
            }
        }
    }

    // ── Soft likes ────────────────────────────────────────────────────────
    let mut likes = Vec::<String>::new();
    for entry in &prefs.likes {
        for slug in match_entry(entry, &all) {
            if !likes.contains(&slug) {
                likes.push(slug);
            }
        }
    }

    if !excludes.is_empty() || !likes.is_empty() {
        tracing::debug!(
            "🎯 preferences resolved: {} excludes, {} likes (from {} allergies + {} intolerances + {} dislikes + {} likes)",
            excludes.len(), likes.len(),
            prefs.allergies.len(), prefs.intolerances.len(),
            prefs.dislikes.len(), prefs.likes.len()
        );
    }

    PreferenceHints { excludes, likes }
}

/// Match one preference entry (free text) against the cache.
/// Returns all matching slugs — a single entry may match multiple products
/// when it's a group keyword ("nuts", "молочка").
fn match_entry(entry: &str, all: &[IngredientData]) -> Vec<String> {
    let needle = entry.trim().to_lowercase();
    if needle.is_empty() || needle.len() < 2 {
        return vec![];
    }

    // ── Step 1: allergen / category groups ────────────────────────────────
    // Expand broad terms like "nuts", "dairy", "молочка" → all members
    // of that product_type. These keywords are intentionally shorter than
    // stem-matching thresholds so they take precedence.
    if let Some(group) = match_group(&needle) {
        return all.iter()
            .filter(|p| p.product_type.eq_ignore_ascii_case(group))
            .map(|p| p.slug.clone())
            .collect();
    }

    // ── Step 2: direct name match (any language) ──────────────────────────
    // Match in BOTH directions: user text contains product name, OR
    // product name contains user text (handles "яйцо" matching "Яйца").
    let mut hits = Vec::<String>::new();
    for p in all {
        let names = [
            p.name_en.to_lowercase(),
            p.name_ru.to_lowercase(),
            p.name_pl.to_lowercase(),
            p.name_uk.to_lowercase(),
            p.slug.replace('-', " "),
        ];

        for name in &names {
            if name.len() < 3 { continue; } // avoid false positives on "рис"/"oat"
            if needle.contains(name.as_str()) || name.contains(needle.as_str()) {
                if !hits.contains(&p.slug) {
                    hits.push(p.slug.clone());
                }
                break;
            }
        }
    }

    hits
}

/// Map a group/allergen keyword to a `product_type` from the catalog.
/// Returns None if the word isn't a known group name.
fn match_group(needle: &str) -> Option<&'static str> {
    // Dairy
    if matches!(needle,
        "dairy" | "молочка" | "молочное" | "молочные" | "молоко"
      | "nabiał" | "молочні" | "молочка"
    ) {
        return Some("dairy");
    }
    // Nuts
    if matches!(needle,
        "nuts" | "орехи" | "орех" | "орехов" | "orzechy" | "горіхи" | "горіх"
    ) {
        return Some("nut");
    }
    // Fish (FYI most allergy lists say "fish" separately from "seafood")
    if matches!(needle,
        "fish" | "рыба" | "рыбу" | "ryby" | "ryba" | "риба"
    ) {
        return Some("fish");
    }
    if matches!(needle,
        "seafood" | "морепродукты" | "морепродукт" | "owoce morza" | "морепродукти"
    ) {
        return Some("seafood");
    }
    // Legumes
    if matches!(needle,
        "legumes" | "бобовые" | "бобові" | "strączki"
    ) {
        return Some("legume");
    }
    None
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_p(slug: &str, ru: &str, en: &str, product_type: &str) -> IngredientData {
        IngredientData {
            slug: slug.into(),
            name_en: en.into(),
            name_ru: ru.into(),
            name_pl: en.into(),
            name_uk: en.into(),
            calories_per_100g: 100.0,
            protein_per_100g: 1.0,
            fat_per_100g: 1.0,
            carbs_per_100g: 1.0,
            image_url: None,
            product_type: product_type.into(),
            density_g_per_ml: None,
            behaviors: vec![],
            states: vec![],
        }
    }

    #[test]
    fn group_dairy_expands() {
        let all = vec![
            make_p("milk", "Молоко", "Milk", "dairy"),
            make_p("cheese", "Сыр", "Cheese", "dairy"),
            make_p("chicken-breast", "Куриное филе", "Chicken breast", "meat"),
        ];
        let hits = match_entry("молочка", &all);
        assert_eq!(hits.len(), 2);
        assert!(hits.contains(&"milk".to_string()));
        assert!(hits.contains(&"cheese".to_string()));
    }

    #[test]
    fn group_nuts_expands() {
        let all = vec![
            make_p("almond", "Миндаль", "Almonds", "nut"),
            make_p("walnut", "Грецкий орех", "Walnuts", "nut"),
            make_p("apple", "Яблоко", "Apple", "fruit"),
        ];
        let hits = match_entry("nuts", &all);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn direct_name_ru() {
        let all = vec![
            make_p("eggs", "Яйца", "Eggs", "dairy"),
            make_p("apple", "Яблоко", "Apple", "fruit"),
        ];
        assert_eq!(match_entry("яйца", &all), vec!["eggs".to_string()]);
    }

    #[test]
    fn empty_and_short_rejected() {
        let all = vec![make_p("rice", "Рис", "Rice", "grain")];
        assert!(match_entry("", &all).is_empty());
        assert!(match_entry("a", &all).is_empty());
    }

    #[test]
    fn partial_needle_matches_longer_name() {
        // User wrote "грецк" — stem for грецкий орех.
        let all = vec![make_p("walnut", "Грецкий орех", "Walnuts", "nut")];
        let hits = match_entry("грецкий орех", &all);
        assert_eq!(hits, vec!["walnut".to_string()]);
    }
}
