// ─── SEO Title — ≤60 char title generation ──────────────────────────────────

use super::helpers::{capitalize_words, smart_truncate};
use crate::application::lab_combos::nutrition::NutritionTotals;

/// Auto-generate SEO title from ingredients + context (≤ 60 chars).
/// Format: "Salmon Rice Bowl (34g Protein, 15 Min)"
pub fn generate_title(
    ingredients: &[String],
    goal: Option<&str>,
    meal_type: Option<&str>,
    _locale: &str,
    nt: &NutritionTotals,
) -> String {
    let names = capitalize_words(&ingredients.join(" "));
    let meal = meal_type
        .map(|m| capitalize_words(&m.replace('_', " ")))
        .unwrap_or_default();

    let dish = if meal.is_empty() {
        names.clone()
    } else {
        format!("{} {} Bowl", names, meal)
    };

    let est_protein = nt.protein_per_serving.round() as i64;

    let hook = if est_protein > 5 {
        match goal {
            Some(g) if g.contains("loss") || g.contains("low_cal") => {
                format!("({}g Protein, Low Cal)", est_protein)
            }
            Some(g) if g.contains("keto") => format!("({}g Protein, Keto)", est_protein),
            _ => format!("({}g Protein, 15 Min)", est_protein),
        }
    } else {
        "(15 Min)".to_string()
    };

    smart_truncate(&format!("{} {}", dish, hook), 60)
}
