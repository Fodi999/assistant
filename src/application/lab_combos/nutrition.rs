// ─── Nutrition Calculator (Single Source of Truth) ──────────────────────────
//
// USDA-based per-100g nutrition lookup + default portion sizes.
// This module is the ONLY source of nutrition numbers in the system.
// All display fields, AI prompts, and SEO text read from NutritionTotals.

/// Pre-calculated nutrition totals for a combo.
/// This is the SINGLE SOURCE OF TRUTH — all display reads from these values.
#[derive(Debug, Clone)]
pub struct NutritionTotals {
    pub total_weight_g: f64,
    pub servings_count: i16,
    pub calories_total: f64,
    pub protein_total: f64,
    pub fat_total: f64,
    pub carbs_total: f64,
    pub fiber_total: f64,
    pub calories_per_serving: f64,
    pub protein_per_serving: f64,
    pub fat_per_serving: f64,
    pub carbs_per_serving: f64,
    pub fiber_per_serving: f64,
    /// Per-ingredient breakdown for the AI prompt
    pub breakdown: Vec<String>,
}

/// Calculate nutrition totals from ingredient list.
/// Uses USDA-based lookup table × default portion sizes.
/// Returns one authoritative NutritionTotals — used everywhere.
pub fn calculate_nutrition(ingredients: &[String]) -> NutritionTotals {
    let mut total_weight = 0.0_f64;
    let mut total_kcal = 0.0_f64;
    let mut total_protein = 0.0_f64;
    let mut total_fat = 0.0_f64;
    let mut total_carbs = 0.0_f64;
    let mut total_fiber = 0.0_f64;
    let mut breakdown = Vec::new();

    for ing in ingredients {
        let (kcal100, prot100, fat100, carbs100, fiber100) = nutrition_per_100g(ing);
        let portion = default_portion_grams(ing);

        let kcal = kcal100 * portion / 100.0;
        let prot = prot100 * portion / 100.0;
        let fat = fat100 * portion / 100.0;
        let carbs = carbs100 * portion / 100.0;
        let fiber = fiber100 * portion / 100.0;

        total_weight += portion;
        total_kcal += kcal;
        total_protein += prot;
        total_fat += fat;
        total_carbs += carbs;
        total_fiber += fiber;

        breakdown.push(format!(
            "- {} ({}g): {:.0} kcal, {:.1}g protein, {:.1}g fat, {:.1}g carbs",
            ing, portion, kcal, prot, fat, carbs
        ));
    }

    // 1 serving = whole recipe (single portion)
    let servings: i16 = 1;
    NutritionTotals {
        total_weight_g: total_weight,
        servings_count: servings,
        calories_total: total_kcal,
        protein_total: total_protein,
        fat_total: total_fat,
        carbs_total: total_carbs,
        fiber_total: total_fiber,
        calories_per_serving: total_kcal,
        protein_per_serving: total_protein,
        fat_per_serving: total_fat,
        carbs_per_serving: total_carbs,
        fiber_per_serving: total_fiber,
        breakdown,
    }
}

