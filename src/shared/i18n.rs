use crate::shared::Language;

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
        AssistantHint::InventoryWhy => "Having a complete inventory helps track costs and prevent shortages.",
        AssistantHint::RecipeWhy => "Recipes standardize preparation and calculate exact ingredient costs.",
        AssistantHint::DishWhy => "Menu dishes combine recipes with pricing to calculate profit margins.",
        AssistantHint::ReportWhy => "The report shows your business metrics and identifies opportunities.",
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
        AssistantHint::InventoryWhy => "Kompletny magazyn pomaga śledzić koszty i zapobiegać brakom.",
        AssistantHint::RecipeWhy => "Przepisy standaryzują przygotowanie i obliczają dokładne koszty składników.",
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
        AssistantHint::InventoryWhy => "Повний склад допомагає відстежувати витрати та запобігати дефіциту.",
        AssistantHint::RecipeWhy => "Рецепти стандартизують приготування та розраховують точну вартість інгредієнтів.",
        AssistantHint::DishWhy => "Страви меню поєднують рецепти з цінами для розрахунку маржі прибутку.",
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
        AssistantHint::InventoryWhy => "Полный склад помогает отслеживать затраты и предотвращать дефицит.",
        AssistantHint::RecipeWhy => "Рецепты стандартизируют приготовление и рассчитывают точную стоимость ингредиентов.",
        AssistantHint::DishWhy => "Блюда меню объединяют рецепты с ценами для расчёта маржи прибыли.",
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
        let keys = [
            AssistantHint::InventoryWhy,
            AssistantHint::RecipeWhy,
        ];

        for key in keys {
            assert!(!translate_hint(key, Language::En).is_empty());
            assert!(!translate_hint(key, Language::Pl).is_empty());
            assert!(!translate_hint(key, Language::Uk).is_empty());
            assert!(!translate_hint(key, Language::Ru).is_empty());
        }
    }
}
