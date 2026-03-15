//! Suggestion Engine — domain layer
//!
//! Rule-based + graph-traversal logic to recommend ingredients
//! that improve a recipe's flavor balance, nutrition, or both.
//!
//! No DB, no HTTP — pure functions. The application layer fetches
//! candidates from DB and passes them here for scoring.

use serde::{Deserialize, Serialize};
use crate::domain::tools::unit_converter as uc;
use crate::domain::tools::flavor_graph::{FlavorVector, FlavorBalance};
use crate::domain::tools::nutrition::NutritionBreakdown;

// ── Input: candidate ingredient ──────────────────────────────────────────────

/// A candidate ingredient that could be added to the recipe.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub slug:        String,
    pub name:        String,
    pub image_url:   Option<String>,
    /// Culinary flavor vector
    pub flavor:      FlavorVector,
    /// Nutrition per 100g
    pub nutrition:   NutritionBreakdown,
    /// Pairing score with existing recipe ingredients (avg)
    pub pair_score:  f64,
    /// Typical portion to add (grams)
    pub typical_g:   f64,
}

// ── Output: suggestion ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub slug:           String,
    pub name:           String,
    pub image_url:      Option<String>,
    /// 0–100 overall recommendation score
    pub score:          u8,
    /// Why this ingredient is suggested
    pub reasons:        Vec<String>,
    /// Which weak dimensions it addresses
    pub fills_gaps:     Vec<String>,
    /// Suggested amount in grams
    pub suggested_grams: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResult {
    /// Current recipe balance (before suggestions)
    pub current_balance: FlavorBalance,
    /// Top suggestions ordered by score
    pub suggestions:     Vec<Suggestion>,
}

// ── Scoring weights ──────────────────────────────────────────────────────────

/// How much each factor contributes to the final suggestion score.
struct Weights {
    /// How well the candidate fills flavor gaps (0–40 points)
    flavor_gap_fill: f64,
    /// Pairing score with existing ingredients (0–30 points)
    pairing:         f64,
    /// Nutritional improvement potential (0–20 points)
    nutrition:       f64,
    /// Aromatic contribution (0–10 points)
    aroma:           f64,
}

const WEIGHTS: Weights = Weights {
    flavor_gap_fill: 40.0,
    pairing:         30.0,
    nutrition:       20.0,
    aroma:           10.0,
};

// ── Core suggestion function ─────────────────────────────────────────────────

/// Score and rank candidate ingredients for a recipe.
///
/// Arguments:
/// - `balance`: current recipe's flavor analysis
/// - `candidates`: potential ingredients to add
/// - `existing_slugs`: slugs already in the recipe (to exclude)
/// - `max_results`: how many suggestions to return
pub fn suggest_ingredients(
    balance: &FlavorBalance,
    candidates: &[Candidate],
    existing_slugs: &[String],
    max_results: usize,
) -> SuggestionResult {
    let mut suggestions: Vec<Suggestion> = candidates
        .iter()
        .filter(|c| !existing_slugs.contains(&c.slug))
        .map(|c| score_candidate(balance, c))
        .filter(|s| s.score > 10) // minimum threshold
        .collect();

    // Sort by score descending
    suggestions.sort_by(|a, b| b.score.cmp(&a.score));
    suggestions.truncate(max_results);

    SuggestionResult {
        current_balance: balance.clone(),
        suggestions,
    }
}

/// Score a single candidate against the current recipe balance.
fn score_candidate(balance: &FlavorBalance, candidate: &Candidate) -> Suggestion {
    let mut reasons = Vec::new();
    let mut fills_gaps = Vec::new();
    let mut score = 0.0;

    // ── 1. Flavor gap filling (up to 40 points) ──
    let gap_score = flavor_gap_score(balance, &candidate.flavor, &mut fills_gaps);
    score += gap_score * WEIGHTS.flavor_gap_fill;
    if !fills_gaps.is_empty() {
        reasons.push(format!("fills flavor gap: {}", fills_gaps.join(", ")));
    }

    // ── 2. Pairing affinity (up to 30 points) ──
    let pair_norm = (candidate.pair_score / 10.0).clamp(0.0, 1.0);
    score += pair_norm * WEIGHTS.pairing;
    if candidate.pair_score > 6.0 {
        reasons.push(format!("strong pairing affinity ({:.1})", candidate.pair_score));
    }

    // ── 3. Nutritional value (up to 20 points) ──
    let nut_score = nutrition_bonus(&candidate.nutrition);
    score += nut_score * WEIGHTS.nutrition;
    if nut_score > 0.5 {
        reasons.push("adds nutritional value".to_string());
    }

    // ── 4. Aroma contribution (up to 10 points) ──
    let aroma_norm = (candidate.flavor.aroma / 10.0).clamp(0.0, 1.0);
    score += aroma_norm * WEIGHTS.aroma;
    if candidate.flavor.aroma > 6.0 {
        reasons.push("aromatic boost".to_string());
    }

    if reasons.is_empty() {
        reasons.push("general complement".to_string());
    }

    Suggestion {
        slug: candidate.slug.clone(),
        name: candidate.name.clone(),
        image_url: candidate.image_url.clone(),
        score: score.clamp(0.0, 100.0).round() as u8,
        reasons,
        fills_gaps,
        suggested_grams: uc::round_to(candidate.typical_g, 0),
    }
}

