use crate::shared::Language;

// ── Season translations ───────────────────────────────────────────────────────

/// Translate a raw season code (e.g. "AllYear", "Spring") to a human-readable label.
pub fn translate_season(season: &str, lang: Language) -> String {
    let label: &str = match season.to_lowercase().replace(['-', '_', ' '], "").as_str() {
        "allyear" | "all_year" | "allyears" => match lang {
            Language::En => "all year",
            Language::Pl => "cały rok",
            Language::Ru => "круглый год",
            Language::Uk => "цілий рік",
        },
        "spring" => match lang {
            Language::En => "spring",
            Language::Pl => "wiosna",
            Language::Ru => "весна",
            Language::Uk => "весна",
        },
        "summer" => match lang {
            Language::En => "summer",
            Language::Pl => "lato",
            Language::Ru => "лето",
            Language::Uk => "літо",
        },
        "autumn" | "fall" => match lang {
            Language::En => "autumn",
            Language::Pl => "jesień",
            Language::Ru => "осень",
            Language::Uk => "осінь",
        },
        "winter" => match lang {
            Language::En => "winter",
            Language::Pl => "zima",
            Language::Ru => "зима",
            Language::Uk => "зима",
        },
        "january" => match lang {
            Language::En => "January",
            Language::Pl => "styczeń",
            Language::Ru => "январь",
            Language::Uk => "січень",
        },
        "february" => match lang {
            Language::En => "February",
            Language::Pl => "luty",
            Language::Ru => "февраль",
            Language::Uk => "лютий",
        },
        "march" => match lang {
            Language::En => "March",
            Language::Pl => "marzec",
            Language::Ru => "март",
            Language::Uk => "березень",
        },
        "april" => match lang {
            Language::En => "April",
            Language::Pl => "kwiecień",
            Language::Ru => "апрель",
            Language::Uk => "квітень",
        },
        "may" => match lang {
            Language::En => "May",
            Language::Pl => "maj",
            Language::Ru => "май",
            Language::Uk => "травень",
        },
        "june" => match lang {
            Language::En => "June",
            Language::Pl => "czerwiec",
            Language::Ru => "июнь",
            Language::Uk => "червень",
        },
        "july" => match lang {
            Language::En => "July",
            Language::Pl => "lipiec",
            Language::Ru => "июль",
            Language::Uk => "липень",
        },
        "august" => match lang {
            Language::En => "August",
            Language::Pl => "sierpień",
            Language::Ru => "август",
            Language::Uk => "серпень",
        },
        "september" => match lang {
            Language::En => "September",
            Language::Pl => "wrzesień",
            Language::Ru => "сентябрь",
            Language::Uk => "вересень",
        },
        "october" => match lang {
            Language::En => "October",
            Language::Pl => "październik",
            Language::Ru => "октябрь",
            Language::Uk => "жовтень",
        },
        "november" => match lang {
            Language::En => "November",
            Language::Pl => "listopad",
            Language::Ru => "ноябрь",
            Language::Uk => "листопад",
        },
        "december" => match lang {
            Language::En => "December",
            Language::Pl => "grudzień",
            Language::Ru => "декабрь",
            Language::Uk => "грудень",
        },
        _ => return season.to_string(),
    };
    label.to_string()
}

/// Translate a list of raw season codes
pub fn translate_seasons(seasons: &[String], lang: Language) -> Vec<String> {
    seasons
        .iter()
        .map(|s| translate_season(s, lang).to_string())
        .collect()
}

// ── Allergen translations ─────────────────────────────────────────────────────

pub fn translate_allergen(allergen: &str, lang: Language) -> String {
    let label: &str = match allergen.to_lowercase().as_str() {
        "fish" => match lang {
            Language::En => "fish",
            Language::Pl => "ryby",
            Language::Ru => "рыба",
            Language::Uk => "риба",
        },
        "milk" | "dairy" => match lang {
            Language::En => "milk",
            Language::Pl => "mleko",
            Language::Ru => "молоко",
            Language::Uk => "молоко",
        },
        "eggs" | "egg" => match lang {
            Language::En => "eggs",
            Language::Pl => "jaja",
            Language::Ru => "яйца",
            Language::Uk => "яйця",
        },
        "wheat" | "gluten" => match lang {
            Language::En => "wheat / gluten",
            Language::Pl => "pszenica / gluten",
            Language::Ru => "пшеница / глютен",
            Language::Uk => "пшениця / глютен",
        },
        "nuts" | "treenuts" => match lang {
            Language::En => "tree nuts",
            Language::Pl => "orzechy",
            Language::Ru => "орехи",
            Language::Uk => "горіхи",
        },
        "peanuts" => match lang {
            Language::En => "peanuts",
            Language::Pl => "orzeszki ziemne",
            Language::Ru => "арахис",
            Language::Uk => "арахіс",
        },
        "soy" | "soya" => match lang {
            Language::En => "soy",
            Language::Pl => "soja",
            Language::Ru => "соя",
            Language::Uk => "соя",
        },
        "shellfish" => match lang {
            Language::En => "shellfish",
            Language::Pl => "skorupiaki",
            Language::Ru => "ракообразные",
            Language::Uk => "ракоподібні",
        },
        "sesame" => match lang {
            Language::En => "sesame",
            Language::Pl => "sezam",
            Language::Ru => "кунжут",
            Language::Uk => "кунжут",
        },
        "mustard" => match lang {
            Language::En => "mustard",
            Language::Pl => "gorczyca",
            Language::Ru => "горчица",
            Language::Uk => "гірчиця",
        },
        _ => return allergen.to_string(),
    };
    label.to_string()
}

