//! Recipe Analyzer — domain layer
//!
//! Aggregates ingredient data to produce a complete recipe analysis:
//! - Total nutrition (sum of scaled NutritionBreakdown)
//! - Per-portion nutrition
//! - Flavor balance (via FlavorVector aggregation)
//! - Diet compatibility (intersect diet flags)
//! - Cost analysis
//!
//! No DB, no HTTP — pure functions only.

use serde::{Deserialize, Serialize};
use crate::domain::tools::unit_converter as uc;
use crate::domain::tools::nutrition::{self, NutritionBreakdown, MacrosRatio};
use crate::domain::tools::flavor_graph::{self, FlavorVector, FlavorIngredient, FlavorBalance};

// ── Input ────────────────────────────────────────────────────────────────────

/// One ingredient entry for recipe analysis.
#[derive(Debug, Clone)]
pub struct RecipeIngredientInput {
    pub slug:           String,
    pub grams:          f64,
    /// Per-100g nutrition from catalog
    pub nutrition_100g: NutritionBreakdown,
    /// Culinary flavor vector from food_culinary_properties
    pub flavor:         FlavorVector,
    /// Cost per kg (optional)
    pub cost_per_kg:    Option<f64>,
    /// Diet flags
    pub diet_flags:     DietFlags,
}

/// Boolean diet flags for intersection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DietFlags {
    pub vegan:         bool,
    pub vegetarian:    bool,
    pub keto:          bool,
    pub paleo:         bool,
    pub gluten_free:   bool,
    pub mediterranean: bool,
    pub low_carb:      bool,
}

impl DietFlags {
    /// Intersect: result is true only if ALL ingredients have the flag.
    pub fn intersect(&self, other: &DietFlags) -> DietFlags {
        DietFlags {
            vegan:         self.vegan         && other.vegan,
            vegetarian:    self.vegetarian    && other.vegetarian,
            keto:          self.keto          && other.keto,
            paleo:         self.paleo         && other.paleo,
            gluten_free:   self.gluten_free   && other.gluten_free,
            mediterranean: self.mediterranean && other.mediterranean,
            low_carb:      self.low_carb      && other.low_carb,
        }
    }

    /// Returns list of active diet labels
    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.vegan         { labels.push("vegan"); }
        if self.vegetarian    { labels.push("vegetarian"); }
        if self.keto          { labels.push("keto"); }
        if self.paleo         { labels.push("paleo"); }
        if self.gluten_free   { labels.push("gluten_free"); }
        if self.mediterranean { labels.push("mediterranean"); }
        if self.low_carb      { labels.push("low_carb"); }
        labels
    }
}

// ── Output ───────────────────────────────────────────────────────────────────

/// Complete recipe analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeAnalysis {
    /// Total nutrition for entire recipe
    pub total_nutrition: NutritionBreakdown,
    /// Per-portion nutrition
    pub per_portion:     NutritionBreakdown,
    /// Macros ratio (% of kcal from P/F/C)
    pub macros:          MacrosRatio,
    /// Nutrition score (0–100)
    pub nutrition_score: u8,
    /// Flavor balance analysis
    pub flavor:          FlavorBalance,
    /// Diet compatibility (intersection of all ingredients)
    pub diet_flags:      DietFlags,
    /// Total recipe weight in grams
    pub total_weight_g:  f64,
    /// Number of portions
    pub portions:        u32,
    /// Cost breakdown (if cost data available)
    pub cost:            Option<CostBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub total_cost:     f64,
    pub cost_per_portion: f64,
    pub ingredients:    Vec<IngredientCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngredientCost {
    pub slug:  String,
    pub grams: f64,
    pub cost:  f64,
}

// ── Core analysis function ───────────────────────────────────────────────────

