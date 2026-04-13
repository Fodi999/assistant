//! Response Templates — "как это звучит"
//!
//! Pure functions that generate human-readable text.
//! No IO, no async, no side effects. Only `&data → String`.
//!
//! Each function receives a typed context and returns localized text.

use super::intent_router::ChatLang;
use crate::infrastructure::ingredient_cache::IngredientData;
use super::meal_builder::MealCombo;

// Re-export HealthGoal so templates can use it
pub use super::response_builder::HealthGoal;

// ── Greeting ─────────────────────────────────────────────────────────────────

pub fn greeting(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "Привет 👋 Я ChefOS — твой кулинарный помощник! Спроси меня:\n• «что полезного поесть»\n• «сколько калорий в шпинате»\n• «200 грамм в ложках»\n• «что приготовить на ужин»",
        ChatLang::En => "Hello 👋 I'm ChefOS — your culinary assistant! Ask me:\n• \"healthy product ideas\"\n• \"calories in spinach\"\n• \"convert 200g to tablespoons\"\n• \"dinner idea\"",
        ChatLang::Pl => "Cześć 👋 Jestem ChefOS — Twoim kulinarnym asystentem! Zapytaj mnie:\n• «zdrowy produkt»\n• «kalorie szpinaku»\n• «200 gramów na łyżki»\n• «co ugotować na obiad»",
        ChatLang::Uk => "Привіт 👋 Я ChefOS — твій кулінарний помічник! Запитай мене:\n• «що корисного з'їсти»\n• «калорії шпинату»\n• «200 грамів в ложках»\n• «що приготувати на вечерю»",
    }
}

// ── Unknown / fallback ───────────────────────────────────────────────────────

pub fn unknown(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "Не совсем понял 🤔 Попробуй:\n• «что полезного поесть»\n• «калории в шпинате»\n• «200 грамм в ложках»\n• «что приготовить на ужин»\n• «что такое лосось»",
        ChatLang::En => "I'm not sure what you mean 🤔 Try:\n• \"healthy food ideas\"\n• \"calories in spinach\"\n• \"convert 200g to tablespoons\"\n• \"dinner idea\"\n• \"what is salmon\"",
        ChatLang::Pl => "Nie rozumiem 🤔 Spróbuj:\n• «zdrowy produkt»\n• «kalorie szpinaku»\n• «przelicz 200g na łyżki»\n• «pomysł na obiad»\n• «co to jest szpinak»",
        ChatLang::Uk => "Не зовсім зрозумів 🤔 Спробуй:\n• «що корисного з'їсти»\n• «калорії шпинату»\n• «200 грамів в ложках»\n• «що приготувати на вечерю»\n• «що таке лосось»",
    }
}

// ── Healthy product fallback ─────────────────────────────────────────────────

pub fn healthy_fallback(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "🥗 Полезные продукты: шпинат, брокколи, лосось, куриная грудка, яйца, миндаль. Спроси о конкретном — расскажу подробнее!",
        ChatLang::En => "🥗 Healthy picks: spinach, broccoli, salmon, chicken breast, eggs, almonds. Ask about a specific one for details!",
        ChatLang::Pl => "🥗 Zdrowe produkty: szpinak, brokuły, łosoś, filet z kurczaka, jajka, migdały. Zapytaj o konkretny — powiem więcej!",
        ChatLang::Uk => "🥗 Корисні продукти: шпинат, броколі, лосось, куряча грудка, яйця, мигдаль. Запитай про конкретний — розповім докладніше!",
    }
}

// ── Healthy product text (expert-style) ──────────────────────────────────────

pub fn healthy_text(name: &str, p: &IngredientData, lang: ChatLang, goal: HealthGoal) -> String {
    let cal = p.calories_per_100g as i32;
    let pro = p.protein_per_100g;
    let fat = p.fat_per_100g;
    let carb = p.carbs_per_100g;

    let mut bullets = Vec::new();

    if pro >= 15.0 {
        bullets.push(match lang {
            ChatLang::Ru => format!("• много белка ({:.0}г) → держит сытость", pro),
            ChatLang::En => format!("• high protein ({:.0}g) → keeps you full longer", pro),
            ChatLang::Pl => format!("• dużo białka ({:.0}g) → dłużej trzyma sytość", pro),
            ChatLang::Uk => format!("• багато білка ({:.0}г) → тримає ситість", pro),
        });
    }
    if cal < 150 {
        bullets.push(match lang {
            ChatLang::Ru => format!("• мало калорий ({} ккал) → можно есть больше", cal),
            ChatLang::En => format!("• low calories ({} kcal) → more food per day", cal),
            ChatLang::Pl => format!("• mało kalorii ({} kcal) → możesz jeść więcej", cal),
            ChatLang::Uk => format!("• мало калорій ({} ккал) → можна їсти більше", cal),
        });
    }
    if carb < 5.0 {
        bullets.push(match lang {
            ChatLang::Ru => "• почти нет углеводов → не скачет инсулин".into(),
            ChatLang::En => "• near-zero carbs → stable insulin".into(),
            ChatLang::Pl => "• prawie zero węglowodanów → stabilna insulina".into(),
            ChatLang::Uk => "• майже нуль вуглеводів → стабільний інсулін".into(),
        });
    } else if carb < 15.0 {
        bullets.push(match lang {
            ChatLang::Ru => format!("• мало углеводов ({:.0}г) → стабильный сахар в крови", carb),
            ChatLang::En => format!("• low carbs ({:.0}g) → stable blood sugar", carb),
            ChatLang::Pl => format!("• mało węglowodanów ({:.0}g) → stabilny cukier", carb),
            ChatLang::Uk => format!("• мало вуглеводів ({:.0}г) → стабільний цукор", carb),
        });
    }
    if fat < 3.0 {
        bullets.push(match lang {
            ChatLang::Ru => "• минимум жира → чистый белок".into(),
            ChatLang::En => "• minimal fat → clean protein source".into(),
            ChatLang::Pl => "• minimum tłuszczu → czyste białko".into(),
            ChatLang::Uk => "• мінімум жиру → чистий білок".into(),
        });
    }

    if bullets.is_empty() {
        bullets.push(match lang {
            ChatLang::Ru => format!("• {} ккал · {:.0}г белка · {:.0}г жиров · {:.0}г углеводов", cal, pro, fat, carb),
            ChatLang::En => format!("• {} kcal · {:.0}g protein · {:.0}g fat · {:.0}g carbs", cal, pro, fat, carb),
            ChatLang::Pl => format!("• {} kcal · {:.0}g białka · {:.0}g tłuszczu · {:.0}g węglowodanów", cal, pro, fat, carb),
            ChatLang::Uk => format!("• {} ккал · {:.0}г білка · {:.0}г жирів · {:.0}г вуглеводів", cal, pro, fat, carb),
        });
    }

    let opener = match (lang, goal) {
        (ChatLang::Ru, HealthGoal::LowCalorie)  => format!("Для похудения **{}** — хороший вариант:", name),
        (ChatLang::Ru, HealthGoal::HighProtein)  => format!("Для набора массы **{}** — сильный выбор:", name),
        (ChatLang::Ru, HealthGoal::Balanced)     => format!("**{}** — сбалансированный вариант:", name),
        (ChatLang::En, HealthGoal::LowCalorie)   => format!("For weight loss, **{}** works well:", name),
        (ChatLang::En, HealthGoal::HighProtein)   => format!("For muscle gain, **{}** is a strong pick:", name),
        (ChatLang::En, HealthGoal::Balanced)      => format!("**{}** — a balanced option:", name),
        (ChatLang::Pl, HealthGoal::LowCalorie)   => format!("Na odchudzanie **{}** — dobry wybór:", name),
        (ChatLang::Pl, HealthGoal::HighProtein)   => format!("Na masę **{}** — mocny wybór:", name),
        (ChatLang::Pl, HealthGoal::Balanced)      => format!("**{}** — zbalansowana opcja:", name),
        (ChatLang::Uk, HealthGoal::LowCalorie)   => format!("Для схуднення **{}** — хороший варіант:", name),
        (ChatLang::Uk, HealthGoal::HighProtein)   => format!("Для набору маси **{}** — сильний вибір:", name),
        (ChatLang::Uk, HealthGoal::Balanced)      => format!("**{}** — збалансований варіант:", name),
    };

    format!("{}\n{}", opener, bullets.join("\n"))
}