pub fn translate_allergens(allergens: &[String], lang: Language) -> Vec<String> {
    allergens
        .iter()
        .map(|a| translate_allergen(a, lang).to_string())
        .collect()
}



/// Message keys for assistant messages
#[derive(Debug, Clone, Copy)]
pub enum AssistantMessage {
    Welcome,
    InventorySetup,
    RecipeSetup,
    DishSetup,
    Report,
    Completed,
}

/// Action label keys for assistant actions
#[derive(Debug, Clone, Copy)]
pub enum AssistantActionLabel {
    AddProducts,
    AddMoreProducts,
    ProceedToRecipes,
    AddRecipe,
    ProceedToDishes,
    AddDish,
    GenerateReport,
    ViewReport,
    Finish,
    Restart,
}

/// Hint keys for contextual help
#[derive(Debug, Clone, Copy)]
pub enum AssistantHint {
    InventoryWhy,
    RecipeWhy,
    DishWhy,
    ReportWhy,
}

/// Translate an assistant message to the appropriate language
pub fn translate_message(key: AssistantMessage, language: Language) -> &'static str {
    match language {
        Language::En => translate_message_en(key),
        Language::Pl => translate_message_pl(key),
        Language::Uk => translate_message_uk(key),
        Language::Ru => translate_message_ru(key),
        #[allow(unreachable_patterns)]
        _ => translate_message_en(key), // Fallback to English
    }
}

/// Translate an action label to the appropriate language
pub fn translate_action(key: AssistantActionLabel, language: Language) -> &'static str {
    match language {
        Language::En => translate_action_en(key),
        Language::Pl => translate_action_pl(key),
        Language::Uk => translate_action_uk(key),
        Language::Ru => translate_action_ru(key),
        #[allow(unreachable_patterns)]
        _ => translate_action_en(key), // Fallback to English
    }
}

/// Translate a hint to the appropriate language
pub fn translate_hint(key: AssistantHint, language: Language) -> &'static str {
    match language {
        Language::En => translate_hint_en(key),
        Language::Pl => translate_hint_pl(key),
        Language::Uk => translate_hint_uk(key),
        Language::Ru => translate_hint_ru(key),
        #[allow(unreachable_patterns)]
        _ => translate_hint_en(key), // Fallback to English
    }
}

// ============================================================================
// English translations
// ============================================================================

fn translate_message_en(key: AssistantMessage) -> &'static str {
    match key {
        AssistantMessage::Welcome => "Welcome! Let's start by adding your products to the inventory.",
        AssistantMessage::InventorySetup => "Great! Add your products to the inventory. You can add more products or proceed to creating recipes.",
        AssistantMessage::RecipeSetup => "Now let's create recipes from your products. Add recipes or proceed to creating dishes.",
        AssistantMessage::DishSetup => "Time to create dishes for your menu! Add dishes or generate a report.",
        AssistantMessage::Report => "Your report is ready! Review your inventory, recipes, and dishes.",
        AssistantMessage::Completed => "Setup completed! Your restaurant is ready to operate.",
    }
}

fn translate_action_en(key: AssistantActionLabel) -> &'static str {
    match key {
        AssistantActionLabel::AddProducts => "📦 Add Products",
        AssistantActionLabel::AddMoreProducts => "➕ Add More Products",
        AssistantActionLabel::ProceedToRecipes => "📖 Create Recipes",
        AssistantActionLabel::AddRecipe => "🍳 Add Recipe",
        AssistantActionLabel::ProceedToDishes => "🍽️ Create Menu Dishes",
        AssistantActionLabel::AddDish => "✨ Add Dish",
        AssistantActionLabel::GenerateReport => "📊 Generate Report",
        AssistantActionLabel::ViewReport => "👁️ View Report",
        AssistantActionLabel::Finish => "✅ Finish Setup",
        AssistantActionLabel::Restart => "🔄 Start Over",
    }
}

