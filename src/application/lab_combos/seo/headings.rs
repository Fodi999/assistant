// ─── SEO Headings — H1 + Intro paragraph ────────────────────────────────────

use super::helpers::capitalize_words;
use crate::application::lab_combos::nutrition::NutritionTotals;

/// Auto-generate H1 heading. Recipe-style, no "analysis" words.
pub fn generate_h1(
    ingredients: &[String],
    goal: Option<&str>,
    meal_type: Option<&str>,
    locale: &str,
) -> String {
    let names = capitalize_words(&ingredients.join(" "));
    let meal = meal_type.map(|m| capitalize_words(&m.replace('_', " ")));
    let goal_hint = goal.map(|g| capitalize_words(&g.replace('_', " ")));

    match locale {
        "ru" => {
            if let Some(m) = &meal {
                format!("{names} — рецепт на {m}")
            } else if let Some(g) = &goal_hint {
                format!("{names} — рецепт ({g})")
            } else {
                format!("{names} — быстрый рецепт")
            }
        }
        "pl" => {
            if let Some(m) = &meal {
                format!("{names} — przepis na {m}")
            } else if let Some(g) = &goal_hint {
                format!("{names} — przepis ({g})")
            } else {
                format!("{names} — szybki przepis")
            }
        }
        "uk" => {
            if let Some(m) = &meal {
                format!("{names} — рецепт на {m}")
            } else if let Some(g) = &goal_hint {
                format!("{names} — рецепт ({g})")
            } else {
                format!("{names} — швидкий рецепт")
            }
        }
        _ => {
            if let Some(m) = &meal {
                format!("{names} Bowl — Healthy {m} Recipe")
            } else if let Some(g) = &goal_hint {
                format!("{names} Recipe — {g}")
            } else {
                format!("{names} — Quick & Easy Recipe")
            }
        }
    }
}

/// Intro paragraph — first sentence = direct answer with REAL nutrition.
/// Targets Google featured snippets.
pub fn generate_intro(
    ingredients: &[String],
    _goal: Option<&str>,
    locale: &str,
    nt: &NutritionTotals,
) -> String {
    let names = ingredients.join(", ");
    let est_protein = nt.protein_per_serving.round() as i64;
    let est_calories = nt.calories_per_serving.round() as i64;

    let protein_text = if est_protein > 5 {
        match locale {
            "ru" => format!(" ~{}г белка,", est_protein),
            "pl" => format!(" ~{}g białka,", est_protein),
            "uk" => format!(" ~{}г білка,", est_protein),
            _ => format!(" ~{}g protein,", est_protein),
        }
    } else {
        String::new()
    };

    match locale {
        "ru" => format!(
            "Это блюдо из {names} содержит{protein_text} ~{est_calories} ккал на порцию и готовится за 15–20 минут. \
             Ниже — пошаговый рецепт с точными граммовками и КБЖУ на порцию.",
        ),
        "pl" => format!(
            "To danie z {names} zawiera{protein_text} ~{est_calories} kcal na porcję i przygotujesz je w 15–20 minut. \
             Poniżej — przepis krok po kroku z dokładnymi gramówkami i KBJU na porcję.",
        ),
        "uk" => format!(
            "Ця страва з {names} містить{protein_text} ~{est_calories} ккал на порцію і готується за 15–20 хвилин. \
             Нижче — покроковий рецепт із точними грамовками та КБЖУ на порцію.",
        ),
        _ => format!(
            "This {names} dish delivers{protein_text} ~{est_calories} kcal per serving and is ready in 15–20 minutes. \
             Below: step-by-step recipe with exact portions and macros per serving.",
        ),
    }
}
