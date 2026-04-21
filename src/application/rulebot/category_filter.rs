//! Category filter for healthy_product intent.
//!
//! Extracts a food category from free-text user input and maps it to the
//! `product_type` column used in the ingredient cache. This is the "intent
//! engine" layer that prevents the bot from returning tuna and chicken when
//! the user asked for vegetables.
//!
//! Languages covered: RU / EN / PL / UK (matches the app's 4 locales).
//! Matching is substring-based on a lowercased input — we do NOT need full
//! morphology here; just common stems: "овощ" matches "овощи", "овощей", etc.

/// Detect a requested product category from free-text input.
///
/// Returns the `product_type` value that matches rows in `catalog_ingredients`.
/// Returns `None` when the user did not specify a category (then we fall back
/// to the goal-based ranking across all products).
pub fn detect_category(text: &str) -> Option<ProductCategory> {
    let t = text.to_lowercase();

    // Order matters: check more specific categories first (berry before fruit,
    // seafood before fish, etc.) to avoid false positives.
    const RULES: &[(ProductCategory, &[&str])] = &[
        // ── Vegetables ────────────────────────────────────────────────────
        (ProductCategory::Vegetable, &[
            // RU
            "овощ", "овощи", "овощн",
            // EN
            "vegetable", "veggie", "veggies", "veg ",
            // PL
            "warzyw", "warzywa",
            // UK
            "овоч",
        ]),
        // ── Fruits ────────────────────────────────────────────────────────
        (ProductCategory::Fruit, &[
            "фрукт", "фрукты", "fruit", "fruits", "owoc", "owoce", "овоч.фрукт",
        ]),
        // ── Berries (separate from fruit in our taxonomy) ────────────────
        (ProductCategory::Berry, &[
            "ягод", "berry", "berries", "jagod", "ягід",
        ]),
        // ── Seafood (check BEFORE fish — seafood is more specific) ───────
        (ProductCategory::Seafood, &[
            "морепродукт", "креветк", "кальмар", "мидии", "устриц",
            "seafood", "shrimp", "squid", "mussels", "oysters",
            "owoce morza", "krewetk", "kalmar",
            "морепродукт", "креветок",
        ]),
        // ── Fish ─────────────────────────────────────────────────────────
        (ProductCategory::Fish, &[
            "рыб", "fish", "ryb", "риб",
        ]),
        // ── Meat ─────────────────────────────────────────────────────────
        (ProductCategory::Meat, &[
            "мяс", "курин", "говяд", "свинин", "баран", "индейк",
            "meat", "chicken", "beef", "pork", "lamb", "turkey",
            "mięs", "wołowin", "wieprzowin", "kurczak",
            "м'яс", "куряч", "яловичин", "свинин",
        ]),
        // ── Dairy ────────────────────────────────────────────────────────
        (ProductCategory::Dairy, &[
            "молочн", "сыр", "творог", "йогурт", "кефир", "молоко",
            "dairy", "cheese", "yogurt", "yoghurt", "milk", "cottage",
            "nabiał", "ser", "jogurt", "mleko",
            "молочн", "сир", "йогурт", "кефір", "молоко",
        ]),
        // ── Grains ───────────────────────────────────────────────────────
        (ProductCategory::Grain, &[
            "крупа", "крупы", "зерн", "каша", "рис", "гречк", "овсян", "макарон",
            "grain", "cereal", "rice", "oats", "pasta", "buckwheat",
            "kasza", "zboż", "ryż", "owsian", "makaron",
            "крупа", "зерн", "каша", "рис", "гречк", "вівсян",
        ]),
        // ── Legumes ──────────────────────────────────────────────────────
        (ProductCategory::Legume, &[
            "бобов", "фасол", "чечевиц", "нут", "горох",
            "legume", "beans", "lentil", "chickpea",
            "strączk", "fasol", "soczewic", "ciecierzyc",
            "бобов", "квасол", "сочевиц",
        ]),
        // ── Nuts & seeds ─────────────────────────────────────────────────
        (ProductCategory::Nut, &[
            "орех", "семечк", "семена",
            "nut ", "nuts", "seed", "seeds", "almond", "walnut",
            "orzech", "nasion",
            "горіх", "насінн",
        ]),
        // ── Mushrooms ────────────────────────────────────────────────────
        (ProductCategory::Mushroom, &[
            "гриб", "mushroom", "grzyb", "гриб",
        ]),
        // ── Herbs / spices ───────────────────────────────────────────────
        (ProductCategory::Herb, &[
            "зелен", "травы", "herb", "zioł", "зелень", "трав",
        ]),
    ];

    for (cat, keywords) in RULES {
        for kw in *keywords {
            if t.contains(kw) {
                return Some(*cat);
            }
        }
    }
    None
}

