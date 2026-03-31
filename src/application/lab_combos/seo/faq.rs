// ─── SEO FAQ — structured FAQ schema generation ─────────────────────────────

use crate::application::lab_combos::nutrition::NutritionTotals;

/// Auto-generate FAQ from SmartResponse data.
pub fn generate_faq(
    ingredients: &[String],
    smart: &serde_json::Value,
    locale: &str,
    nt: &NutritionTotals,
) -> serde_json::Value {
    let names = ingredients.join(", ");
    let mut faq = Vec::new();

    // Q1: Calories/protein
    {
        let serving_kcal = nt.calories_per_serving.round() as i64;
        let serving_prot = nt.protein_per_serving.round() as i64;
        let serving_weight = nt.total_weight_g.round() as i64;
        let q = match locale {
            "ru" => format!("Сколько калорий и белка в блюде из {}?", names),
            "pl" => format!("Ile kalorii i białka ma danie z {}?", names),
            "uk" => format!("Скільки калорій і білка у страві з {}?", names),
            _ => format!("How many calories and protein in a {} dish?", names),
        };
        let a = match locale {
            "ru" => format!(
                "Примерно {} ккал и {} г белка на порцию (~{} г).",
                serving_kcal, serving_prot, serving_weight
            ),
            "pl" => format!(
                "Około {} kcal i {} g białka na porcję (~{} g).",
                serving_kcal, serving_prot, serving_weight
            ),
            "uk" => format!(
                "Приблизно {} ккал і {} г білка на порцію (~{} г).",
                serving_kcal, serving_prot, serving_weight
            ),
            _ => format!(
                "Approximately {} kcal and {}g protein per serving (~{}g).",
                serving_kcal, serving_prot, serving_weight
            ),
        };
        faq.push(serde_json::json!({ "question": q, "answer": a }));
    }

    // Q2: Cooking time
    {
        let q = match locale {
            "ru" => format!("Сколько времени готовить {}?", names),
            "pl" => format!("Ile czasu zajmuje przygotowanie {}?", names),
            "uk" => format!("Скільки часу готувати {}?", names),
            _ => format!("How long does it take to cook {}?", names),
        };
        let a = match locale {
            "ru" => "Активное время — 15–20 минут. Полное время с подготовкой — около 25 минут."
                .to_string(),
            "pl" => "Czas aktywny — 15–20 minut. Pełny czas z przygotowaniem — około 25 minut."
                .to_string(),
            "uk" => "Активний час — 15–20 хвилин. Повний час із підготовкою — близько 25 хвилин."
                .to_string(),
            _ => "Active time: 15–20 minutes. Total time including prep: about 25 minutes."
                .to_string(),
        };
        faq.push(serde_json::json!({ "question": q, "answer": a }));
    }

    // Q3: Substitutions
    let suggestions = smart.get("suggestions").and_then(|s| s.as_array());
    if let Some(sugg) = suggestions {
        let top: Vec<String> = sugg
            .iter()
            .take(3)
            .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect();
        if !top.is_empty() {
            let q = match locale {
                "ru" => "Чем можно заменить ингредиенты в рецепте?".to_string(),
                "pl" => "Czym można zastąpić składniki w przepisie?".to_string(),
                "uk" => "Чим можна замінити інгредієнти в рецепті?".to_string(),
                _ => "What substitutions work in this recipe?".to_string(),
            };
            let a = match locale {
                "ru" => format!("Попробуйте добавить или заменить на: {}.", top.join(", ")),
                "pl" => format!("Spróbuj dodać lub zamienić na: {}.", top.join(", ")),
                "uk" => format!("Спробуйте додати або замінити на: {}.", top.join(", ")),
                _ => format!("Try adding or swapping with: {}.", top.join(", ")),
            };
            faq.push(serde_json::json!({ "question": q, "answer": a }));
        }
    }

    // Q4: Dish variants
    let variants = smart.get("variants").and_then(|v| v.as_array());
    if let Some(vars) = variants {
        if !vars.is_empty() {
            let q = match locale {
                "ru" => format!("Какие блюда можно приготовить из {}?", names),
                "pl" => format!("Jakie dania można zrobić z {}?", names),
                "uk" => format!("Які страви можна приготувати з {}?", names),
                _ => format!("What dishes can I make with {}?", names),
            };
            let variant_names: Vec<String> = vars
                .iter()
                .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect();
            let a = if variant_names.is_empty() {
                match locale {
                    "ru" => format!("{} вариантов блюд — от боула до салата.", vars.len()),
                    "pl" => format!("{} wariantów dań — od bowla po sałatkę.", vars.len()),
                    "uk" => format!("{} варіантів страв — від боулу до салату.", vars.len()),
                    _ => format!("{} dish variants — from bowl to salad.", vars.len()),
                }
            } else {
                variant_names.join("; ")
            };
            faq.push(serde_json::json!({ "question": q, "answer": a }));
        }
    }

    serde_json::json!(faq)
}