fn translate_hint_en(key: AssistantHint) -> &'static str {
    match key {
        AssistantHint::InventoryWhy => {
            "Having a complete inventory helps track costs and prevent shortages."
        }
        AssistantHint::RecipeWhy => {
            "Recipes standardize preparation and calculate exact ingredient costs."
        }
        AssistantHint::DishWhy => {
            "Menu dishes combine recipes with pricing to calculate profit margins."
        }
        AssistantHint::ReportWhy => {
            "The report shows your business metrics and identifies opportunities."
        }
    }
}

// ============================================================================
// Polish translations
// ============================================================================

fn translate_message_pl(key: AssistantMessage) -> &'static str {
    match key {
        AssistantMessage::Welcome => "Witamy! Zacznijmy od dodania produktów do magazynu.",
        AssistantMessage::InventorySetup => "Świetnie! Dodaj produkty do magazynu. Możesz dodać więcej produktów lub przejść do tworzenia przepisów.",
        AssistantMessage::RecipeSetup => "Teraz stwórzmy przepisy z Twoich produktów. Dodaj przepisy lub przejdź do tworzenia dań.",
        AssistantMessage::DishSetup => "Czas stworzyć dania do menu! Dodaj dania lub wygeneruj raport.",
        AssistantMessage::Report => "Twój raport jest gotowy! Przejrzyj magazyn, przepisy i dania.",
        AssistantMessage::Completed => "Konfiguracja zakończona! Twoja restauracja jest gotowa do pracy.",
    }
}

fn translate_action_pl(key: AssistantActionLabel) -> &'static str {
    match key {
        AssistantActionLabel::AddProducts => "📦 Dodaj Produkty",
        AssistantActionLabel::AddMoreProducts => "➕ Dodaj Więcej Produktów",
        AssistantActionLabel::ProceedToRecipes => "📖 Utwórz Przepisy",
        AssistantActionLabel::AddRecipe => "🍳 Dodaj Przepis",
        AssistantActionLabel::ProceedToDishes => "🍽️ Utwórz Dania Menu",
        AssistantActionLabel::AddDish => "✨ Dodaj Danie",
        AssistantActionLabel::GenerateReport => "📊 Wygeneruj Raport",
        AssistantActionLabel::ViewReport => "👁️ Zobacz Raport",
        AssistantActionLabel::Finish => "✅ Zakończ Konfigurację",
        AssistantActionLabel::Restart => "🔄 Zacznij Od Nowa",
    }
}

fn translate_hint_pl(key: AssistantHint) -> &'static str {
    match key {
        AssistantHint::InventoryWhy => {
            "Kompletny magazyn pomaga śledzić koszty i zapobiegać brakom."
        }
        AssistantHint::RecipeWhy => {
            "Przepisy standaryzują przygotowanie i obliczają dokładne koszty składników."
        }
        AssistantHint::DishWhy => "Dania menu łączą przepisy z cenami, obliczając marże zysku.",
        AssistantHint::ReportWhy => "Raport pokazuje metryki biznesowe i identyfikuje możliwości.",
    }
}

// ============================================================================
// Ukrainian translations
// ============================================================================

fn translate_message_uk(key: AssistantMessage) -> &'static str {
    match key {
        AssistantMessage::Welcome => "Ласкаво просимо! Почнемо з додавання продуктів до складу.",
        AssistantMessage::InventorySetup => "Чудово! Додайте продукти до складу. Ви можете додати більше продуктів або перейти до створення рецептів.",
        AssistantMessage::RecipeSetup => "Тепер створимо рецепти з ваших продуктів. Додайте рецепти або перейдіть до створення страв.",
        AssistantMessage::DishSetup => "Час створити страви для меню! Додайте страви або згенеруйте звіт.",
        AssistantMessage::Report => "Ваш звіт готовий! Перегляньте склад, рецепти та страви.",
        AssistantMessage::Completed => "Налаштування завершено! Ваш ресторан готовий до роботи.",
    }
}

fn translate_action_uk(key: AssistantActionLabel) -> &'static str {
    match key {
        AssistantActionLabel::AddProducts => "📦 Додати Продукти",
        AssistantActionLabel::AddMoreProducts => "➕ Додати Більше Продуктів",
        AssistantActionLabel::ProceedToRecipes => "📖 Створити Рецепти",
        AssistantActionLabel::AddRecipe => "🍳 Додати Рецепт",
        AssistantActionLabel::ProceedToDishes => "🍽️ Створити Страви Меню",
        AssistantActionLabel::AddDish => "✨ Додати Страву",
        AssistantActionLabel::GenerateReport => "📊 Згенерувати Звіт",
        AssistantActionLabel::ViewReport => "👁️ Переглянути Звіт",
        AssistantActionLabel::Finish => "✅ Завершити Налаштування",
        AssistantActionLabel::Restart => "🔄 Почати Спочатку",
    }
}