/// Per-100g USDA-based nutrition data for common ingredients.
/// (kcal, protein, fat, carbs, fiber)
pub fn nutrition_per_100g(ingredient: &str) -> (f64, f64, f64, f64, f64) {
    let name = ingredient.to_lowercase();
    // Fish & seafood
    if name.contains("salmon")   { return (208.0, 20.0, 13.0,  0.0, 0.0); }
    if name.contains("tuna")     { return (132.0, 23.0,  5.0,  0.0, 0.0); }
    if name.contains("cod")      { return ( 82.0, 17.0,  0.7,  0.0, 0.0); }
    if name.contains("shrimp") || name.contains("prawn") { return ( 99.0, 24.0,  0.3,  0.2, 0.0); }
    if name.contains("mackerel") { return (205.0, 19.0, 14.0,  0.0, 0.0); }
    if name.contains("trout")    { return (148.0, 20.0,  7.0,  0.0, 0.0); }
    if name.contains("sardine")  { return (208.0, 21.0, 11.0,  0.0, 0.0); }
    // Poultry & meat
    if name.contains("chicken")  { return (165.0, 31.0,  3.6,  0.0, 0.0); }
    if name.contains("turkey")   { return (157.0, 29.0,  3.2,  0.0, 0.0); }
    if name.contains("beef")     { return (250.0, 26.0, 15.0,  0.0, 0.0); }
    if name.contains("pork")     { return (242.0, 25.0, 14.0,  0.0, 0.0); }
    if name.contains("lamb")     { return (258.0, 25.0, 17.0,  0.0, 0.0); }
    if name.contains("duck")     { return (201.0, 19.0, 14.0,  0.0, 0.0); }
    // Eggs & dairy
    if name.contains("egg")      { return (155.0, 13.0, 11.0,  1.1, 0.0); }
    if name.contains("cheese")   { return (350.0, 22.0, 28.0,  1.3, 0.0); }
    if name.contains("yogurt") || name.contains("yoghurt") { return ( 59.0,  5.0,  0.4,  3.6, 0.0); }
    // Legumes & plant protein
    if name.contains("tofu")     { return ( 76.0,  8.0,  4.8,  1.9, 0.3); }
    if name.contains("tempeh")   { return (193.0, 19.0, 11.0,  9.4, 0.0); }
    if name.contains("lentil")   { return (116.0,  9.0,  0.4, 20.0, 7.9); }
    if name.contains("chickpea") { return (164.0,  8.5,  2.6, 27.0, 7.6); }
    if name.contains("bean")     { return (127.0,  8.0,  0.5, 22.0, 7.4); }
    // Grains (cooked values)
    if name.contains("quinoa")   { return (120.0,  4.4,  1.9, 21.0, 2.8); }
    if name.contains("rice")     { return (130.0,  2.7,  0.3, 28.0, 0.4); }
    if name.contains("pasta") || name.contains("noodle") { return (131.0,  5.0,  1.1, 25.0, 1.8); }
    if name.contains("oat")      { return ( 68.0,  2.4,  1.4, 12.0, 1.7); }
    if name.contains("bread")    { return (265.0,  9.0,  3.2, 49.0, 2.7); }
    if name.contains("potato")   { return ( 77.0,  2.0,  0.1, 17.0, 2.2); }
    // Vegetables
    if name.contains("broccoli") { return ( 34.0,  2.8,  0.4,  7.0, 2.6); }
    if name.contains("spinach")  { return ( 23.0,  2.9,  0.4,  3.6, 2.2); }
    if name.contains("tomato")   { return ( 18.0,  0.9,  0.2,  3.9, 1.2); }
    if name.contains("cucumber") { return ( 15.0,  0.7,  0.1,  3.6, 0.5); }
    if name.contains("pepper") || name.contains("paprika") { return ( 31.0,  1.0,  0.3,  6.0, 2.1); }
    if name.contains("onion")    { return ( 40.0,  1.1,  0.1,  9.3, 1.7); }
    if name.contains("carrot")   { return ( 41.0,  0.9,  0.2,  9.6, 2.8); }
    if name.contains("zucchini") || name.contains("courgette") { return ( 17.0,  1.2,  0.3,  3.1, 1.0); }
    if name.contains("sweet-potato") || name.contains("sweet_potato") { return ( 86.0, 1.6, 0.1, 20.0, 3.0); }
    if name.contains("mushroom") { return ( 22.0,  3.1,  0.3,  3.3, 1.0); }
    if name.contains("asparagus") { return ( 20.0, 2.2, 0.1, 3.9, 2.1); }
    if name.contains("cauliflower") { return ( 25.0, 1.9, 0.3, 5.0, 2.0); }
    if name.contains("eggplant") || name.contains("aubergine") { return ( 25.0, 1.0, 0.2, 6.0, 3.0); }
    if name.contains("kale")     { return ( 49.0,  4.3,  0.9,  8.8, 3.6); }
    if name.contains("pea")      { return ( 81.0,  5.4,  0.4, 14.0, 5.1); }
    if name.contains("corn")     { return ( 86.0,  3.3,  1.2, 19.0, 2.7); }
    if name.contains("cabbage")  { return ( 25.0,  1.3,  0.1,  5.8, 2.5); }
    if name.contains("lettuce") || name.contains("arugula") { return ( 15.0, 1.4, 0.2, 2.9, 1.3); }
    // Fruits & fats
    if name.contains("avocado")  { return (160.0,  2.0, 15.0,  9.0, 7.0); }
    if name.contains("banana")   { return ( 89.0,  1.1,  0.3, 23.0, 2.6); }
    if name.contains("apple")    { return ( 52.0,  0.3,  0.2, 14.0, 2.4); }
    if name.contains("olive")    { return (115.0,  0.8, 11.0,  6.0, 3.2); }
    if name.contains("coconut")  { return (354.0,  3.3, 33.0, 15.0, 9.0); }
    // Nuts & seeds
    if name.contains("almond")   { return (579.0, 21.0, 50.0, 22.0, 12.0); }
    if name.contains("walnut")   { return (654.0, 15.0, 65.0, 14.0, 6.7); }
    if name.contains("sesame") || name.contains("tahini") { return (573.0, 17.0, 50.0, 23.0, 12.0); }
    if name.contains("peanut")   { return (567.0, 26.0, 49.0, 16.0, 8.5); }
    // Default: unknown ingredient (moderate vegetable)
    (35.0, 1.5, 0.3, 7.0, 2.0)
}

/// Default portion size (grams) for each ingredient type.
pub fn default_portion_grams(ingredient: &str) -> f64 {
    let name = ingredient.to_lowercase();
    // Proteins: larger portions
    if name.contains("salmon") || name.contains("tuna") || name.contains("chicken")
        || name.contains("beef") || name.contains("pork") || name.contains("turkey")
        || name.contains("lamb") || name.contains("cod") || name.contains("shrimp")
        || name.contains("duck") || name.contains("trout") || name.contains("mackerel")
    {
        return 150.0;
    }
    // Eggs
    if name.contains("egg") { return 100.0; }
    // Grains/starches (cooked)
    if name.contains("rice") || name.contains("pasta") || name.contains("quinoa")
        || name.contains("noodle") || name.contains("oat")
    {
        return 100.0;
    }
    if name.contains("potato") || name.contains("sweet-potato") { return 150.0; }
    if name.contains("bread") { return 60.0; }
    // Avocado, cheese — smaller
    if name.contains("avocado") { return 80.0; }
    if name.contains("cheese")  { return 30.0; }
    // Nuts/seeds — small portion
    if name.contains("almond") || name.contains("walnut") || name.contains("peanut")
        || name.contains("sesame") || name.contains("tahini")
    {
        return 20.0;
    }
    // Legumes
    if name.contains("lentil") || name.contains("chickpea") || name.contains("bean")
        || name.contains("tofu") || name.contains("tempeh")
    {
        return 120.0;
    }
    // Default vegetable/fruit
    80.0
}
