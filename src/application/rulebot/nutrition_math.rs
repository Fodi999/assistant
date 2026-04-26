//! Nutrition Math — portions, yields, roles, КБЖУ, allergens, diet tags.
//!
//! Responsibilities:
//!   - `build_ingredient_for_dish`: role → method → portion → gross/net/yield → КБЖУ
//!   - `override_role`: correct misclassified aromatics/oils/condiments
//!   - `recipe_portion` / `recipe_portion_goal`: goal-aware grams
//!   - `edible_yield_for`: waste coefficients (potato 0.80, garlic 0.62…)
//!   - `detect_allergens` / `detect_diet_tags`: analysis helpers
//!   - `compute_complexity`: easy/medium/hard from step count + time
//!
//! Pure math — no IO, no LLM, no cache lookups.

use crate::infrastructure::ingredient_cache::IngredientData;
use super::meal_builder::CookMethod;
use super::response_builder::HealthGoal;
use super::recipe_engine::{ResolvedIngredient, DishType, CookingStep};

// ── Build Ingredient ─────────────────────────────────────────────────────────

/// Build a fully-resolved ingredient from a cache product (default dish type).
pub fn build_ingredient(product: &IngredientData, slug_hint: &str, goal: HealthGoal) -> ResolvedIngredient {
    build_ingredient_for_dish(product, slug_hint, goal, DishType::Default)
}

/// Build ingredient with dish-aware cooking method.
/// Uses DDD cooking_rules to determine method by (dish_type, role, slug).
pub fn build_ingredient_for_dish(
    product: &IngredientData,
    slug_hint: &str,
    goal: HealthGoal,
    dish_type: DishType,
) -> ResolvedIngredient {
    let role = override_role(product);
    let method = dish_type.cook_method(role, &product.slug, &product.product_type, goal);
    let state = method_to_state(&method);

    let cooked_portion = recipe_portion_goal(product, role, goal);
    let yield_factor = method.yield_factor(&product.product_type);
    let cleaned_net = cooked_portion / yield_factor;
    let edible = edible_yield_for(&product.product_type, &product.slug);
    let gross = cleaned_net / edible;

    ResolvedIngredient {
        product: Some(product.clone()),
        slug_hint: slug_hint.to_string(),
        resolved_slug: Some(product.slug.clone()),
        state: state.into(),
        role: role.into(),
        gross_g: round1(gross),
        cleaned_net_g: round1(cleaned_net),
        cooked_net_g: round1(cooked_portion),
        kcal: product.kcal_for(gross),
        protein_g: product.protein_for(gross),
        fat_g: product.fat_for(gross),
        carbs_g: product.carbs_for(gross),
    }
}

// ── Role Assignment ──────────────────────────────────────────────────────────

/// Aromatics, oils, condiments that meal_role() misclassifies as "side".
pub fn override_role(product: &IngredientData) -> &'static str {
    let slug = product.slug.as_str();
    match slug {
        "garlic" | "ginger" | "chili" | "chili-pepper" | "horseradish"
        | "turmeric" | "lemongrass" | "shallot" => return "spice",
        "salt" | "pepper" | "cumin" | "paprika" | "cinnamon" | "nutmeg"
        | "coriander" | "bay-leaf" | "saffron" | "vanilla" => return "spice",
        "olive-oil" | "sunflower-oil" | "butter" | "ghee" | "coconut-oil"
        | "sesame-oil" => return "oil",
        "soy-sauce" | "vinegar" | "mustard" | "ketchup" | "mayo"
        | "tomato-paste" | "fish-sauce" | "worcestershire" => return "condiment",
        _ => {}
    }
    // Fruits that act as a binder/base in sweet dishes (banana pancakes, apple cake…).
    // They are the structural/sweetening backbone — not a garnish ("side").
    if product.product_type == "fruit"
        && ["banana", "apple", "pear", "mango", "date", "fig", "plantain"]
            .iter().any(|k| slug.contains(k))
    {
        return "base";
    }
    match product.product_type.as_str() {
        "oil" | "fat" => "oil",
        "spice" | "herb" | "seasoning" => "spice",
        "condiment" | "sauce" => "condiment",
        _ => product.meal_role(),
    }
}

