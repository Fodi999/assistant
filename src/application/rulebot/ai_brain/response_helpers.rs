//! Response helpers — text formatting, suggestions, fallback messages.
//!
//! Stateless utility functions shared across AI Brain modules.
//! No LLM calls, no async — pure formatting.

use crate::application::rulebot::intent_router::ChatLang;
use crate::application::rulebot::chat_response::Suggestion;
use crate::application::rulebot::session_context::SessionContext;
use crate::infrastructure::ingredient_cache::IngredientData;

// ── Nutrition formatting ─────────────────────────────────────────────────────

pub(crate) fn format_nutrition_response(
    name: &str,
    p: &IngredientData,
    lang: ChatLang,
) -> String {
    match lang {
        ChatLang::Ru => format!(
            "**{}** — на 100г: **{} ккал**, белок **{}г**, жиры **{}г**, углеводы **{}г**.",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::En => format!(
            "**{}** — per 100g: **{} kcal**, protein **{}g**, fat **{}g**, carbs **{}g**.",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Pl => format!(
            "**{}** — na 100g: **{} kcal**, białko **{}g**, tłuszcz **{}g**, węglowodany **{}g**.",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
        ChatLang::Uk => format!(
            "**{}** — на 100г: **{} ккал**, білок **{}г**, жири **{}г**, вуглеводи **{}г**.",
            name, p.calories_per_100g as i32, p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g
        ),
    }
}

// ── Fallback / empty texts ───────────────────────────────────────────────────

pub(crate) fn no_products_text(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "К сожалению, база продуктов пуста. Попробуй позже.",
        ChatLang::En => "Sorry, the product database is empty. Please try later.",
        ChatLang::Pl => "Niestety baza produktów jest pusta. Spróbuj później.",
        ChatLang::Uk => "На жаль, база продуктів порожня. Спробуй пізніше.",
    }
}

pub(crate) fn fallback_text(lang: ChatLang) -> &'static str {
    match lang {
        ChatLang::Ru => "Произошла ошибка. Попробуй переформулировать вопрос.",
        ChatLang::En => "An error occurred. Try rephrasing your question.",
        ChatLang::Pl => "Wystąpił błąd. Spróbuj inaczej sformułować pytanie.",
        ChatLang::Uk => "Сталася помилка. Спробуй переформулювати питання.",
    }
}

// ── Suggestions ──────────────────────────────────────────────────────────────

/// Suggestions that redirect user to core capabilities.
pub(crate) fn redirect_suggestions(lang: ChatLang) -> Vec<Suggestion> {
    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "🥗 Полезные продукты".to_string(), query: "что полезного поесть".to_string(), emoji: Some("🥗") },
            Suggestion { label: "🍳 Идея на ужин".to_string(), query: "что приготовить на ужин".to_string(), emoji: Some("🍳") },
            Suggestion { label: "📊 Калории продукта".to_string(), query: "сколько калорий в курице".to_string(), emoji: Some("📊") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "🥗 Healthy foods".to_string(), query: "healthy food ideas".to_string(), emoji: Some("🥗") },
            Suggestion { label: "🍳 Dinner idea".to_string(), query: "what to cook for dinner".to_string(), emoji: Some("🍳") },
            Suggestion { label: "📊 Calories".to_string(), query: "calories in chicken".to_string(), emoji: Some("📊") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "🥗 Zdrowe produkty".to_string(), query: "co zdrowego zjeść".to_string(), emoji: Some("🥗") },
            Suggestion { label: "🍳 Pomysł na obiad".to_string(), query: "co ugotować na obiad".to_string(), emoji: Some("🍳") },
            Suggestion { label: "📊 Kalorie".to_string(), query: "kalorie w kurczaku".to_string(), emoji: Some("📊") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "🥗 Корисні продукти".to_string(), query: "що корисного з'їсти".to_string(), emoji: Some("🥗") },
            Suggestion { label: "🍳 Ідея на вечерю".to_string(), query: "що приготувати на вечерю".to_string(), emoji: Some("🍳") },
            Suggestion { label: "📊 Калорії".to_string(), query: "скільки калорій в курці".to_string(), emoji: Some("📊") },
        ],
    }
}

/// Suggestions for borderline price questions — redirect to cost calculation.
pub(crate) fn borderline_price_suggestions(lang: ChatLang) -> Vec<Suggestion> {
    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "💰 Себестоимость блюда".to_string(), query: "рассчитай себестоимость бургера".to_string(), emoji: Some("💰") },
            Suggestion { label: "📊 Калорийность".to_string(), query: "калорийность бургера".to_string(), emoji: Some("📊") },
            Suggestion { label: "🍳 Рецепт".to_string(), query: "рецепт домашнего бургера".to_string(), emoji: Some("🍳") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "💰 Dish cost".to_string(), query: "calculate burger cost".to_string(), emoji: Some("💰") },
            Suggestion { label: "📊 Calories".to_string(), query: "burger calories".to_string(), emoji: Some("📊") },
            Suggestion { label: "🍳 Recipe".to_string(), query: "homemade burger recipe".to_string(), emoji: Some("🍳") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "💰 Koszt potrawy".to_string(), query: "oblicz koszt burgera".to_string(), emoji: Some("💰") },
            Suggestion { label: "📊 Kalorie".to_string(), query: "kalorie burgera".to_string(), emoji: Some("📊") },
            Suggestion { label: "🍳 Przepis".to_string(), query: "przepis na domowego burgera".to_string(), emoji: Some("🍳") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "💰 Собівартість страви".to_string(), query: "розрахуй собівартість бургера".to_string(), emoji: Some("💰") },
            Suggestion { label: "📊 Калорійність".to_string(), query: "калорійність бургера".to_string(), emoji: Some("📊") },
            Suggestion { label: "🍳 Рецепт".to_string(), query: "рецепт домашнього бургера".to_string(), emoji: Some("🍳") },
        ],
    }
}

