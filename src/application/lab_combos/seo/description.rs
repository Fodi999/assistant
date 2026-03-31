// ─── SEO Description — 80-155 char meta description ─────────────────────────

use super::helpers::smart_truncate;
use crate::application::lab_combos::nutrition::NutritionTotals;

/// Auto-generate SEO description (80–155 chars).
pub fn generate_description(
    ingredients: &[String],
    goal: Option<&str>,
    locale: &str,
    nt: &NutritionTotals,
) -> String {
    let names = ingredients.join(", ");
    let est_protein = nt.protein_per_serving.round() as i64;
    let goal_text = goal
        .map(|g| format!(" for {}", g.replace('_', " ")))
        .unwrap_or_default();

    let desc = match locale {
        "ru" => format!(
            "Рецепт из {names}{goal_text} за 15 мин.{} Пошаговая инструкция и КБЖУ.",
            if est_protein > 5 {
                format!(" {}г белка на порцию.", est_protein)
            } else {
                String::new()
            }
        ),
        "pl" => format!(
            "Przepis z {names}{goal_text} w 15 min.{} Instrukcja krok po kroku i KBJU.",
            if est_protein > 5 {
                format!(" {}g białka na porcję.", est_protein)
            } else {
                String::new()
            }
        ),
        "uk" => format!(
            "Рецепт з {names}{goal_text} за 15 хв.{} Покрокова інструкція та КБЖУ.",
            if est_protein > 5 {
                format!(" {}г білка на порцію.", est_protein)
            } else {
                String::new()
            }
        ),
        _ => {
            let protein_hint = if est_protein > 5 {
                format!(" {}g protein per serving.", est_protein)
            } else {
                String::new()
            };
            format!(
                "Quick recipe with {names}{goal_text}.{protein_hint} Step-by-step instructions and macros."
            )
        }
    };

    smart_truncate(&desc, 155)
}