// ── Portions ─────────────────────────────────────────────────────────────────

/// Recipe-specific portion (grams of cooked food on plate).
pub fn recipe_portion(product: &IngredientData, role: &str) -> f32 {
    match role {
        "protein" => match product.product_type.as_str() {
            "meat" | "fish" | "seafood" => 100.0,
            "eggs" => 60.0,
            "dairy" => 50.0,
            _ => 80.0,
        },
        "base" => match product.product_type.as_str() {
            "grain" | "legume" => 60.0,
            _ => 80.0,
        },
        "side" => match product.product_type.as_str() {
            "vegetable" | "mushroom" => 50.0,
            "fruit" => 40.0,
            _ => 50.0,
        },
        "spice" => 5.0,
        "oil" => 15.0,
        "condiment" => 15.0,
        _ => 30.0,
    }
}

/// Goal-aware portion: LowCalorie = less oil/meat, more veg.
pub fn recipe_portion_goal(product: &IngredientData, role: &str, goal: HealthGoal) -> f32 {
    let base = recipe_portion(product, role);
    match goal {
        HealthGoal::LowCalorie => match role {
            "oil"     => 5.0,
            "protein" => base * 0.8,
            "side"    => base * 1.2,
            "condiment" => base * 0.5,
            _ => base,
        },
        HealthGoal::HighProtein => match role {
            "protein" => base * 1.3,
            "side"    => base * 0.8,
            _ => base,
        },
        HealthGoal::Balanced => base,
    }
}

pub fn method_to_state(method: &CookMethod) -> &'static str {
    match method {
        CookMethod::Grill => "grilled",
        CookMethod::Bake => "baked",
        CookMethod::Boil => "boiled",
        CookMethod::Steam => "steamed",
        CookMethod::Fry => "fried",
        CookMethod::Saute => "sauteed",
        CookMethod::Raw => "raw",
    }
}

// ── Yield Tables ─────────────────────────────────────────────────────────────

pub fn edible_yield_for(product_type: &str, slug: &str) -> f32 {
    let specific = match slug {
        s if s.contains("potato") => Some(0.80),
        s if s.contains("carrot") => Some(0.82),
        s if s.contains("onion") => Some(0.84),
        s if s.contains("garlic") => Some(0.62),
        s if s.contains("shrimp") => Some(0.65),
        s if s.contains("walnut") || s.contains("almond") || s.contains("pistachio") => Some(0.55),
        s if s.contains("banana") => Some(0.64),
        s if s.contains("lemon") || s.contains("orange") => Some(0.65),
        s if s.contains("avocado") => Some(0.67),
        s if s.contains("pumpkin") => Some(0.70),
        s if s.contains("cabbage") => Some(0.80),
        _ => None,
    };
    if let Some(y) = specific { return y; }

    match product_type {
        "fish" => 0.80,
        "meat" => 0.92,
        "seafood" => 0.70,
        "vegetable" => 0.88,
        "fruit" => 0.82,
        "mushroom" => 0.95,
        _ => 1.0,
    }
}

pub fn round1(v: f32) -> f32 { (v * 10.0).round() / 10.0 }

// ── Dish Context Analysis ────────────────────────────────────────────────────

/// Complexity: easy (<= 4 steps && <= 20 min), hard (>= 8 steps || >= 60 min), else medium.
pub fn compute_complexity(steps: &[CookingStep]) -> String {
    let total_min: u16 = steps.iter().filter_map(|s| s.time_min).sum();
    let n = steps.len();
    if n <= 4 && total_min <= 20 {
        "easy".into()
    } else if n >= 8 || total_min >= 60 {
        "hard".into()
    } else {
        "medium".into()
    }
}