/// Analyze a full recipe: nutrition + flavor + diet + cost.
pub fn analyze_recipe(
    ingredients: &[RecipeIngredientInput],
    portions: u32,
) -> RecipeAnalysis {
    let portions = portions.max(1);
    let total_weight_g: f64 = ingredients.iter().map(|i| i.grams).sum();

    // ── 1. Aggregate nutrition ──
    let mut total = NutritionBreakdown::zero();
    for ing in ingredients {
        let factor = ing.grams / 100.0;
        let scaled = ing.nutrition_100g.scale(factor);
        total = add_nutrition(&total, &scaled);
    }
    let per_portion = total.scale(1.0 / portions as f64);

    // ── 2. Macros ratio ──
    let macros = nutrition::macros_ratio(
        per_portion.protein_g,
        per_portion.fat_g,
        per_portion.carbs_g,
    );

    // ── 3. Nutrition score (per portion) ──
    let nscore = nutrition::nutrition_score(
        per_portion.calories,
        per_portion.protein_g,
        per_portion.fat_g,
        per_portion.carbs_g,
        per_portion.fiber_g,
        per_portion.sugar_g,
        per_portion.salt_g,
    );

    // ── 4. Flavor analysis ──
    let flavor_ingredients: Vec<FlavorIngredient> = ingredients.iter().map(|i| {
        FlavorIngredient {
            slug: i.slug.clone(),
            grams: i.grams,
            flavor: i.flavor.clone(),
        }
    }).collect();
    let flavor = flavor_graph::analyze_balance(&flavor_ingredients);

    // ── 5. Diet flags intersection ──
    let diet_flags = if ingredients.is_empty() {
        DietFlags::default()
    } else {
        let mut flags = ingredients[0].diet_flags.clone();
        for ing in &ingredients[1..] {
            flags = flags.intersect(&ing.diet_flags);
        }
        flags
    };

    // ── 6. Cost ──
    let cost = compute_cost(ingredients, portions);

    RecipeAnalysis {
        total_nutrition: total,
        per_portion,
        macros,
        nutrition_score: nscore,
        flavor,
        diet_flags,
        total_weight_g: uc::round_to(total_weight_g, 1),
        portions,
        cost,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn add_nutrition(a: &NutritionBreakdown, b: &NutritionBreakdown) -> NutritionBreakdown {
    NutritionBreakdown {
        calories:  uc::round_to(a.calories  + b.calories, 1),
        protein_g: uc::round_to(a.protein_g + b.protein_g, 1),
        fat_g:     uc::round_to(a.fat_g     + b.fat_g, 1),
        carbs_g:   uc::round_to(a.carbs_g   + b.carbs_g, 1),
        fiber_g:   uc::round_to(a.fiber_g   + b.fiber_g, 1),
        sugar_g:   uc::round_to(a.sugar_g   + b.sugar_g, 1),
        salt_g:    uc::round_to(a.salt_g    + b.salt_g, 2),
        sodium_mg: uc::round_to(a.sodium_mg + b.sodium_mg, 1),
    }
}

fn compute_cost(ingredients: &[RecipeIngredientInput], portions: u32) -> Option<CostBreakdown> {
    let mut total_cost = 0.0;
    let mut all_have_cost = true;
    let mut items = Vec::new();

    for ing in ingredients {
        if let Some(cost_per_kg) = ing.cost_per_kg {
            let cost = uc::round_to(cost_per_kg * ing.grams / 1000.0, 2);
            total_cost += cost;
            items.push(IngredientCost {
                slug: ing.slug.clone(),
                grams: ing.grams,
                cost,
            });
        } else {
            all_have_cost = false;
        }
    }

    if items.is_empty() {
        return None;
    }

    Some(CostBreakdown {
        total_cost: uc::round_to(total_cost, 2),
        cost_per_portion: uc::round_to(total_cost / portions.max(1) as f64, 2),
        ingredients: items,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tomato() -> RecipeIngredientInput {
        RecipeIngredientInput {
            slug: "tomato".to_string(),
            grams: 200.0,
            nutrition_100g: NutritionBreakdown {
                calories: 18.0, protein_g: 0.9, fat_g: 0.2, carbs_g: 3.9,
                fiber_g: 1.2, sugar_g: 2.6, salt_g: 0.01, sodium_mg: 3.9,
            },
            flavor: FlavorVector {
                sweetness: 4.0, acidity: 6.0, bitterness: 1.0,
                umami: 7.0, fat: 0.2, aroma: 5.0,
            },
            cost_per_kg: Some(4.50),
            diet_flags: DietFlags {
                vegan: true, vegetarian: true, keto: true, paleo: true,
                gluten_free: true, mediterranean: true, low_carb: true,
            },
        }
    }

    fn sample_pasta() -> RecipeIngredientInput {
        RecipeIngredientInput {
            slug: "pasta".to_string(),
            grams: 250.0,
            nutrition_100g: NutritionBreakdown {
                calories: 371.0, protein_g: 13.0, fat_g: 1.5, carbs_g: 74.0,
                fiber_g: 3.2, sugar_g: 2.0, salt_g: 0.01, sodium_mg: 3.9,
            },
            flavor: FlavorVector {
                sweetness: 2.0, acidity: 0.5, bitterness: 0.5,
                umami: 1.0, fat: 1.5, aroma: 1.0,
            },
            cost_per_kg: Some(3.20),
            diet_flags: DietFlags {
                vegan: true, vegetarian: true, keto: false, paleo: false,
                gluten_free: false, mediterranean: true, low_carb: false,
            },
        }
    }

    #[test]
    fn recipe_analysis_basic() {
        let recipe = vec![sample_tomato(), sample_pasta()];
        let result = analyze_recipe(&recipe, 4);

        // Total weight
        assert_eq!(result.total_weight_g, 450.0);
        assert_eq!(result.portions, 4);

        // Per-portion calories should be roughly (36 + 927.5) / 4 ≈ 240
        assert!(result.per_portion.calories > 200.0, "per portion cal > 200, got {}", result.per_portion.calories);
        assert!(result.per_portion.calories < 300.0, "per portion cal < 300, got {}", result.per_portion.calories);

        // Nutrition score
        assert!(result.nutrition_score > 0);
    }

    #[test]
    fn diet_flags_intersect_correctly() {
        let recipe = vec![sample_tomato(), sample_pasta()];
        let result = analyze_recipe(&recipe, 2);

        assert!(result.diet_flags.vegetarian, "both are vegetarian");
        assert!(!result.diet_flags.keto, "pasta is not keto");
        assert!(!result.diet_flags.gluten_free, "pasta has gluten");
        assert!(result.diet_flags.mediterranean, "both are mediterranean");
    }

    #[test]
    fn cost_calculated() {
        let recipe = vec![sample_tomato(), sample_pasta()];
        let result = analyze_recipe(&recipe, 4);
        let cost = result.cost.unwrap();

        // Tomato: 4.50 × 0.2 = 0.90, Pasta: 3.20 × 0.25 = 0.80
        assert!(cost.total_cost > 1.5, "total cost > 1.5, got {}", cost.total_cost);
        assert!(cost.total_cost < 2.0, "total cost < 2.0, got {}", cost.total_cost);
        assert_eq!(cost.ingredients.len(), 2);
    }
}
