//! Flavor Graph Engine — domain layer
//!
//! Pure functions for flavor vector operations:
//! - Aggregate culinary properties across recipe ingredients
//! - Compute flavor balance score
//! - Identify weak/strong dimensions for suggestions
//!
//! No DB, no HTTP — only data structures and math.

use serde::{Deserialize, Serialize};
use crate::domain::tools::unit_converter as uc;

// ── Flavor Vector ────────────────────────────────────────────────────────────

/// 6-dimensional flavor vector representing an ingredient or a recipe.
/// All values are 0.0–10.0 scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlavorVector {
    pub sweetness:  f64,
    pub acidity:    f64,
    pub bitterness: f64,
    pub umami:      f64,
    pub fat:        f64,   // richness / fattiness
    pub aroma:      f64,
}

impl FlavorVector {
    pub fn zero() -> Self {
        Self { sweetness: 0.0, acidity: 0.0, bitterness: 0.0, umami: 0.0, fat: 0.0, aroma: 0.0 }
    }

    /// Weighted add: accumulate another vector scaled by weight (grams / total grams)
    pub fn add_weighted(&mut self, other: &FlavorVector, weight: f64) {
        self.sweetness  += other.sweetness  * weight;
        self.acidity    += other.acidity    * weight;
        self.bitterness += other.bitterness * weight;
        self.umami      += other.umami      * weight;
        self.fat        += other.fat        * weight;
        self.aroma      += other.aroma      * weight;
    }

    /// Round all values to 2 decimal places
    pub fn round(&mut self) {
        self.sweetness  = uc::round_to(self.sweetness, 2);
        self.acidity    = uc::round_to(self.acidity, 2);
        self.bitterness = uc::round_to(self.bitterness, 2);
        self.umami      = uc::round_to(self.umami, 2);
        self.fat        = uc::round_to(self.fat, 2);
        self.aroma      = uc::round_to(self.aroma, 2);
    }

    /// Return all dimensions as (name, value) pairs
    pub fn dimensions(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("sweetness",  self.sweetness),
            ("acidity",    self.acidity),
            ("bitterness", self.bitterness),
            ("umami",      self.umami),
            ("fat",        self.fat),
            ("aroma",      self.aroma),
        ]
    }

    /// Mean value across all dimensions
    pub fn mean(&self) -> f64 {
        let dims = self.dimensions();
        let sum: f64 = dims.iter().map(|(_, v)| v).sum();
        sum / dims.len() as f64
    }

    /// Standard deviation — measures how unbalanced the flavor is
    pub fn std_dev(&self) -> f64 {
        let m = self.mean();
        let dims = self.dimensions();
        let variance: f64 = dims.iter().map(|(_, v)| (v - m).powi(2)).sum::<f64>() / dims.len() as f64;
        variance.sqrt()
    }
}

// ── Ingredient entry for aggregation ─────────────────────────────────────────

/// One ingredient in a recipe with its weight and flavor profile.
#[derive(Debug, Clone)]
pub struct FlavorIngredient {
    pub slug:    String,
    pub grams:   f64,
    pub flavor:  FlavorVector,
}

// ── Flavor Balance ───────────────────────────────────────────────────────────

/// Result of analyzing a recipe's overall flavor profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlavorBalance {
    /// Aggregated flavor vector (weighted by ingredient mass)
    pub vector: FlavorVector,
    /// 0–100 score: 100 = perfectly balanced, 0 = one dimension dominates
    pub balance_score: u8,
    /// Weakest dimensions (candidates to boost)
    pub weak_dimensions: Vec<DimensionGap>,
    /// Strongest dimensions (already saturated)
    pub strong_dimensions: Vec<DimensionGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionGap {
    pub dimension: String,
    pub value:     f64,
    /// How far from the mean (negative = below, positive = above)
    pub deviation: f64,
}

// ── Pure functions ───────────────────────────────────────────────────────────

/// Aggregate flavor vectors from recipe ingredients, weighted by mass.
pub fn aggregate_flavors(ingredients: &[FlavorIngredient]) -> FlavorVector {
    let total_grams: f64 = ingredients.iter().map(|i| i.grams).sum();
    if total_grams <= 0.0 {
        return FlavorVector::zero();
    }

    let mut result = FlavorVector::zero();
    for ing in ingredients {
        let weight = ing.grams / total_grams;
        result.add_weighted(&ing.flavor, weight);
    }
    result.round();
    result
}

/// Compute flavor balance for a recipe.
pub fn analyze_balance(ingredients: &[FlavorIngredient]) -> FlavorBalance {
    let vector = aggregate_flavors(ingredients);
    let mean = vector.mean();
    let std_dev = vector.std_dev();

    // Balance score: lower std_dev → higher score
    // Perfect balance (std_dev=0) → 100, heavy imbalance (std_dev≥4) → 0
    let balance_raw = (1.0 - std_dev / 4.0).clamp(0.0, 1.0) * 100.0;
    let balance_score = balance_raw.round() as u8;

    let mut weak_dimensions = Vec::new();
    let mut strong_dimensions = Vec::new();

    for (name, val) in vector.dimensions() {
        let deviation = uc::round_to(val - mean, 2);
        let gap = DimensionGap {
            dimension: name.to_string(),
            value: uc::round_to(val, 2),
            deviation,
        };
        if deviation < -0.5 {
            weak_dimensions.push(gap);
        } else if deviation > 0.5 {
            strong_dimensions.push(gap);
        }
    }

    // Sort: weakest first, strongest first
    weak_dimensions.sort_by(|a, b| a.deviation.partial_cmp(&b.deviation).unwrap_or(std::cmp::Ordering::Equal));
    strong_dimensions.sort_by(|a, b| b.deviation.partial_cmp(&a.deviation).unwrap_or(std::cmp::Ordering::Equal));

    FlavorBalance {
        vector,
        balance_score,
        weak_dimensions,
        strong_dimensions,
    }
}