// ── Highlight (product card badge) ───────────────────────────────────────────

pub fn highlight(p: &IngredientData, lang: ChatLang, goal: HealthGoal) -> String {
    match goal {
        HealthGoal::HighProtein => match lang {
            ChatLang::Ru => format!("высокий белок — {:.1}г/100г", p.protein_per_100g),
            ChatLang::En => format!("high protein — {:.1}g/100g", p.protein_per_100g),
            ChatLang::Pl => format!("wysokie białko — {:.1}g/100g", p.protein_per_100g),
            ChatLang::Uk => format!("високий білок — {:.1}г/100г", p.protein_per_100g),
        },
        HealthGoal::LowCalorie => match lang {
            ChatLang::Ru => format!("мало калорий — {} ккал/100г", p.calories_per_100g as i32),
            ChatLang::En => format!("low calorie — {} kcal/100g", p.calories_per_100g as i32),
            ChatLang::Pl => format!("mało kalorii — {} kcal/100g", p.calories_per_100g as i32),
            ChatLang::Uk => format!("мало калорій — {} ккал/100г", p.calories_per_100g as i32),
        },
        HealthGoal::Balanced => {
            if p.protein_per_100g >= 20.0 {
                match lang {
                    ChatLang::Ru => format!("высокий белок — {:.1}г/100г", p.protein_per_100g),
                    ChatLang::En => format!("high protein — {:.1}g/100g", p.protein_per_100g),
                    ChatLang::Pl => format!("wysokie białko — {:.1}g/100g", p.protein_per_100g),
                    ChatLang::Uk => format!("високий білок — {:.1}г/100г", p.protein_per_100g),
                }
            } else if p.calories_per_100g < 50.0 {
                match lang {
                    ChatLang::Ru => format!("мало калорий — {} ккал/100г", p.calories_per_100g as i32),
                    ChatLang::En => format!("low calorie — {} kcal/100g", p.calories_per_100g as i32),
                    ChatLang::Pl => format!("mało kalorii — {} kcal/100g", p.calories_per_100g as i32),
                    ChatLang::Uk => format!("мало калорій — {} ккал/100г", p.calories_per_100g as i32),
                }
            } else {
                match lang {
                    ChatLang::Ru => format!("{} ккал/100г", p.calories_per_100g as i32),
                    ChatLang::En => format!("{} kcal/100g", p.calories_per_100g as i32),
                    ChatLang::Pl => format!("{} kcal/100g", p.calories_per_100g as i32),
                    ChatLang::Uk => format!("{} ккал/100г", p.calories_per_100g as i32),
                }
            }
        }
    }
}

// ── Macro summary (reason field) ─────────────────────────────────────────────

pub fn macro_summary(p: &IngredientData, lang: ChatLang, goal: HealthGoal, total_options: usize) -> String {
    let pro = p.protein_per_100g;
    let fat = p.fat_per_100g;
    let cal = p.calories_per_100g as i32;

    let pro_level = if pro >= 20.0 { "high" } else if pro >= 10.0 { "moderate" } else { "low" };
    let fat_level = if fat >= 15.0 { "high" } else if fat >= 5.0 { "moderate" } else { "low" };

    let extras = if total_options > 1 {
        match lang {
            ChatLang::Ru => format!(" · +{} вариантов ниже", total_options - 1),
            ChatLang::En => format!(" · +{} more below", total_options - 1),
            ChatLang::Pl => format!(" · +{} więcej poniżej", total_options - 1),
            ChatLang::Uk => format!(" · +{} варіантів нижче", total_options - 1),
        }
    } else {
        String::new()
    };

    match lang {
        ChatLang::Ru => {
            let pro_s = match pro_level { "high" => "белок высокий", "moderate" => "белок средний", _ => "белка мало" };
            let fat_s = match fat_level { "high" => "жир высокий", "moderate" => "жир умеренный", _ => "жира минимум" };
            let action = match goal {
                HealthGoal::LowCalorie  => format!(" → {} ккал, можно улучшить", cal),
                HealthGoal::HighProtein => format!(" → {:.0}г белка/100г, хороший старт", pro),
                HealthGoal::Balanced    => " → баланс ОК".into(),
            };
            format!("{}, {}{}{}", pro_s, fat_s, action, extras)
        }
        ChatLang::En => {
            let pro_s = match pro_level { "high" => "protein high", "moderate" => "protein moderate", _ => "protein low" };
            let fat_s = match fat_level { "high" => "fat high", "moderate" => "fat moderate", _ => "fat minimal" };
            let action = match goal {
                HealthGoal::LowCalorie  => format!(" → {} kcal, room to improve", cal),
                HealthGoal::HighProtein => format!(" → {:.0}g protein/100g, good start", pro),
                HealthGoal::Balanced    => " → balance OK".into(),
            };
            format!("{}, {}{}{}", pro_s, fat_s, action, extras)
        }
        ChatLang::Pl => {
            let pro_s = match pro_level { "high" => "białko wysokie", "moderate" => "białko średnie", _ => "białka mało" };
            let fat_s = match fat_level { "high" => "tłuszcz wysoki", "moderate" => "tłuszcz umiarkowany", _ => "tłuszczu minimum" };
            let action = match goal {
                HealthGoal::LowCalorie  => format!(" → {} kcal, można poprawić", cal),
                HealthGoal::HighProtein => format!(" → {:.0}g białka/100g, dobry start", pro),
                HealthGoal::Balanced    => " → balans OK".into(),
            };
            format!("{}, {}{}{}", pro_s, fat_s, action, extras)
        }
        ChatLang::Uk => {
            let pro_s = match pro_level { "high" => "білок високий", "moderate" => "білок середній", _ => "білка мало" };
            let fat_s = match fat_level { "high" => "жир високий", "moderate" => "жир помірний", _ => "жиру мінімум" };
            let action = match goal {
                HealthGoal::LowCalorie  => format!(" → {} ккал, можна покращити", cal),
                HealthGoal::HighProtein => format!(" → {:.0}г білка/100г, хороший старт", pro),
                HealthGoal::Balanced    => " → баланс ОК".into(),
            };
            format!("{}, {}{}{}", pro_s, fat_s, action, extras)
        }
    }
}