/// High-level food category. Maps 1:1 to the `product_type` string used in
/// the `catalog_ingredients` table and `IngredientData.product_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductCategory {
    Vegetable,
    Fruit,
    Berry,
    Fish,
    Seafood,
    Meat,
    Dairy,
    Grain,
    Legume,
    Nut,
    Mushroom,
    Herb,
}

impl ProductCategory {
    /// Returns the `product_type` strings that belong to this category.
    /// Some categories are unions (e.g. Seafood also covers "fish" in some
    /// rows of the DB where classification is fuzzy).
    pub fn product_types(self) -> &'static [&'static str] {
        match self {
            ProductCategory::Vegetable => &["vegetable"],
            ProductCategory::Fruit     => &["fruit"],
            ProductCategory::Berry     => &["berry", "fruit"], // fall back to fruit if no berries tagged
            ProductCategory::Fish      => &["fish", "seafood"],
            ProductCategory::Seafood   => &["seafood", "fish"],
            ProductCategory::Meat      => &["meat"],
            ProductCategory::Dairy     => &["dairy"],
            ProductCategory::Grain     => &["grain"],
            ProductCategory::Legume    => &["legume"],
            ProductCategory::Nut       => &["nut"],
            ProductCategory::Mushroom  => &["mushroom"],
            ProductCategory::Herb      => &["herb", "spice"],
        }
    }

    /// Debug label for logging / tracing.
    pub fn as_str(self) -> &'static str {
        match self {
            ProductCategory::Vegetable => "vegetable",
            ProductCategory::Fruit     => "fruit",
            ProductCategory::Berry     => "berry",
            ProductCategory::Fish      => "fish",
            ProductCategory::Seafood   => "seafood",
            ProductCategory::Meat      => "meat",
            ProductCategory::Dairy     => "dairy",
            ProductCategory::Grain     => "grain",
            ProductCategory::Legume    => "legume",
            ProductCategory::Nut       => "nut",
            ProductCategory::Mushroom  => "mushroom",
            ProductCategory::Herb      => "herb",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ru_vegetable() {
        assert_eq!(detect_category("какой овощ полезный"), Some(ProductCategory::Vegetable));
        assert_eq!(detect_category("какие овощи полезны"), Some(ProductCategory::Vegetable));
        assert_eq!(detect_category("посоветуй овощей"), Some(ProductCategory::Vegetable));
    }

    #[test]
    fn detects_en_vegetable() {
        assert_eq!(detect_category("what vegetables are healthy"), Some(ProductCategory::Vegetable));
        assert_eq!(detect_category("recommend a veggie"), Some(ProductCategory::Vegetable));
    }

    #[test]
    fn detects_fish_before_seafood_collision() {
        assert_eq!(detect_category("какая рыба полезна"), Some(ProductCategory::Fish));
        assert_eq!(detect_category("креветки полезны?"), Some(ProductCategory::Seafood));
    }

    #[test]
    fn no_category_for_generic_query() {
        assert_eq!(detect_category("что полезно есть"), None);
        assert_eq!(detect_category("hello"), None);
    }

    #[test]
    fn detects_meat() {
        assert_eq!(detect_category("какое мясо выбрать"), Some(ProductCategory::Meat));
        assert_eq!(detect_category("best chicken breast"), Some(ProductCategory::Meat));
    }

    #[test]
    fn detects_dairy() {
        assert_eq!(detect_category("полезные молочные продукты"), Some(ProductCategory::Dairy));
        assert_eq!(detect_category("what cheese is best"), Some(ProductCategory::Dairy));
    }
}
