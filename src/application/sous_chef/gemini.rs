//! Gemini / LLM prompt builder & response parser for Sous-Chef Planner.
//!
//! Only TEXT goes to LLM — no nutrition data, no recipes.
//! LLM adds personality: chef_intro, explanation, motivation.

use serde::Deserialize;
use super::goal::Goal;
use super::types::MealVariant;

/// LLM response shape.
#[derive(Debug, Deserialize)]
pub struct GeminiChefResponse {
    pub chef_intro: String,
    pub explanation: String,
    pub motivation: String,
}

/// Build a prompt that asks LLM to generate personality text only.
pub fn build_gemini_prompt(goal: Goal, lang: &str, variants: &[MealVariant]) -> String {
    let lang_name = match lang {
        "ru" => "Russian",
        "pl" => "Polish",
        "uk" => "Ukrainian",
        _ => "English",
    };

    let variant_titles: Vec<String> = variants
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{}. {} {} — {} kcal", i + 1, v.emoji, v.title, v.calories))
        .collect();

    format!(
        r#"You are a friendly professional chef assistant. Respond ONLY in {lang_name}.

User goal: {goal}

I already have {n} meal variants prepared:
{variants}

Generate a JSON response with EXACTLY these 3 fields:
{{
  "chef_intro": "A warm 1-2 sentence greeting acknowledging the user's goal. Be encouraging and professional.",
  "explanation": "A brief 2-3 sentence explanation of the nutritional strategy behind these meals. Why these ingredients work for this goal.",
  "motivation": "An encouraging 1-2 sentence motivation to try these meals."
}}

Rules:
- Respond ONLY with valid JSON, no markdown, no extra text
- All text must be in {lang_name}
- Keep it concise and practical
- Be warm but professional"#,
        lang_name = lang_name,
        goal = goal.slug(),
        n = variants.len(),
        variants = variant_titles.join("\n"),
    )
}

/// Parse raw LLM response into structured GeminiChefResponse.
pub fn parse_gemini_response(raw: &str) -> Option<GeminiChefResponse> {
    // Try direct parse
    if let Ok(r) = serde_json::from_str::<GeminiChefResponse>(raw) {
        return Some(r);
    }
    // Try extracting JSON from markdown fences
    let trimmed = raw.trim();
    let json_str = if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            &trimmed[start..=end]
        } else {
            return None;
        }
    } else {
        return None;
    };
    serde_json::from_str::<GeminiChefResponse>(json_str).ok()
}

// ── Fallbacks (when LLM is unavailable) ─────────────────────────────────────

pub fn fallback_intro(goal: Goal, lang: &str) -> String {
    match (goal, lang) {
        (Goal::WeightLoss, "ru") => "🧑‍🍳 Я подобрал для вас лёгкие, но сытные блюда с дефицитом калорий.".into(),
        (Goal::WeightLoss, "pl") => "🧑‍🍳 Przygotowałem dla Ciebie lekkie, ale sycące dania z deficytem kalorycznym.".into(),
        (Goal::WeightLoss, "uk") => "🧑‍🍳 Я підібрав для вас легкі, але ситні страви з дефіцитом калорій.".into(),
        (Goal::WeightLoss, _)   => "🧑‍🍳 I've selected light but satisfying meals with a calorie deficit for you.".into(),

        (Goal::HighProtein, "ru") => "🧑‍🍳 Вот план с высоким содержанием белка для ваших целей.".into(),
        (Goal::HighProtein, "pl") => "🧑‍🍳 Oto plan z dużą ilością białka dla Twoich celów.".into(),
        (Goal::HighProtein, "uk") => "🧑‍🍳 Ось план з високим вмістом білка для ваших цілей.".into(),
        (Goal::HighProtein, _)   => "🧑‍🍳 Here's a high-protein plan tailored to your goals.".into(),

        (Goal::QuickBreakfast, "ru") => "🧑‍🍳 Быстрый и питательный завтрак — лучшее начало дня!".into(),
        (Goal::QuickBreakfast, "pl") => "🧑‍🍳 Szybkie i pożywne śniadanie — najlepszy początek dnia!".into(),
        (Goal::QuickBreakfast, "uk") => "🧑‍🍳 Швидкий та поживний сніданок — найкращий початок дня!".into(),
        (Goal::QuickBreakfast, _)   => "🧑‍🍳 A quick and nutritious breakfast — the best way to start your day!".into(),

        (_, "ru") => "🧑‍🍳 Я подготовил для вас несколько вариантов на выбор.".into(),
        (_, "pl") => "🧑‍🍳 Przygotowałem dla Ciebie kilka opcji do wyboru.".into(),
        (_, "uk") => "🧑‍🍳 Я підготував для вас кілька варіантів на вибір.".into(),
        (_, _)   => "🧑‍🍳 I've prepared several options for you to choose from.".into(),
    }
}

pub fn fallback_explanation(goal: Goal, lang: &str) -> String {
    match (goal, lang) {
        (Goal::WeightLoss, "ru") => "Эти блюда содержат много клетчатки и белка при умеренной калорийности. Это поможет чувствовать сытость без переедания.".into(),
        (Goal::WeightLoss, "pl") => "Te dania zawierają dużo błonnika i białka przy umiarkowanej kaloryczności. To pomoże czuć sytość bez przejadania.".into(),
        (Goal::WeightLoss, "uk") => "Ці страви містять багато клітковини та білка при помірній калорійності. Це допоможе відчувати ситість без переїдання.".into(),
        (Goal::WeightLoss, _)   => "These meals are rich in fiber and protein with moderate calories. This helps you feel full without overeating.".into(),

        (Goal::HighProtein, "ru") => "Каждый вариант содержит от 30г белка — идеально для восстановления и роста мышц.".into(),
        (Goal::HighProtein, "pl") => "Każda opcja zawiera od 30g białka — idealna do regeneracji i budowy mięśni.".into(),
        (Goal::HighProtein, "uk") => "Кожен варіант містить від 30г білка — ідеально для відновлення та росту м'язів.".into(),
        (Goal::HighProtein, _)   => "Each option contains 30g+ protein — ideal for recovery and muscle growth.".into(),

        (_, "ru") => "Сбалансированное сочетание белков, жиров и углеводов для энергии и здоровья.".into(),
        (_, "pl") => "Zbilansowane połączenie białek, tłuszczów i węglowodanów dla energii i zdrowia.".into(),
        (_, "uk") => "Збалансоване поєднання білків, жирів та вуглеводів для енергії та здоров'я.".into(),
        (_, _)   => "A balanced mix of protein, fats, and carbs for energy and health.".into(),
    }
}

pub fn fallback_motivation(lang: &str) -> String {
    match lang {
        "ru" => "💪 Начните с любого варианта — каждый из них принесёт пользу вашему организму!".into(),
        "pl" => "💪 Zacznij od dowolnej opcji — każda z nich przyniesie korzyści Twojemu organizmowi!".into(),
        "uk" => "💪 Почніть з будь-якого варіанту — кожен з них принесе користь вашому організму!".into(),
        _ =>    "💪 Start with any option — each one will benefit your body!".into(),
    }
}