// ── Nutrition info ───────────────────────────────────────────────────────────

pub fn nutrition_text(name: &str, p: &IngredientData, lang: ChatLang) -> String {
    match lang {
        ChatLang::Ru => format!(
            "📊 **{}** (на 100г):\n• Калории: {} ккал\n• Белки: {} г\n• Жиры: {} г\n• Углеводы: {} г",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::En => format!(
            "📊 **{}** (per 100g):\n• Calories: {} kcal\n• Protein: {}g\n• Fat: {}g\n• Carbs: {}g",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Pl => format!(
            "📊 **{}** (na 100g):\n• Kalorie: {} kcal\n• Białko: {}g\n• Tłuszcz: {}g\n• Węglowodany: {}g",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Uk => format!(
            "📊 **{}** (на 100г):\n• Калорії: {} ккал\n• Білки: {}г\n• Жири: {}г\n• Вуглеводи: {}г",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
    }
}

// ── Product info (detailed) ──────────────────────────────────────────────────

pub fn product_info_text(name: &str, p: &IngredientData, lang: ChatLang) -> String {
    match lang {
        ChatLang::Ru => format!(
            "🔍 **{}**\n\nНутриенты на 100г:\n• Калории: {} ккал\n• Белки: {:.1} г\n• Жиры: {:.1} г\n• Углеводы: {:.1} г",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::En => format!(
            "🔍 **{}**\n\nNutrition per 100g:\n• Calories: {} kcal\n• Protein: {:.1}g\n• Fat: {:.1}g\n• Carbs: {:.1}g",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Pl => format!(
            "🔍 **{}**\n\nWartości na 100g:\n• Kalorie: {} kcal\n• Białko: {:.1}g\n• Tłuszcz: {:.1}g\n• Węglowodany: {:.1}g",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Uk => format!(
            "🔍 **{}**\n\nПоживні речовини на 100г:\n• Калорії: {} ккал\n• Білки: {:.1}г\n• Жири: {:.1}г\n• Вуглеводи: {:.1}г",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
    }
}

// ── Conversion ───────────────────────────────────────────────────────────────

pub fn conversion_text(value: f64, from: &str, result: f64, to: &str, supported: bool, lang: ChatLang) -> String {
    if !supported {
        return match lang {
            ChatLang::Ru => format!("Не могу перевести {} {} в {} — такая конвертация не поддерживается. Попробуй: г, кг, мл, л, ст.л., ч.л.", value, from, to),
            ChatLang::En => format!("Cannot convert {} {} to {} — unsupported conversion. Try: g, kg, ml, l, tbsp, tsp.", value, from, to),
            ChatLang::Pl => format!("Nie mogę przeliczyć {} {} na {} — taka konwersja nie jest obsługiwana. Spróbuj: g, kg, ml, l, łyżka, łyżeczka.", value, from, to),
            ChatLang::Uk => format!("Не можу перевести {} {} в {} — така конвертація не підтримується.", value, from, to),
        };
    }
    format!("✅ {} {} = **{} {}**", value, from, result, to)
}

pub fn conversion_hint(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "Напиши, например: «переведи 200 грамм в унции» или «сколько ложек в 100 мл»",
        ChatLang::En => "Try something like: \"convert 200 grams to ounces\" or \"how many tablespoons in 100ml\"",
        ChatLang::Pl => "Spróbuj: «przelicz 200 gramów na uncje» lub «ile łyżek to 100 ml»",
        ChatLang::Uk => "Напиши, наприклад: «переведи 200 грамів в унції» або «скільки ложок в 100 мл»",
    }
}

// ── Nutrition hint ───────────────────────────────────────────────────────────

pub fn nutrition_hint(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "Укажи продукт — например: «калории шпината» или «белок в курице»",
        ChatLang::En => "Tell me which product — e.g. \"calories in spinach\" or \"protein in chicken\"",
        ChatLang::Pl => "Podaj produkt — np. «kalorie szpinaku» lub «białko w kurczaku»",
        ChatLang::Uk => "Вкажи продукт — наприклад: «калорії шпинату» або «білок у курці»",
    }
}

// ── Seasonality ──────────────────────────────────────────────────────────────

pub fn season_text(product: &str, lang: ChatLang) -> String {
    let season_info = match product {
        "salmon" | "лосось" => ("salmon", "June–September", "Июнь–Сентябрь", "Czerwiec–Wrzesień"),
        "strawberry" | "клубника" | "truskawka" => ("strawberry", "May–July", "Май–Июль", "Maj–Lipiec"),
        "herring" | "сельдь" | "śledź" => ("herring", "October–April", "Октябрь–Апрель", "Październik–Kwiecień"),
        "mushrooms" | "грибы" | "grzyby" => ("mushrooms", "August–October", "Август–Октябрь", "Sierpień–Październik"),
        _ => return season_hint(lang).to_string(),
    };
    match lang {
        ChatLang::Ru => format!("📅 **{}**: сезон {}", season_info.0, season_info.2),
        ChatLang::En => format!("📅 **{}**: season {}", season_info.0, season_info.1),
        ChatLang::Pl => format!("📅 **{}**: sezon {}", season_info.0, season_info.3),
        ChatLang::Uk => format!("📅 **{}**: сезон {}", season_info.0, season_info.2),
    }
}

pub fn season_hint(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "Спроси о конкретном продукте — например: «сезон лосося» или «когда клубника в сезоне»",
        ChatLang::En => "Ask about a specific product — e.g. \"salmon season\" or \"when are strawberries in season\"",
        ChatLang::Pl => "Zapytaj o konkretny produkt — np. «sezon łososia» lub «kiedy truskawki są w sezonie»",
        ChatLang::Uk => "Запитай про конкретний продукт — наприклад: «сезон лосося» або «коли полуниця в сезоні»",
    }
}

// ── Recipe hint ──────────────────────────────────────────────────────────────

pub fn recipe_hint(dish: &str, lang: ChatLang) -> String {
    match lang {
        ChatLang::Ru => format!("🍳 Ищешь рецепт: **{}**? Перейди в раздел «Рецепты» — там подробные шаги, КБЖУ и стоимость ингредиентов.", dish),
        ChatLang::En => format!("🍳 Looking for a **{}** recipe? Check the \"Recipes\" section — detailed steps, macros and ingredient costs.", dish),
        ChatLang::Pl => format!("🍳 Szukasz przepisu na **{}**? Przejdź do sekcji «Przepisy» — szczegółowe kroki, makroskładniki i ceny.", dish),
        ChatLang::Uk => format!("🍳 Шукаєш рецепт: **{}**? Перейди до розділу «Рецепти» — там покрокові інструкції, КБЖУ та вартість.", dish),
    }
}

pub fn recipe_generic(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "Для рецептов перейди в раздел «Рецепты» — там можно найти рецепты с подробными шагами, калориями и стоимостью ингредиентов 🍳",
        ChatLang::En => "For recipes, visit the \"Recipes\" section — you'll find step-by-step instructions, nutrition info and ingredient costs 🍳",
        ChatLang::Pl => "Po przepisy przejdź do sekcji «Przepisy» — znajdziesz tam krok po kroku instrukcje, kalorie i ceny składników 🍳",
        ChatLang::Uk => "Для рецептів перейди до розділу «Рецепти» — там є покрокові інструкції, калорії та вартість інгредієнтів 🍳",
    }
}

// ── Meal idea ────────────────────────────────────────────────────────────────

pub fn meal_idea_with_product(meal_name: &str, description: &str, ingredient_name: &str, cal: i32, lang: ChatLang) -> String {
    match lang {
        ChatLang::Ru => format!("🍽️ Идея на сегодня: **{}**\n\n{}\n\nГлавный ингредиент: {} ({} ккал/100г)", meal_name, description, ingredient_name, cal),
        ChatLang::En => format!("🍽️ Today's idea: **{}**\n\n{}\n\nMain ingredient: {} ({} kcal/100g)", meal_name, description, ingredient_name, cal),
        ChatLang::Pl | ChatLang::Uk => format!("🍽️ Pomysł na dziś: **{}**\n\n{}\n\nGłówny składnik: {} ({} kcal/100g)", meal_name, description, ingredient_name, cal),
    }
}

pub fn meal_idea_text_only(meal_name: &str, description: &str, lang: ChatLang) -> String {
    match lang {
        ChatLang::Ru => format!("🍽️ Идея на сегодня: **{}**\n\n{}", meal_name, description),
        ChatLang::En => format!("🍽️ Today's idea: **{}**\n\n{}", meal_name, description),
        ChatLang::Pl | ChatLang::Uk => format!("🍽️ Pomysł na dziś: **{}**\n\n{}", meal_name, description),
    }
}

// ── Meal Plan (full day) ─────────────────────────────────────────────────────

pub fn meal_plan_text(
    products: &[(IngredientData, &'static str, String)],
    meal_labels: &[&str],
    lang: ChatLang,
    goal: HealthGoal,
) -> String {
    let goal_label = match (lang, goal) {
        (ChatLang::Ru, HealthGoal::LowCalorie)  => "для похудения",
        (ChatLang::Ru, HealthGoal::HighProtein)  => "для набора массы",
        (ChatLang::Ru, HealthGoal::Balanced)     => "сбалансированный",
        (ChatLang::En, HealthGoal::LowCalorie)   => "for weight loss",
        (ChatLang::En, HealthGoal::HighProtein)   => "for muscle gain",
        (ChatLang::En, HealthGoal::Balanced)      => "balanced",
        (ChatLang::Pl, HealthGoal::LowCalorie)   => "na odchudzanie",
        (ChatLang::Pl, HealthGoal::HighProtein)   => "na masę",
        (ChatLang::Pl, HealthGoal::Balanced)      => "zbalansowany",
        (ChatLang::Uk, HealthGoal::LowCalorie)   => "для схуднення",
        (ChatLang::Uk, HealthGoal::HighProtein)   => "для набору маси",
        (ChatLang::Uk, HealthGoal::Balanced)      => "збалансований",
    };

    let header = match lang {
        ChatLang::Ru => format!("📋 **План питания на день** ({})\n", goal_label),
        ChatLang::En => format!("📋 **Daily Meal Plan** ({})\n", goal_label),
        ChatLang::Pl => format!("📋 **Plan na dzień** ({})\n", goal_label),
        ChatLang::Uk => format!("📋 **План на день** ({})\n", goal_label),
    };

    let mut lines = vec![header];
    for (i, (p, _, _)) in products.iter().enumerate() {
        let label = meal_labels.get(i).copied().unwrap_or("🍽️");
        let name = p.name(lang.code());
        let cal = p.calories_per_100g as i32;
        let pro = p.protein_per_100g;
        lines.push(format!("{}: **{}** — {} ккал · {:.0}г белка", label, name, cal * 2, pro * 2.0));
    }

    let total_cal: i32 = products.iter().map(|(p, _, _)| p.calories_per_100g as i32 * 2).sum();
    let total_pro: f32 = products.iter().map(|(p, _, _)| p.protein_per_100g * 2.0).sum();

    let footer = match lang {
        ChatLang::Ru => format!("\n**Итого: ~{} ккал · {:.0}г белка** (порции ~200г)", total_cal, total_pro),
        ChatLang::En => format!("\n**Total: ~{} kcal · {:.0}g protein** (~200g portions)", total_cal, total_pro),
        ChatLang::Pl => format!("\n**Razem: ~{} kcal · {:.0}g białka** (porcje ~200g)", total_cal, total_pro),
        ChatLang::Uk => format!("\n**Всього: ~{} ккал · {:.0}г білка** (порції ~200г)", total_cal, total_pro),
    };
    lines.push(footer);

    lines.join("\n")
}

// ── Already-seen product (anti-duplicate) ────────────────────────────────────

/// Text when user asks about a product they've already seen this session.
/// Shows brief confirmation + pivots to alternatives.
pub fn already_seen_text(
    name: &str,
    p: &IngredientData,
    alternatives: &[(IngredientData, &'static str, String)],
    lang: ChatLang,
    goal: HealthGoal,
) -> String {
    let cal = p.calories_per_100g as i32;
    let pro = p.protein_per_100g;

    let alt_names: Vec<String> = alternatives.iter()
        .map(|(a, _, _)| a.name(lang.code()).to_string())
        .collect();
    let alt_list = if alt_names.is_empty() {
        String::new()
    } else {
        alt_names.join(", ")
    };

    match (lang, goal) {
        (ChatLang::Ru, HealthGoal::LowCalorie) => {
            let base = format!("✅ **{}** — отличный выбор для похудения ({} ккал, {:.0}г белка).", name, cal, pro);
            if alt_list.is_empty() {
                format!("{}\n\n💡 Чередуй с рыбой и овощами — разнообразие ускоряет результат.", base)
            } else {
                format!("{}\n\nА вот что ещё поможет:\n🔀 Попробуй чередовать с **{}** — разнообразие ускоряет результат.", base, alt_list)
            }
        }
        (ChatLang::Ru, HealthGoal::HighProtein) => {
            let base = format!("✅ **{}** — мощный источник белка ({:.0}г на 100г).", name, pro);
            if alt_list.is_empty() {
                format!("{}\n\n💡 Комбинируй с крупами для полного аминокислотного профиля.", base)
            } else {
                format!("{}\n\nДля разнообразия добавь:\n🔀 **{}** — другие сильные источники белка.", base, alt_list)
            }
        }
        (ChatLang::Ru, HealthGoal::Balanced) => {
            let base = format!("✅ **{}** — сбалансированный выбор ({} ккал, {:.0}г белка).", name, cal, pro);
            if alt_list.is_empty() {
                format!("{}\n\n💡 Разнообразь рацион — каждый продукт даёт свой набор микроэлементов.", base)
            } else {
                format!("{}\n\nДля полноценного рациона добавь:\n🔀 **{}** — хорошо дополняют друг друга.", base, alt_list)
            }
        }
        (ChatLang::En, HealthGoal::LowCalorie) => {
            let base = format!("✅ **{}** — great choice for weight loss ({} kcal, {:.0}g protein).", name, cal, pro);
            if alt_list.is_empty() {
                format!("{}\n\n💡 Rotate with fish and veggies — variety boosts results.", base)
            } else {
                format!("{}\n\nAlso consider:\n🔀 Try alternating with **{}** — variety boosts results.", base, alt_list)
            }
        }
        (ChatLang::En, HealthGoal::HighProtein) => {
            let base = format!("✅ **{}** — powerful protein source ({:.0}g per 100g).", name, pro);
            if alt_list.is_empty() {
                format!("{}\n\n💡 Pair with grains for a complete amino acid profile.", base)
            } else {
                format!("{}\n\nFor variety, add:\n🔀 **{}** — other strong protein sources.", base, alt_list)
            }
        }
        (ChatLang::En, HealthGoal::Balanced) => {
            let base = format!("✅ **{}** — balanced pick ({} kcal, {:.0}g protein).", name, cal, pro);
            if alt_list.is_empty() {
                format!("{}\n\n💡 Diversify your meals — each product brings unique micronutrients.", base)
            } else {
                format!("{}\n\nTo round out your diet:\n🔀 **{}** — they complement each other well.", base, alt_list)
            }
        }
        (ChatLang::Pl, _) => {
            let base = format!("✅ **{}** — dobry wybór ({} kcal, {:.0}g białka).", name, cal, pro);
            if alt_list.is_empty() {
                format!("{}\n\n💡 Urozmaicaj dietę — każdy produkt wnosi inne składniki.", base)
            } else {
                format!("{}\n\nDla urozmaicenia:\n🔀 **{}** — dobrze się uzupełniają.", base, alt_list)
            }
        }
        (ChatLang::Uk, _) => {
            let base = format!("✅ **{}** — хороший вибір ({} ккал, {:.0}г білка).", name, cal, pro);
            if alt_list.is_empty() {
                format!("{}\n\n💡 Урізноманітнюй раціон — кожен продукт дає свої мікроелементи.", base)
            } else {
                format!("{}\n\nДля різноманіття:\n🔀 **{}** — добре доповнюють один одного.", base, alt_list)
            }
        }
    }
}

/// Explainability reason for "already seen" response.
pub fn already_seen_reason(
    name: &str,
    alternatives: &[(IngredientData, &'static str, String)],
    lang: ChatLang,
    _goal: HealthGoal,
) -> String {
    let alt_count = alternatives.len();
    match lang {
        ChatLang::Ru => format!("{} уже показан → предлагаю {} альтернатив(ы)", name, alt_count),
        ChatLang::En => format!("{} already shown → suggesting {} alternative(s)", name, alt_count),
        ChatLang::Pl => format!("{} już pokazany → proponuję {} alternatyw(y)", name, alt_count),
        ChatLang::Uk => format!("{} вже показано → пропоную {} альтернатив(и)", name, alt_count),
    }
}

/// Chef tip for "already seen" — different tip from the first time.
pub fn chef_tip_alternative(p: &IngredientData, lang: ChatLang, goal: HealthGoal) -> String {
    let slug = p.slug.to_lowercase();
    let cal = p.calories_per_100g as i32;
    let pro = p.protein_per_100g;

    // Different tips from chef_tip() — focus on meal planning & combos
    let tip: (&str, &str, &str, &str) = if slug.contains("chicken") {
        (
            "Курица + гречка + брокколи = идеальная тарелка: 40г белка, <500 ккал. Готовь batch на 3 дня.",
            "Chicken + buckwheat + broccoli = perfect plate: 40g protein, <500 kcal. Meal prep for 3 days.",
            "Kurczak + kasza gryczana + brokuły = idealny talerz: 40g białka, <500 kcal.",
            "Курка + гречка + броколі = ідеальна тарілка: 40г білка, <500 ккал.",
        )
    } else if slug.contains("salmon") {
        (
            "Лосось + авокадо + рис = полный спектр: омега-3, клетчатка, сложные углеводы.",
            "Salmon + avocado + rice = full spectrum: omega-3, fiber, complex carbs.",
            "Łosoś + awokado + ryż = pełne spektrum: omega-3, błonnik, złożone węglowodany.",
            "Лосось + авокадо + рис = повний спектр: омега-3, клітковина, складні вуглеводи.",
        )
    } else if slug.contains("egg") {
        (
            "Яйца + шпинат + тост = завтрак чемпиона: 25г белка, железо, клетчатка за 10 минут.",
            "Eggs + spinach + toast = champion's breakfast: 25g protein, iron, fiber in 10 min.",
            "Jajka + szpinak + tost = śniadanie mistrza: 25g białka, żelazo, błonnik w 10 min.",
            "Яйця + шпинат + тост = сніданок чемпіона: 25г білка, залізо, клітковина за 10 хв.",
        )
    } else if pro >= 20.0 {
        (
            &format!("Высокобелковые продукты лучше распределять по дню: по 30г белка за приём — оптимально для усвоения."),
            "Spread high-protein foods across meals: ~30g per meal is optimal for absorption.",
            "Rozłóż białko na cały dzień: ~30g na posiłek — optymalnie dla wchłaniania.",
            "Розподіляй білок по дню: ~30г на прийом — оптимально для засвоєння.",
        )
    } else if cal < 100 {
        (
            "Низкокалорийные продукты — основа объёма. Добавь белковый источник, чтобы не проголодаться.",
            "Low-cal foods are great for volume. Add a protein source to stay full.",
            "Niskokaloryczne produkty dają objętość. Dodaj źródło białka, żeby się najeść.",
            "Низькокалорійні продукти — основа об'єму. Додай білок, щоб не проголодатися.",
        )
    } else {
        (
            "Разнообразие — ключ к полноценному питанию. Каждый продукт несёт свой набор витаминов.",
            "Variety is key to complete nutrition. Each product brings its own vitamin set.",
            "Różnorodność to klucz. Każdy produkt wnosi swój zestaw witamin.",
            "Різноманіття — ключ до повноцінного харчування. Кожен продукт несе свої вітаміни.",
        )
    };

    match lang {
        ChatLang::Ru => format!("💡 Шеф-совет: {}", tip.0),
        ChatLang::En => format!("💡 Chef tip: {}", tip.1),
        ChatLang::Pl => format!("💡 Rada szefa: {}", tip.2),
        ChatLang::Uk => format!("💡 Порада шефа: {}", tip.3),
    }
}

// ── Product info fallback ────────────────────────────────────────────────────

pub fn product_not_found(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "Продукт не найден в базе. Попробуй уточнить название.",
        ChatLang::En => "Product not found in database. Try rephrasing the name.",
        ChatLang::Pl => "Produkt nie znaleziony. Spróbuj innej nazwy.",
        ChatLang::Uk => "Продукт не знайдено. Спробуй уточнити назву.",
    }
}

// ── Chef tips ────────────────────────────────────────────────────────────────

pub fn chef_tip(p: &IngredientData, lang: ChatLang, goal: HealthGoal) -> String {
    let slug = p.slug.to_lowercase();

    let tip_seed = {
        use std::time::{SystemTime, UNIX_EPOCH};
        (SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() / 10) as usize
    };

    // ── Product-specific tips ──
    let product_tip: Option<(&str, &str, &str, &str)> = if slug.contains("chicken") {
        Some((
            "Запекай курицу без кожи — минус ~120 ккал на порцию. Мясо останется сочным, если накрыть фольгой.",
            "Bake chicken without skin — minus ~120 kcal per serving. Cover with foil to keep it juicy.",
            "Piecz kurczaka bez skóry — minus ~120 kcal na porcję. Przykryj folią, żeby był soczysty.",
            "Запікай курку без шкіри — мінус ~120 ккал на порцію. Накрий фольгою для соковитості.",
        ))
    } else if slug.contains("salmon") {
        Some((
            "Лосось на пару за 12 минут — сохраняет омега-3 и текстуру. Жарка разрушает до 30% жирных кислот.",
            "Steam salmon for 12 min — preserves omega-3 and texture. Frying destroys up to 30% of fatty acids.",
            "Łosoś na parze 12 min — zachowuje omega-3 i teksturę. Smażenie niszczy do 30% kwasów tłuszczowych.",
            "Лосось на парі 12 хв — зберігає омега-3. Смаження руйнує до 30% жирних кислот.",
        ))
    } else if slug.contains("egg") {
        Some((
            "Варёное яйцо — 78 ккал. Жареное — 120+. Разница в масле, а не в яйце.",
            "Boiled egg: 78 kcal. Fried: 120+. The difference is the oil, not the egg.",
            "Jajko gotowane: 78 kcal. Smażone: 120+. Różnica w oleju, nie w jajku.",
            "Варене яйце — 78 ккал. Смажене — 120+. Різниця в олії, а не в яйці.",
        ))
    } else if slug.contains("spinach") {
        Some((
            "Шпинат теряет 50% объёма при готовке — клади в 2 раза больше, чем кажется нужным.",
            "Spinach loses 50% volume when cooked — use 2x more than you think you need.",
            "Szpinak traci 50% objętości przy gotowaniu — daj 2x więcej niż ci się wydaje.",
            "Шпинат втрачає 50% об'єму при готуванні — клади вдвічі більше.",
        ))
    } else if slug.contains("broccoli") {
        Some((
            "Брокколи на пару 5 мин — максимум витамина C. Если варить дольше — теряешь до 60%.",
            "Steam broccoli for 5 min — maximum vitamin C. Boiling longer loses up to 60%.",
            "Brokuły na parze 5 min — maks. witaminy C. Dłuższe gotowanie traci do 60%.",
            "Броколі на парі 5 хв — максимум вітаміну C. Довше варити — мінус 60%.",
        ))
    } else if slug.contains("tuna") {
        Some((
            "Тунец из банки в собственном соку — 100 ккал. В масле — 200+. Всегда выбирай «в соку».",
            "Canned tuna in water: 100 kcal. In oil: 200+. Always pick water-packed.",
            "Tuńczyk w wodzie: 100 kcal. W oleju: 200+. Zawsze wybieraj w sosie własnym.",
            "Тунець у власному соці — 100 ккал. В олії — 200+. Завжди обирай «у соці».",
        ))
    } else if slug.contains("almond") {
        Some((
            "Миндаль — 30г (горсть) = ~170 ккал. Легко переесть. Отмеряй порцию заранее.",
            "Almonds — 30g (handful) = ~170 kcal. Easy to overeat. Pre-measure your portion.",
            "Migdały — 30g (garść) = ~170 kcal. Łatwo zjeść za dużo. Odmierz porcję z góry.",
            "Мигдаль — 30г (жменька) = ~170 ккал. Легко переїсти. Відміряй порцію заздалегідь.",
        ))
    } else if slug.contains("rice") {
        Some((
            "Охлаждённый рис содержит резистентный крахмал — меньше калорий усваивается. Приготовь заранее.",
            "Cooled rice contains resistant starch — fewer calories absorbed. Cook it ahead.",
            "Schłodzony ryż zawiera skrobię oporną — mniej kalorii się wchłania. Ugotuj wcześniej.",
            "Охолоджений рис містить резистентний крохмаль — менше калорій засвоюється.",
        ))
    } else if slug.contains("beef") {
        Some((
            "Говядина: дай стейку отдохнуть 5 мин — соки перераспределятся, мясо будет нежнее на 40%.",
            "Beef: let the steak rest 5 min — juices redistribute, 40% more tender.",
            "Wołowina: daj stekowi odpocząć 5 min — soki się rozprowadzą, mięso o 40% delikatniejsze.",
            "Яловичина: дай стейку відпочити 5 хв — соки розподіляться, м'ясо ніжніше на 40%.",
        ))
    } else if slug.contains("blueberr") {
        Some((
            "Замороженная черника сохраняет 95% антиоксидантов — не хуже свежей, а дешевле в 3 раза.",
            "Frozen blueberries retain 95% of antioxidants — as good as fresh, 3x cheaper.",
            "Mrożone jagody zachowują 95% antyoksydantów — tak dobre jak świeże, 3x tańsze.",
            "Заморожена чорниця зберігає 95% антиоксидантів — не гірша за свіжу, а дешевша в 3 рази.",
        ))
    } else {
        None
    };

    if let Some((ru, en, pl, uk)) = product_tip {
        return match lang {
            ChatLang::Ru => format!("💡 Шеф-совет: {}", ru),
            ChatLang::En => format!("💡 Chef tip: {}", en),
            ChatLang::Pl => format!("💡 Rada szefa: {}", pl),
            ChatLang::Uk => format!("💡 Порада шефа: {}", uk),
        };
    }

    // ── Macro-based fallback tips ──
    let high_protein = p.protein_per_100g >= 20.0;
    let low_cal = p.calories_per_100g < 80.0;
    let high_fat = p.fat_per_100g >= 15.0;
    let is_meat = p.protein_per_100g >= 18.0 && p.fat_per_100g >= 3.0;
    let is_veggie = p.calories_per_100g < 50.0 && p.protein_per_100g < 5.0;

    let tips: Vec<(&str, &str, &str, &str)> = if is_meat {
        vec![
            ("Готовь мясо на решётке или в духовке — жир стечёт, минус ~100 ккал.",
             "Cook meat on a rack or in the oven — fat drips off, minus ~100 kcal.",
             "Piecz mięso na ruszcie — tłuszcz ścieka, minus ~100 kcal.",
             "Готуй м'ясо на решітці — жир стече, мінус ~100 ккал."),
            ("Маринуй в лимонном соке + травы — вкуснее и мягче без масла.",
             "Marinate in lemon juice + herbs — tastier and tender without oil.",
             "Marynuj w soku z cytryny + zioła — smaczniej i delikatniej bez oleju.",
             "Маринуй в лимонному соці + трави — смачніше без олії."),
            ("Дай мясу «отдохнуть» 5 мин после готовки — соки распределятся.",
             "Let meat rest 5 min after cooking — juices redistribute evenly.",
             "Daj mięsu odpocząć 5 min — soki się rozprowadzą.",
             "Дай м'ясу «відпочити» 5 хв — соки розподіляться."),
        ]
    } else if is_veggie {
        vec![
            ("Овощи аль-денте сохраняют витамины. Не переваривай — 3-5 мин на пару достаточно.",
             "Al dente veggies keep their vitamins. Don't overcook — 3-5 min steaming is enough.",
             "Warzywa al dente zachowują witaminy. Nie rozgotowuj — 3-5 min na parze wystarczy.",
             "Овочі аль-денте зберігають вітаміни. Не перевар — 3-5 хв на парі достатньо."),
            ("Заправляй лимонным соком вместо масла — минус 100 ккал, плюс витамин C.",
             "Use lemon juice instead of oil — minus 100 kcal, plus vitamin C.",
             "Zamiast oleju — sok z cytryny — minus 100 kcal, plus witamina C.",
             "Замість олії — лимонний сік — мінус 100 ккал, плюс вітамін C."),
            ("Запекай вместо варки — карамелизация даёт вкус без калорий.",
             "Roast instead of boiling — caramelization adds flavor without calories.",
             "Piecz zamiast gotować — karmelizacja daje smak bez kalorii.",
             "Запікай замість варки — карамелізація дає смак без калорій."),
        ]
    } else if high_protein && matches!(goal, HealthGoal::HighProtein) {
        vec![
            ("Готовь на пару — сохраняет до 95% белка, в отличие от жарки.",
             "Steam instead of frying — preserves up to 95% of protein.",
             "Gotuj na parze — zachowuje do 95% białka.",
             "Готуй на парі — зберігає до 95% білка."),
            ("Сочетай с бобовыми — получишь полный аминокислотный профиль.",
             "Pair with legumes for a complete amino acid profile.",
             "Połącz z roślinami strączkowymi — pełny profil aminokwasów.",
             "Поєднуй з бобовими — повний амінокислотний профіль."),
        ]
    } else if high_fat {
        vec![
            ("Калорийный продукт — используй как усилитель вкуса, не как основу.",
             "Calorie-dense — use as a flavor booster, not the main course.",
             "Kaloryczny produkt — używaj jako wzmacniacz smaku, nie podstawę.",
             "Калорійний продукт — використовуй як підсилювач смаку."),
            ("Отмеряй порцию заранее — легко переесть на 200+ ккал.",
             "Pre-measure your portion — easy to overeat by 200+ kcal.",
             "Odmierz porcję z góry — łatwo zjeść za dużo.",
             "Відміряй порцію заздалегідь — легко переїсти на 200+ ккал."),
        ]
    } else if low_cal && matches!(goal, HealthGoal::LowCalorie) {
        vec![
            ("Запекай вместо жарки — экономишь ~80 ккал на порцию.",
             "Bake instead of frying — saves ~80 kcal per serving.",
             "Piecz zamiast smażyć — oszczędzasz ~80 kcal na porcję.",
             "Запікай замість смаження — економиш ~80 ккал на порцію."),
            ("Ешь медленнее — насыщение приходит через 20 минут.",
             "Eat slowly — fullness takes 20 minutes to kick in.",
             "Jedz wolniej — sytość przychodzi po 20 minutach.",
             "Їж повільніше — ситість приходить через 20 хвилин."),
        ]
    } else {
        vec![
            ("Свежие специи (базилик, кинза) добавляй в конце — так ярче вкус.",
             "Add fresh herbs (basil, cilantro) at the end for brighter flavor.",
             "Świeże zioła (bazylia, kolendra) dodawaj na końcu — smak będzie żywszy.",
             "Свіжі спеції додавай в кінці — так яскравіший смак."),
            ("Пробуй новые способы готовки: пар, гриль, запекание — каждый раскрывает продукт по-разному.",
             "Try different cooking methods: steam, grill, roast — each reveals the product differently.",
             "Wypróbuj różne metody: para, grill, pieczenie — każda wydobywa inny smak.",
             "Спробуй різні способи: пара, гриль, запікання — кожен розкриває продукт інакше."),
        ]
    };

    let (ru, en, pl, uk) = &tips[tip_seed % tips.len()];
    match lang {
        ChatLang::Ru => format!("💡 Шеф-совет: {}", ru),
        ChatLang::En => format!("💡 Chef tip: {}", en),
        ChatLang::Pl => format!("💡 Rada szefa: {}", pl),
        ChatLang::Uk => format!("💡 Порада шефа: {}", uk),
    }
}

// ── Meal Combo ───────────────────────────────────────────────────────────────

/// Localized text for a dynamically assembled meal combo.
pub fn meal_combo_text(combo: &MealCombo, lang: ChatLang, goal: HealthGoal) -> String {
    let pname = match lang {
        ChatLang::Ru => &combo.protein.name_ru,
        ChatLang::En => &combo.protein.name_en,
        ChatLang::Pl => &combo.protein.name_pl,
        ChatLang::Uk => &combo.protein.name_uk,
    };
    let sname = match lang {
        ChatLang::Ru => &combo.side.name_ru,
        ChatLang::En => &combo.side.name_en,
        ChatLang::Pl => &combo.side.name_pl,
        ChatLang::Uk => &combo.side.name_uk,
    };
    let bname = combo.base.as_ref().map(|b| match lang {
        ChatLang::Ru => &b.name_ru,
        ChatLang::En => &b.name_en,
        ChatLang::Pl => &b.name_pl,
        ChatLang::Uk => &b.name_uk,
    });

    let goal_label = match (lang, goal) {
        (ChatLang::Ru, HealthGoal::HighProtein) => "💪 Высокобелковый",
        (ChatLang::Ru, HealthGoal::LowCalorie)  => "🥗 Низкокалорийный",
        (ChatLang::Ru, HealthGoal::Balanced)     => "⚖️ Сбалансированный",
        (ChatLang::En, HealthGoal::HighProtein) => "💪 High-Protein",
        (ChatLang::En, HealthGoal::LowCalorie)  => "🥗 Low-Calorie",
        (ChatLang::En, HealthGoal::Balanced)     => "⚖️ Balanced",
        (ChatLang::Pl, HealthGoal::HighProtein) => "💪 Wysokobiałkowy",
        (ChatLang::Pl, HealthGoal::LowCalorie)  => "🥗 Niskokaloryczny",
        (ChatLang::Pl, HealthGoal::Balanced)     => "⚖️ Zbilansowany",
        (ChatLang::Uk, HealthGoal::HighProtein) => "💪 Високобілковий",
        (ChatLang::Uk, HealthGoal::LowCalorie)  => "🥗 Низькокалорійний",
        (ChatLang::Uk, HealthGoal::Balanced)     => "⚖️ Збалансований",
    };

    let combo_parts = if let Some(bn) = bname {
        format!("{} + {} + {}", pname, sname, bn)
    } else {
        format!("{} + {}", pname, sname)
    };

    let portions = if combo.base.is_some() {
        match lang {
            ChatLang::Ru => format!("({}г + {}г + {}г)", combo.protein_g as u32, combo.side_g as u32, combo.base_g as u32),
            ChatLang::En => format!("({}g + {}g + {}g)", combo.protein_g as u32, combo.side_g as u32, combo.base_g as u32),
            ChatLang::Pl => format!("({}g + {}g + {}g)", combo.protein_g as u32, combo.side_g as u32, combo.base_g as u32),
            ChatLang::Uk => format!("({}г + {}г + {}г)", combo.protein_g as u32, combo.side_g as u32, combo.base_g as u32),
        }
    } else {
        match lang {
            ChatLang::Ru => format!("({}г + {}г)", combo.protein_g as u32, combo.side_g as u32),
            ChatLang::En => format!("({}g + {}g)", combo.protein_g as u32, combo.side_g as u32),
            ChatLang::Pl => format!("({}g + {}g)", combo.protein_g as u32, combo.side_g as u32),
            ChatLang::Uk => format!("({}г + {}г)", combo.protein_g as u32, combo.side_g as u32),
        }
    };

    let stats = match lang {
        ChatLang::Ru => format!(
            "📊 {} ккал • белок {:.0}г • жиры {:.0}г • углеводы {:.0}г",
            combo.total_kcal, combo.total_protein, combo.total_fat, combo.total_carbs
        ),
        ChatLang::En => format!(
            "📊 {} kcal • protein {:.0}g • fat {:.0}g • carbs {:.0}g",
            combo.total_kcal, combo.total_protein, combo.total_fat, combo.total_carbs
        ),
        ChatLang::Pl => format!(
            "📊 {} kcal • białko {:.0}g • tłuszcze {:.0}g • węglowodany {:.0}g",
            combo.total_kcal, combo.total_protein, combo.total_fat, combo.total_carbs
        ),
        ChatLang::Uk => format!(
            "📊 {} ккал • білок {:.0}г • жири {:.0}г • вуглеводи {:.0}г",
            combo.total_kcal, combo.total_protein, combo.total_fat, combo.total_carbs
        ),
    };

    format!(
        "🍽 {goal_label}\n\n{combo_parts} {portions}\n\n{stats}"
    )
}