/// Build a FlavorVector from DB culinary properties row.
/// Converts DB fields (sweetness, acidity, bitterness, umami, aroma) +
/// derives fat dimension from fat_per_100g.
pub fn flavor_from_culinary(
    sweetness: f64,
    acidity: f64,
    bitterness: f64,
    umami: f64,
    aroma: f64,
    fat_per_100g: f64,
) -> FlavorVector {
    // Fat → 0–10 scale: 0g=0, 50g=5, 100g=10
    let fat_score = (fat_per_100g / 10.0).clamp(0.0, 10.0);

    FlavorVector {
        sweetness,
        acidity,
        bitterness,
        umami,
        fat: uc::round_to(fat_score, 2),
        aroma,
    }
}

// ── Pairing compatibility ────────────────────────────────────────────────────

/// Calculate how well two flavor vectors complement each other.
/// Returns 0.0–1.0 where 1.0 = perfect complement (one fills the other's gaps).
pub fn flavor_compatibility(a: &FlavorVector, b: &FlavorVector) -> f64 {
    // Complementarity: if A is low on umami but B is high, they complement
    let dims_a = a.dimensions();
    let dims_b = b.dimensions();
    let mean_a = a.mean();

    let mut complement_score = 0.0;
    let count = dims_a.len() as f64;

    for ((_, va), (_, vb)) in dims_a.iter().zip(dims_b.iter()) {
        let gap = mean_a - va; // how much A lacks
        let fill = if gap > 0.0 {
            // A is weak here — does B fill the gap?
            (vb / 10.0).min(1.0) * gap.min(5.0) / 5.0
        } else {
            // A is strong here — B should not over-saturate
            let over = vb / 10.0;
            1.0 - over.min(1.0) * 0.3
        };
        complement_score += fill;
    }

    uc::round_to((complement_score / count).clamp(0.0, 1.0), 3)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tomato() -> FlavorIngredient {
        FlavorIngredient {
            slug: "tomato".to_string(),
            grams: 200.0,
            flavor: FlavorVector {
                sweetness: 4.0, acidity: 6.0, bitterness: 1.0,
                umami: 7.0, fat: 0.2, aroma: 5.0,
            },
        }
    }

    fn olive_oil() -> FlavorIngredient {
        FlavorIngredient {
            slug: "olive-oil".to_string(),
            grams: 30.0,
            flavor: FlavorVector {
                sweetness: 0.5, acidity: 1.0, bitterness: 3.0,
                umami: 0.0, fat: 10.0, aroma: 6.0,
            },
        }
    }

    fn garlic() -> FlavorIngredient {
        FlavorIngredient {
            slug: "garlic".to_string(),
            grams: 10.0,
            flavor: FlavorVector {
                sweetness: 1.0, acidity: 0.5, bitterness: 2.0,
                umami: 4.0, fat: 0.1, aroma: 9.0,
            },
        }
    }

    #[test]
    fn aggregate_weighted_by_mass() {
        let ingredients = vec![tomato(), olive_oil(), garlic()];
        let result = aggregate_flavors(&ingredients);
        // Tomato dominates (200g of 240g total = 83%)
        assert!(result.sweetness > 3.0, "sweetness should be close to tomato's 4.0");
        assert!(result.umami > 5.0, "umami should be high from tomato");
        assert!(result.fat < 2.0, "fat should be low (mostly tomato)");
    }

    #[test]
    fn balance_score_reasonable() {
        let ingredients = vec![tomato(), olive_oil(), garlic()];
        let balance = analyze_balance(&ingredients);
        assert!(balance.balance_score > 30, "balance should be >30, got {}", balance.balance_score);
        assert!(balance.balance_score < 90, "balance should be <90, got {}", balance.balance_score);
    }

    #[test]
    fn weak_dimensions_detected() {
        let ingredients = vec![tomato()]; // pure tomato: low fat, low bitterness
        let balance = analyze_balance(&ingredients);
        let weak_names: Vec<&str> = balance.weak_dimensions.iter().map(|d| d.dimension.as_str()).collect();
        assert!(weak_names.contains(&"fat"), "fat should be weak for tomato, weak={:?}", weak_names);
    }

    #[test]
    fn empty_recipe_is_zero() {
        let result = aggregate_flavors(&[]);
        assert_eq!(result.sweetness, 0.0);
        assert_eq!(result.umami, 0.0);
    }

    #[test]
    fn compatibility_complement() {
        let tomato_v = FlavorVector {
            sweetness: 4.0, acidity: 6.0, bitterness: 1.0,
            umami: 7.0, fat: 0.2, aroma: 5.0,
        };
        let cream = FlavorVector {
            sweetness: 3.0, acidity: 1.0, bitterness: 0.0,
            umami: 1.0, fat: 9.0, aroma: 2.0,
        };
        let score = flavor_compatibility(&tomato_v, &cream);
        assert!(score > 0.3, "tomato + cream should complement (fat fills gap), got {}", score);
    }
}