/// Follow-up suggestions after product search.
pub(crate) fn build_followup_suggestions(lang: ChatLang, _goal: &str) -> Vec<Suggestion> {
    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "🔄 Ещё варианты".to_string(), query: "покажи другие продукты".to_string(), emoji: Some("🔄") },
            Suggestion { label: "📊 Сравнить".to_string(), query: "сравни их по калориям".to_string(), emoji: Some("📊") },
            Suggestion { label: "🍽️ Рецепт".to_string(), query: "что из них приготовить".to_string(), emoji: Some("🍽️") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "🔄 More options".to_string(), query: "show me more products".to_string(), emoji: Some("🔄") },
            Suggestion { label: "📊 Compare".to_string(), query: "compare them by calories".to_string(), emoji: Some("📊") },
            Suggestion { label: "🍽️ Recipe".to_string(), query: "what can I cook with these".to_string(), emoji: Some("🍽️") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "🔄 Więcej".to_string(), query: "pokaż więcej produktów".to_string(), emoji: Some("🔄") },
            Suggestion { label: "📊 Porównaj".to_string(), query: "porównaj je po kaloriach".to_string(), emoji: Some("📊") },
            Suggestion { label: "🍽️ Przepis".to_string(), query: "co z nich ugotować".to_string(), emoji: Some("🍽️") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "🔄 Ще варіанти".to_string(), query: "покажи інші продукти".to_string(), emoji: Some("🔄") },
            Suggestion { label: "📊 Порівняти".to_string(), query: "порівняй їх за калоріями".to_string(), emoji: Some("📊") },
            Suggestion { label: "🍽️ Рецепт".to_string(), query: "що з них приготувати".to_string(), emoji: Some("🍽️") },
        ],
    }
}

/// Follow-up suggestions after meal plan.
pub(crate) fn build_meal_plan_suggestions(lang: ChatLang) -> Vec<Suggestion> {
    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "🔄 Другой план".to_string(), query: "предложи другой план".to_string(), emoji: Some("🔄") },
            Suggestion { label: "💪 Больше белка".to_string(), query: "план с большим количеством белка".to_string(), emoji: Some("💪") },
            Suggestion { label: "🥗 Легче".to_string(), query: "план полегче, меньше калорий".to_string(), emoji: Some("🥗") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "🔄 Different plan".to_string(), query: "suggest a different plan".to_string(), emoji: Some("🔄") },
            Suggestion { label: "💪 More protein".to_string(), query: "plan with more protein".to_string(), emoji: Some("💪") },
            Suggestion { label: "🥗 Lighter".to_string(), query: "lighter plan, fewer calories".to_string(), emoji: Some("🥗") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "🔄 Inny plan".to_string(), query: "zaproponuj inny plan".to_string(), emoji: Some("🔄") },
            Suggestion { label: "💪 Więcej białka".to_string(), query: "plan z większą ilością białka".to_string(), emoji: Some("💪") },
            Suggestion { label: "🥗 Lżejszy".to_string(), query: "lżejszy plan, mniej kalorii".to_string(), emoji: Some("🥗") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "🔄 Інший план".to_string(), query: "запропонуй інший план".to_string(), emoji: Some("🔄") },
            Suggestion { label: "💪 Більше білка".to_string(), query: "план з більшою кількістю білка".to_string(), emoji: Some("💪") },
            Suggestion { label: "🥗 Легше".to_string(), query: "легший план, менше калорій".to_string(), emoji: Some("🥗") },
        ],
    }
}

// ── Utility ──────────────────────────────────────────────────────────────────

/// Truncate string for logging.
pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}

/// Build session context hint for LLM prompt.
pub(crate) fn build_context_hint(ctx: &SessionContext) -> String {
    let mut parts = Vec::new();
    if let Some(ref slug) = ctx.last_product_slug {
        parts.push(format!("Last product discussed: {}", slug));
    }
    if let Some(ref name) = ctx.last_product_name {
        parts.push(format!("Last product name: {}", name));
    }
    if let Some(modifier) = ctx.last_modifier {
        parts.push(format!("Active goal: {}", modifier.label()));
    }
    if ctx.turn_count > 0 {
        parts.push(format!("Turn: {}", ctx.turn_count));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("SESSION CONTEXT:\n{}", parts.join("\n"))
    }
}

/// Check if a product name matches any word in the user query.
pub(crate) fn product_matches_query(p: &IngredientData, query: &str) -> bool {
    let words: Vec<&str> = query.split_whitespace().collect();
    words.iter().any(|w| {
        p.name_en.to_lowercase().contains(w)
            || p.name_ru.to_lowercase().contains(w)
            || p.name_pl.to_lowercase().contains(w)
            || p.name_uk.to_lowercase().contains(w)
            || p.slug.contains(w)
    })
}