/// How well does this candidate fill the recipe's weak dimensions?
/// Returns 0.0–1.0
fn flavor_gap_score(
    balance: &FlavorBalance,
    candidate_flavor: &FlavorVector,
    fills: &mut Vec<String>,
) -> f64 {
    if balance.weak_dimensions.is_empty() {
        return 0.2; // no gaps → small baseline
    }

    let candidate_dims = candidate_flavor.dimensions();
    let mut total_fill = 0.0;
    let max_possible = balance.weak_dimensions.len() as f64;

    for gap in &balance.weak_dimensions {
        // Find candidate's value for this dimension
        let candidate_val = candidate_dims
            .iter()
            .find(|(name, _)| *name == gap.dimension.as_str())
            .map(|(_, v)| *v)
            .unwrap_or(0.0);

        // If candidate is strong where recipe is weak → good fill
        if candidate_val > 3.0 {
            let fill_ratio = (candidate_val / 10.0).min(1.0);
            total_fill += fill_ratio;
            fills.push(gap.dimension.clone());
        }
    }

    (total_fill / max_possible.max(1.0)).clamp(0.0, 1.0)
}

/// Nutritional bonus: reward protein-rich, fiber-rich, low-sugar ingredients.
/// Returns 0.0–1.0
fn nutrition_bonus(n: &NutritionBreakdown) -> f64 {
    let protein_score = (n.protein_g / 25.0).min(1.0) * 0.4;
    let fiber_score   = (n.fiber_g / 10.0).min(1.0) * 0.3;
    let sugar_penalty = (n.sugar_g / 20.0).min(1.0) * 0.2;
    let cal_penalty   = if n.calories > 500.0 { 0.1 } else { 0.0 };

    (protein_score + fiber_score - sugar_penalty - cal_penalty).clamp(0.0, 1.0)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tools::flavor_graph::{self, FlavorIngredient, DimensionGap};

    fn tomato_recipe_balance() -> FlavorBalance {
        // Simulated: tomato-heavy recipe — weak on fat, strong on umami
        FlavorBalance {
            vector: FlavorVector {
                sweetness: 4.0, acidity: 6.0, bitterness: 1.0,
                umami: 7.0, fat: 0.2, aroma: 5.0,
            },
            balance_score: 55,
            weak_dimensions: vec![
                DimensionGap { dimension: "fat".to_string(), value: 0.2, deviation: -3.63 },
                DimensionGap { dimension: "bitterness".to_string(), value: 1.0, deviation: -2.83 },
            ],
            strong_dimensions: vec![
                DimensionGap { dimension: "umami".to_string(), value: 7.0, deviation: 3.17 },
            ],
        }
    }

    fn olive_oil_candidate() -> Candidate {
        Candidate {
            slug: "olive-oil".to_string(),
            name: "Olive Oil".to_string(),
            image_url: None,
            flavor: FlavorVector {
                sweetness: 0.5, acidity: 1.0, bitterness: 3.0,
                umami: 0.0, fat: 10.0, aroma: 6.0,
            },
            nutrition: NutritionBreakdown {
                calories: 884.0, protein_g: 0.0, fat_g: 100.0, carbs_g: 0.0,
                fiber_g: 0.0, sugar_g: 0.0, salt_g: 0.0, sodium_mg: 0.0,
            },
            pair_score: 8.5,
            typical_g: 15.0,
        }
    }

    fn sugar_candidate() -> Candidate {
        Candidate {
            slug: "sugar".to_string(),
            name: "Sugar".to_string(),
            image_url: None,
            flavor: FlavorVector {
                sweetness: 10.0, acidity: 0.0, bitterness: 0.0,
                umami: 0.0, fat: 0.0, aroma: 0.0,
            },
            nutrition: NutritionBreakdown {
                calories: 387.0, protein_g: 0.0, fat_g: 0.0, carbs_g: 100.0,
                fiber_g: 0.0, sugar_g: 100.0, salt_g: 0.0, sodium_mg: 0.0,
            },
            pair_score: 2.0,
            typical_g: 5.0,
        }
    }

    #[test]
    fn olive_oil_fills_fat_gap() {
        let balance = tomato_recipe_balance();
        let result = suggest_ingredients(
            &balance,
            &[olive_oil_candidate(), sugar_candidate()],
            &[],
            5,
        );

        assert!(!result.suggestions.is_empty(), "should have suggestions");
        let top = &result.suggestions[0];
        assert_eq!(top.slug, "olive-oil", "olive oil should be #1 for fat gap");
        assert!(top.fills_gaps.contains(&"fat".to_string()), "should fill fat gap");
        assert!(top.score > 40, "olive oil score should be > 40, got {}", top.score);
    }

    #[test]
    fn excludes_existing_ingredients() {
        let balance = tomato_recipe_balance();
        let result = suggest_ingredients(
            &balance,
            &[olive_oil_candidate()],
            &["olive-oil".to_string()],
            5,
        );
        assert!(result.suggestions.is_empty(), "should exclude olive-oil");
    }

    #[test]
    fn sugar_scores_lower_than_olive_oil() {
        let balance = tomato_recipe_balance();
        let result = suggest_ingredients(
            &balance,
            &[olive_oil_candidate(), sugar_candidate()],
            &[],
            5,
        );
        let scores: Vec<(&str, u8)> = result.suggestions.iter().map(|s| (s.slug.as_str(), s.score)).collect();
        let oil_score = scores.iter().find(|(s, _)| *s == "olive-oil").map(|(_, sc)| *sc).unwrap_or(0);
        let sugar_score = scores.iter().find(|(s, _)| *s == "sugar").map(|(_, sc)| *sc).unwrap_or(0);
        assert!(oil_score > sugar_score, "olive oil ({}) should score higher than sugar ({})", oil_score, sugar_score);
    }
}