fn translate_hint_uk(key: AssistantHint) -> &'static str {
    match key {
        AssistantHint::InventoryWhy => {
            "Повний склад допомагає відстежувати витрати та запобігати дефіциту."
        }
        AssistantHint::RecipeWhy => {
            "Рецепти стандартизують приготування та розраховують точну вартість інгредієнтів."
        }
        AssistantHint::DishWhy => {
            "Страви меню поєднують рецепти з цінами для розрахунку маржі прибутку."
        }
        AssistantHint::ReportWhy => "Звіт показує бізнес-метрики та виявляє можливості.",
    }
}

// ============================================================================
// Russian translations
// ============================================================================

fn translate_message_ru(key: AssistantMessage) -> &'static str {
    match key {
        AssistantMessage::Welcome => "Добро пожаловать! Давайте начнём с добавления продуктов на склад.",
        AssistantMessage::InventorySetup => "Отлично! Добавьте продукты на склад. Вы можете добавить ещё продукты или перейти к созданию рецептов.",
        AssistantMessage::RecipeSetup => "Теперь создадим рецепты из ваших продуктов. Добавьте рецепты или перейдите к созданию блюд.",
        AssistantMessage::DishSetup => "Время создать блюда для меню! Добавьте блюда или сгенерируйте отчёт.",
        AssistantMessage::Report => "Ваш отчёт готов! Просмотрите склад, рецепты и блюда.",
        AssistantMessage::Completed => "Настройка завершена! Ваш ресторан готов к работе.",
    }
}

fn translate_action_ru(key: AssistantActionLabel) -> &'static str {
    match key {
        AssistantActionLabel::AddProducts => "📦 Добавить Продукты",
        AssistantActionLabel::AddMoreProducts => "➕ Добавить Ещё Продукты",
        AssistantActionLabel::ProceedToRecipes => "📖 Создать Рецепты",
        AssistantActionLabel::AddRecipe => "🍳 Добавить Рецепт",
        AssistantActionLabel::ProceedToDishes => "🍽️ Создать Блюда Меню",
        AssistantActionLabel::AddDish => "✨ Добавить Блюдо",
        AssistantActionLabel::GenerateReport => "📊 Сгенерировать Отчёт",
        AssistantActionLabel::ViewReport => "👁️ Посмотреть Отчёт",
        AssistantActionLabel::Finish => "✅ Завершить Настройку",
        AssistantActionLabel::Restart => "🔄 Начать Сначала",
    }
}

fn translate_hint_ru(key: AssistantHint) -> &'static str {
    match key {
        AssistantHint::InventoryWhy => {
            "Полный склад помогает отслеживать затраты и предотвращать дефицит."
        }
        AssistantHint::RecipeWhy => {
            "Рецепты стандартизируют приготовление и рассчитывают точную стоимость ингредиентов."
        }
        AssistantHint::DishWhy => {
            "Блюда меню объединяют рецепты с ценами для расчёта маржи прибыли."
        }
        AssistantHint::ReportWhy => "Отчёт показывает бизнес-метрики и выявляет возможности.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_messages_all_languages() {
        let keys = [
            AssistantMessage::Welcome,
            AssistantMessage::InventorySetup,
            AssistantMessage::RecipeSetup,
        ];

        for key in keys {
            assert!(!translate_message(key, Language::En).is_empty());
            assert!(!translate_message(key, Language::Pl).is_empty());
            assert!(!translate_message(key, Language::Uk).is_empty());
            assert!(!translate_message(key, Language::Ru).is_empty());
        }
    }

    #[test]
    fn test_translate_actions_all_languages() {
        let keys = [
            AssistantActionLabel::AddProducts,
            AssistantActionLabel::ProceedToRecipes,
        ];

        for key in keys {
            assert!(!translate_action(key, Language::En).is_empty());
            assert!(!translate_action(key, Language::Pl).is_empty());
            assert!(!translate_action(key, Language::Uk).is_empty());
            assert!(!translate_action(key, Language::Ru).is_empty());
        }
    }

    #[test]
    fn test_translate_hints_all_languages() {
        let keys = [AssistantHint::InventoryWhy, AssistantHint::RecipeWhy];

        for key in keys {
            assert!(!translate_hint(key, Language::En).is_empty());
            assert!(!translate_hint(key, Language::Pl).is_empty());
            assert!(!translate_hint(key, Language::Uk).is_empty());
            assert!(!translate_hint(key, Language::Ru).is_empty());
        }
    }
}