/// Detect allergens/intolerances from ingredient product_types and slugs.
pub fn detect_allergens(ingredients: &[ResolvedIngredient]) -> Vec<String> {
    let mut flags = Vec::new();
    let has = |f: &dyn Fn(&ResolvedIngredient) -> bool| ingredients.iter().any(f);

    // Gluten
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        s.contains("wheat") || s.contains("flour") || s.contains("pasta")
            || s.contains("bread") || s.contains("barley") || s.contains("rye")
            || s.contains("oat") || s.contains("spaghetti") || s.contains("noodle")
            || s.contains("couscous") || s.contains("semolina")
            || (pt == "grain" && !s.contains("rice") && !s.contains("corn") && !s.contains("buckwheat") && !s.contains("quinoa"))
    }) {
        flags.push("gluten".into());
    }

    // Lactose — only genuinely high-lactose products.
    // Butter / ghee contain only trace lactose (< 0.1 g/100 g) and should NOT
    // be flagged — they are cooking fats, not dairy allergens in practice.
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        let is_fat = s.contains("butter") || s.contains("ghee")
            || pt == "oil" || pt == "fat";
        if is_fat { return false; }
        pt == "dairy" || s.contains("milk") || s.contains("cream") || s.contains("cheese")
            || s.contains("yogurt") || s.contains("kefir")
            || s.contains("sour-cream") || s.contains("smetana")
    }) {
        flags.push("lactose".into());
    }

    // Nuts
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        pt == "nut" || s.contains("almond") || s.contains("walnut") || s.contains("cashew")
            || s.contains("peanut") || s.contains("hazelnut") || s.contains("pecan")
            || s.contains("pistachio") || s.contains("macadamia") || s.contains("pine-nut")
    }) {
        flags.push("nuts".into());
    }

    // Eggs
    if has(&|i| { i.slug_hint.to_lowercase().contains("egg") }) {
        flags.push("eggs".into());
    }

    // Fish
    if has(&|i| {
        i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("") == "fish"
    }) {
        flags.push("fish".into());
    }

    // Shellfish / Seafood
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        pt == "seafood" || s.contains("shrimp") || s.contains("prawn") || s.contains("crab")
            || s.contains("lobster") || s.contains("mussel") || s.contains("oyster")
            || s.contains("squid") || s.contains("octopus") || s.contains("clam")
    }) {
        flags.push("shellfish".into());
    }

    // Soy
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        s.contains("soy") || s.contains("tofu") || s.contains("edamame") || s.contains("tempeh")
    }) {
        flags.push("soy".into());
    }

    flags
}

/// Detect diet tags: vegan, vegetarian, pescatarian, etc.
pub fn detect_diet_tags(ingredients: &[ResolvedIngredient]) -> Vec<String> {
    let mut tags = Vec::new();

    let types: Vec<&str> = ingredients
        .iter()
        .filter_map(|i| i.product.as_ref().map(|p| p.product_type.as_str()))
        .collect();

    let slugs: Vec<String> = ingredients.iter().map(|i| i.slug_hint.to_lowercase()).collect();

    let has_meat = types.iter().any(|t| *t == "meat");
    let has_fish = types.iter().any(|t| *t == "fish");
    let has_seafood = types.iter().any(|t| *t == "seafood");
    let has_dairy = types.iter().any(|t| *t == "dairy");
    let has_eggs = slugs.iter().any(|s| s.contains("egg"));

    if !has_meat && !has_fish && !has_seafood && !has_dairy && !has_eggs {
        tags.push("vegan".into());
        tags.push("vegetarian".into());
    } else if !has_meat && !has_fish && !has_seafood {
        tags.push("vegetarian".into());
    } else if !has_meat && (has_fish || has_seafood) {
        tags.push("pescatarian".into());
    }

    tags
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edible_yield_potato() {
        let y = edible_yield_for("vegetable", "potato");
        assert!((y - 0.80).abs() < 0.01);
    }

    #[test]
    fn edible_yield_default() {
        let y = edible_yield_for("dairy", "milk");
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
        assert_eq!(recipe_portion(&meat, "protein"), 100.0);

        let oil = IngredientData {
            slug: "olive-oil".into(), name_en: "Olive Oil".into(),
            name_ru: "".into(), name_pl: "".into(), name_uk: "".into(),
            calories_per_100g: 884.0, protein_per_100g: 0.0,
            fat_per_100g: 100.0, carbs_per_100g: 0.0, image_url: None,
            product_type: "oil".into(), density_g_per_ml: None, behaviors: vec![], states: vec![],
        };
        assert_eq!(recipe_portion(&oil, "oil"), 15.0);
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
}
