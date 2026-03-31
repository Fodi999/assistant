// ─── SEO Content — why_it_works, how_to_cook, optimization_tips ─────────────

use super::helpers::capitalize_words;
use crate::application::lab_combos::nutrition::NutritionTotals;

/// Generate "why this combo works" from SmartResponse + nutrition data.
pub fn generate_why_it_works(
    ingredients: &[String],
    smart: &serde_json::Value,
    goal: Option<&str>,
    locale: &str,
    nt: &NutritionTotals,
) -> String {
    let names = ingredients.join(", ");
    let protein = nt.protein_per_serving;
    let calories = nt.calories_per_serving;
    let fiber = nt.fiber_per_serving;

    let balance_score = smart
        .get("flavor_profile")
        .and_then(|f| f.get("balance"))
        .and_then(|b| b.get("score"))
        .and_then(|v| v.as_f64());

    let dominant_tastes: Vec<String> = smart
        .get("flavor_profile")
        .and_then(|f| f.get("balance"))
        .and_then(|b| b.get("dominant_tastes"))
        .and_then(|dt| dt.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let variant_types: Vec<String> = smart
        .get("variants")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.get("variant_type")
                        .and_then(|t| t.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();

    let mut parts: Vec<String> = Vec::new();

    // Part 1: Nutritional reason
    match locale {
        "ru" => {
            if protein > 15.0 {
                parts.push(format!(
                    "Эта комбинация содержит {:.0} г белка на порцию — отличный источник протеина",
                    protein
                ));
            } else {
                parts.push(format!(
                    "Комбинация {names} даёт {:.0} ккал на порцию",
                    calories
                ));
            }
        }
        "pl" => {
            if protein > 15.0 {
                parts.push(format!(
                    "Ta kombinacja zawiera {:.0} g białka na porcję — świetne źródło proteiny",
                    protein
                ));
            } else {
                parts.push(format!(
                    "Kombinacja {names} dostarcza {:.0} kcal na porcję",
                    calories
                ));
            }
        }
        "uk" => {
            if protein > 15.0 {
                parts.push(format!(
                    "Ця комбінація містить {:.0} г білка на порцію — чудове джерело протеїну",
                    protein
                ));
            } else {
                parts.push(format!(
                    "Комбінація {names} дає {:.0} ккал на порцію",
                    calories
                ));
            }
        }
        _ => {
            if protein > 15.0 {
                parts.push(format!(
                    "This combination provides {:.0}g of protein per serving — an excellent protein source",
                    protein
                ));
            } else {
                parts.push(format!(
                    "The combination of {names} delivers {:.0} kcal per serving",
                    calories
                ));
            }
        }
    }

    // Part 2: Fiber bonus
    if fiber > 3.0 {
        let fiber_note = match locale {
            "ru" => format!(
                "Содержит {:.1} г клетчатки, что поддерживает пищеварение",
                fiber
            ),
            "pl" => format!(
                "Zawiera {:.1} g błonnika, co wspiera trawienie",
                fiber
            ),
            "uk" => format!(
                "Містить {:.1} г клітковини, що підтримує травлення",
                fiber
            ),
            _ => format!(
                "Contains {:.1}g of fiber, supporting healthy digestion",
                fiber
            ),
        };
        parts.push(fiber_note);
    }

    // Part 3: Flavor balance
    if let Some(score) = balance_score {
        let tastes_str = if !dominant_tastes.is_empty() {
            dominant_tastes.join(", ")
        } else {
            String::new()
        };
        let flavor_note = match locale {
            "ru" => {
                if tastes_str.is_empty() {
                    format!("Баланс вкуса оценивается в {:.0}/100", score)
                } else {
                    format!(
                        "Доминирующие вкусы — {tastes_str}, баланс {:.0}/100",
                        score
                    )
                }
            }
            "pl" => {
                if tastes_str.is_empty() {
                    format!("Balans smakowy oceniany na {:.0}/100", score)
                } else {
                    format!(
                        "Dominujące smaki — {tastes_str}, balans {:.0}/100",
                        score
                    )
                }
            }
            "uk" => {
                if tastes_str.is_empty() {
                    format!("Баланс смаку оцінюється в {:.0}/100", score)
                } else {
                    format!(
                        "Домінуючі смаки — {tastes_str}, баланс {:.0}/100",
                        score
                    )
                }
            }
            _ => {
                if tastes_str.is_empty() {
                    format!("Flavor balance scores {:.0}/100", score)
                } else {
                    format!(
                        "Dominant flavors are {tastes_str}, with a balance score of {:.0}/100",
                        score
                    )
                }
            }
        };
        parts.push(flavor_note);
    }

    // Part 4: Goal context
    if let Some(g) = goal {
        let goal_note = match locale {
            "ru" => format!("Оптимизировано для цели: {}", g.replace('_', " ")),
            "pl" => format!("Zoptymalizowane pod cel: {}", g.replace('_', " ")),
            "uk" => format!("Оптимізовано для мети: {}", g.replace('_', " ")),
            _ => format!("Optimized for {}", g.replace('_', " ")),
        };
        parts.push(goal_note);
    }

    // Part 5: Variant versatility
    if !variant_types.is_empty() {
        let types_str = variant_types.join(", ");
        let versatility = match locale {
            "ru" => format!("Подходит для разных стилей подачи: {types_str}"),
            "pl" => format!("Pasuje do różnych stylów podania: {types_str}"),
            "uk" => format!("Підходить для різних стилів подачі: {types_str}"),
            _ => format!("Versatile enough for multiple serving styles: {types_str}"),
        };
        parts.push(versatility);
    }

    if parts.is_empty() {
        return match locale {
            "ru" => format!(
                "{names} — сбалансированное сочетание белка, углеводов и полезных жиров."
            ),
            "pl" => format!(
                "{names} — zbalansowane połączenie białka, węglowodanów i zdrowych tłuszczów."
            ),
            "uk" => format!(
                "{names} — збалансоване поєднання білка, вуглеводів і корисних жирів."
            ),
            _ => format!(
                "{names} — a balanced mix of protein, carbs, and healthy fats."
            ),
        };
    }

    format!("{}.", parts.join(". "))
}

/// Generate template cooking steps from SmartResponse variants data.
pub fn generate_how_to_cook(
    ingredients: &[String],
    smart: &serde_json::Value,
    locale: &str,
) -> serde_json::Value {
    let variants = smart.get("variants").and_then(|v| v.as_array());

    let reference_variant = variants.and_then(|vars| {
        vars.iter()
            .find(|v| v.get("variant_type").and_then(|t| t.as_str()) == Some("balanced"))
            .or_else(|| vars.first())
    });

    let mut steps: Vec<serde_json::Value> = Vec::new();

    let raw_only = [
        "avocado", "lettuce", "arugula", "cucumber", "basil", "cilantro", "parsley", "dill",
        "mint", "lemon", "lime",
    ];
    let grains = [
        "rice", "pasta", "quinoa", "bulgur", "couscous", "oats", "noodle", "noodles",
    ];
    let proteins = [
        "salmon", "chicken", "beef", "pork", "tuna", "cod", "shrimp", "prawn", "turkey", "lamb",
        "duck", "egg", "eggs", "tofu",
    ];

    if let Some(variant) = reference_variant {
        let variant_ingredients = variant.get("ingredients").and_then(|i| i.as_array());

        if let Some(vi) = variant_ingredients {
            let mut grain_items: Vec<(&str, f64)> = Vec::new();
            let mut protein_items: Vec<(&str, f64)> = Vec::new();
            let mut raw_items: Vec<(&str, f64)> = Vec::new();
            let mut other_cook_items: Vec<(&str, f64)> = Vec::new();

            for ing in vi.iter() {
                let name = ing.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let grams = ing.get("grams").and_then(|g| g.as_f64()).unwrap_or(100.0);
                let name_lower = name.to_lowercase();

                if raw_only.iter().any(|r| name_lower.contains(r)) {
                    raw_items.push((name, grams));
                } else if grains.iter().any(|g| name_lower.contains(g)) {
                    grain_items.push((name, grams));
                } else if proteins.iter().any(|p| name_lower.contains(p)) {
                    protein_items.push((name, grams));
                } else {
                    other_cook_items.push((name, grams));
                }
            }

            let mut step_num = 1;

            if !grain_items.is_empty() {
                let details: Vec<String> = grain_items
                    .iter()
                    .map(|(name, grams)| format!("{name} ({grams:.0}g)"))
                    .collect();
                let step_text = match locale {
                    "ru" => format!("Сварите {} в подсоленной воде (соотношение 2:1) 12–15 мин. Снимите с огня, накройте и оставьте на 5 мин.", details.join(", ")),
                    "pl" => format!("Ugotuj {} w osolonej wodzie (proporcja 2:1) 12–15 min. Zdejmij z ognia, przykryj i zostaw na 5 min.", details.join(", ")),
                    "uk" => format!("Зваріть {} у підсоленій воді (співвідношення 2:1) 12–15 хв. Зніміть з вогню, накрийте і залиште на 5 хв.", details.join(", ")),
                    _    => format!("Boil {} in salted water (2:1 ratio) for 12–15 min. Remove from heat, cover, and let rest 5 min.", details.join(", ")),
                };
                steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 15 }));
                step_num += 1;
            }

            if !protein_items.is_empty() {
                let details: Vec<String> = protein_items
                    .iter()
                    .map(|(name, grams)| format!("{name} ({grams:.0}g)"))
                    .collect();
                let name_lower = protein_items[0].0.to_lowercase();
                let (method_en, method_ru, method_pl, method_uk, time) = if name_lower.contains("egg")
                {
                    ("Fry eggs in a non-stick pan over medium heat", "Обжарьте яйца на сковороде с антипригарным покрытием на среднем огне", "Usmaż jajka na patelni z powłoką nieprzywierającą na średnim ogniu", "Обсмажте яйця на сковороді з антипригарним покриттям на середньому вогні", 4)
                } else if name_lower.contains("shrimp") || name_lower.contains("prawn") {
                    ("Sauté shrimp in olive oil over high heat until pink", "Обжарьте креветки в оливковом масле на сильном огне до розового цвета", "Usmaż krewetki na oliwie z oliwek na dużym ogniu do różowego koloru", "Обсмажте креветки в оливковій олії на сильному вогні до рожевого кольору", 4)
                } else {
                    ("Pan-sear over medium-high heat, 4–5 min per side until golden", "Обжарьте на среднем-сильном огне 4–5 мин с каждой стороны до золотистой корочки", "Usmaż na średnio-dużym ogniu 4–5 min z każdej strony do złotego koloru", "Обсмажте на середньо-сильному вогні 4–5 хв з кожного боку до золотистої скоринки", 10)
                };
                let step_text = match locale {
                    "ru" => format!("{} {}.", method_ru, details.join(", ")),
                    "pl" => format!("{} {}.", method_pl, details.join(", ")),
                    "uk" => format!("{} {}.", method_uk, details.join(", ")),
                    _ => format!("{} {}.", method_en, details.join(", ")),
                };
                steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": time }));
                step_num += 1;
            }

            if !other_cook_items.is_empty() {
                let details: Vec<String> = other_cook_items
                    .iter()
                    .map(|(name, grams)| format!("{name} ({grams:.0}g)"))
                    .collect();
                let step_text = match locale {
                    "ru" => format!(
                        "Обжарьте {} на среднем огне 3–4 мин, помешивая.",
                        details.join(", ")
                    ),
                    "pl" => format!(
                        "Usmaż {} na średnim ogniu 3–4 min, mieszając.",
                        details.join(", ")
                    ),
                    "uk" => format!(
                        "Обсмажте {} на середньому вогні 3–4 хв, помішуючи.",
                        details.join(", ")
                    ),
                    _ => format!(
                        "Sauté {} over medium heat for 3–4 min, stirring.",
                        details.join(", ")
                    ),
                };
                steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 4 }));
                step_num += 1;
            }

            if !raw_items.is_empty() {
                let details: Vec<String> = raw_items
                    .iter()
                    .map(|(name, grams)| format!("{name} ({grams:.0}g)"))
                    .collect();
                let step_text = match locale {
                    "ru" => format!(
                        "Нарежьте {} и выложите в тарелку.",
                        details.join(", ")
                    ),
                    "pl" => format!("Pokrój {} i ułóż na talerzu.", details.join(", ")),
                    "uk" => format!(
                        "Наріжте {} та викладіть на тарілку.",
                        details.join(", ")
                    ),
                    _ => format!("Slice {} and arrange on the plate.", details.join(", ")),
                };
                steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 2 }));
                step_num += 1;
            }

            // Final assemble step
            let total_cal = variant
                .get("total_calories")
                .and_then(|c| c.as_i64())
                .unwrap_or(0);
            let total_prot = variant
                .get("total_protein")
                .and_then(|p| p.as_f64())
                .or_else(|| {
                    smart
                        .get("nutrition")
                        .and_then(|n| n.get("protein"))
                        .and_then(|v| v.as_f64())
                        .map(|p| p * 3.0)
                })
                .unwrap_or(0.0);

            let assemble = match locale {
                "ru" => format!("Соберите блюдо: выложите все компоненты на тарелку и подавайте. Порция: ~{total_cal} ккал, ~{total_prot:.0} г белка."),
                "pl" => format!("Złóż danie: ułóż wszystkie składniki na talerzu i podaj. Porcja: ~{total_cal} kcal, ~{total_prot:.0} g białka."),
                "uk" => format!("Зберіть страву: викладіть усі компоненти на тарілку та подавайте. Порція: ~{total_cal} ккал, ~{total_prot:.0} г білка."),
                _    => format!("Assemble: arrange all components on the plate and serve. Per serving: ~{total_cal} kcal, ~{total_prot:.0}g protein."),
            };
            steps.push(serde_json::json!({ "step": step_num, "text": assemble, "time_minutes": 2 }));
        }
    }

    // Fallback: ingredient-aware steps
    if steps.is_empty() {
        let mut step_num = 1;
        let mut fallback_steps: Vec<serde_json::Value> = Vec::new();
        let mut has_grain = false;
        let mut has_protein = false;

        for ing in ingredients {
            let ing_lower = ing.to_lowercase();
            if grains.iter().any(|g| ing_lower.contains(g)) && !has_grain {
                has_grain = true;
                let step_text = match locale {
                    "ru" => format!("Сварите {} (100 г) в подсоленной воде 12–15 мин.", ing),
                    "pl" => format!("Ugotuj {} (100 g) w osolonej wodzie 12–15 min.", ing),
                    "uk" => format!("Зваріть {} (100 г) у підсоленій воді 12–15 хв.", ing),
                    _ => format!("Boil {} (100g) in salted water for 12–15 min.", ing),
                };
                fallback_steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 15 }));
                step_num += 1;
            } else if proteins.iter().any(|p| ing_lower.contains(p)) && !has_protein {
                has_protein = true;
                let step_text = match locale {
                    "ru" => format!("Обжарьте {} (150 г) на среднем-сильном огне 4–5 мин с каждой стороны.", ing),
                    "pl" => format!("Usmaż {} (150 g) na średnio-dużym ogniu 4–5 min z każdej strony.", ing),
                    "uk" => format!("Обсмажте {} (150 г) на середньо-сильному вогні 4–5 хв з кожного боку.", ing),
                    _    => format!("Pan-sear {} (150g) over medium-high heat, 4–5 min per side.", ing),
                };
                fallback_steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 10 }));
                step_num += 1;
            } else if raw_only.iter().any(|r| ing_lower.contains(r)) {
                let step_text = match locale {
                    "ru" => format!("Нарежьте {} (80 г) и отложите.", ing),
                    "pl" => format!("Pokrój {} (80 g) i odłóż.", ing),
                    "uk" => format!("Наріжте {} (80 г) та відкладіть.", ing),
                    _ => format!("Slice {} (80g) and set aside.", ing),
                };
                fallback_steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 2 }));
                step_num += 1;
            }
        }

        let assemble = match locale {
            "ru" => "Соберите блюдо: выложите все компоненты на тарелку и подавайте.".to_string(),
            "pl" => "Złóż danie: ułóż wszystkie składniki na talerzu i podaj.".to_string(),
            "uk" => "Зберіть страву: викладіть усі компоненти на тарілку та подавайте.".to_string(),
            _ => "Assemble: arrange all components on the plate and serve.".to_string(),
        };
        fallback_steps.push(serde_json::json!({ "step": step_num, "text": assemble, "time_minutes": 2 }));

        steps = fallback_steps;
    }

    serde_json::json!(steps)
}

/// Generate optimization tips from SmartResponse next_actions + diagnostics.
pub fn generate_optimization_tips(
    smart: &serde_json::Value,
    locale: &str,
) -> serde_json::Value {
    let mut tips: Vec<serde_json::Value> = Vec::new();

    if let Some(actions) = smart.get("next_actions").and_then(|a| a.as_array()) {
        for action in actions.iter().take(5) {
            let action_type = action
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("add");
            let name = action.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let reason = action.get("reason").and_then(|r| r.as_str()).unwrap_or("");
            if name.is_empty() {
                continue;
            }

            let icon = match action_type {
                "add" => "➕",
                "remove" => "➖",
                "swap" => "🔄",
                "adjust" => "⚙️",
                _ => "💡",
            };

            tips.push(serde_json::json!({
                "icon": icon,
                "action": action_type,
                "ingredient": name,
                "tip": reason
            }));
        }
    }

    if let Some(diag) = smart.get("diagnostics") {
        if let Some(issues) = diag.get("issues").and_then(|i| i.as_array()) {
            for issue in issues.iter().take(3) {
                let severity = issue
                    .get("severity")
                    .and_then(|s| s.as_str())
                    .unwrap_or("info");
                let message = issue
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                if message.is_empty() {
                    continue;
                }

                let icon = match severity {
                    "critical" => "🔴",
                    "warning" => "🟡",
                    _ => "💡",
                };

                tips.push(serde_json::json!({
                    "icon": icon,
                    "action": "tip",
                    "ingredient": "",
                    "tip": message
                }));
            }
        }
    }

    if tips.is_empty() {
        let generic = match locale {
            "ru" => vec![
                ("➕", "Добавьте оливковое масло для улучшения текстуры и усвоения жирорастворимых витаминов"),
                ("➕", "Добавьте лимонный сок для яркости вкуса и лучшего усвоения железа"),
                ("⚙️", "Контролируйте порцию — начните с рекомендованных граммов и корректируйте"),
            ],
            "pl" => vec![
                ("➕", "Dodaj oliwę z oliwek dla lepszej tekstury i wchłaniania witamin rozpuszczalnych w tłuszczach"),
                ("➕", "Dodaj sok z cytryny dla świeżości smaku i lepszego wchłaniania żelaza"),
                ("⚙️", "Kontroluj porcję — zacznij od zalecanych gramów i dostosuj"),
            ],
            "uk" => vec![
                ("➕", "Додайте оливкову олію для кращої текстури та засвоєння жиророзчинних вітамінів"),
                ("➕", "Додайте лимонний сік для яскравості смаку та кращого засвоєння заліза"),
                ("⚙️", "Контролюйте порцію — почніть із рекомендованих грамів та коригуйте"),
            ],
            _ => vec![
                ("➕", "Add olive oil to improve mouthfeel and fat-soluble vitamin absorption"),
                ("➕", "Add lemon juice for brightness and better iron absorption"),
                ("⚙️", "Control portions — start with recommended grams and adjust to taste"),
            ],
        };
        for (icon, tip) in generic {
            tips.push(serde_json::json!({
                "icon": icon,
                "action": "tip",
                "ingredient": "",
                "tip": tip
            }));
        }
    }

    serde_json::json!(tips)
}
