//! Intent Router — score-based intent detection from natural language input.
//!
//! NOT simple `text.contains()` — uses weighted keyword scoring per intent.
//! Supports:
//!   - Multi-intent: "что-то полезное и быстрое" → [HealthyProduct, MealIdea]
//!   - Context modifiers: "на массу" → HighProtein, "хочу сушиться" → LowCalorie
//!   - Score threshold: min 2 pts to activate an intent
//!
//! ```text
//! "что полезного поесть"        → [HealthyProduct]  score=5
//! "200 грамм в ложках"          → [Conversion]      score=6
//! "что поесть полезное быстрое" → [HealthyProduct, MealIdea]
//! "хочу на массу"               → modifier=HighProtein
//! "хочу сушиться"               → modifier=LowCalorie
//! ```

use serde::{Deserialize, Serialize};

// ── Intent Enum ──────────────────────────────────────────────────────────────

/// Detected user intent from free-text input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// "привет", "hello", "cześć"
    Greeting,
    /// "полезный продукт", "healthy food", "zdrowy produkt"
    HealthyProduct,
    /// "грамм в ложках", "convert 200g", "ile to łyżek"
    Conversion,
    /// "рецепт борща", "recipe for soup", "przepis na zupę"
    RecipeHelp,
    /// "сезон лосося", "when is salmon in season"
    Seasonality,
    /// "что приготовить", "meal idea", "co ugotować"
    MealIdea,
    /// "калории шпината", "nutrition info", "wartości odżywcze"
    NutritionInfo,
    /// "что такое лосось", "tell me about chicken", "co to jest szpinak"
    ProductInfo,
    /// Could not determine intent — triggers controlled LLM fallback
    Unknown,
}

impl Intent {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Greeting       => "greeting",
            Self::HealthyProduct => "healthy_product",
            Self::Conversion     => "conversion",
            Self::RecipeHelp     => "recipe_help",
            Self::Seasonality    => "seasonality",
            Self::MealIdea       => "meal_idea",
            Self::NutritionInfo  => "nutrition_info",
            Self::ProductInfo    => "product_info",
            Self::Unknown        => "unknown",
        }
    }
}

// ── Health/goal Modifier ─────────────────────────────────────────────────────

/// Additional context modifier extracted alongside the primary intent.
/// Influences product selection and response text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthModifier {
    /// "на массу", "набрать мышцы", "protein", "muscle"
    HighProtein,
    /// "похудеть", "сушиться", "диета", "lose weight"
    LowCalorie,
    /// "быстрое", "quick", "fast", "за 15 минут"
    Quick,
    /// "бюджетное", "дешевле", "cheap", "budget"
    Budget,
    /// No specific modifier detected
    None,
}

impl HealthModifier {
    pub fn label(&self) -> &'static str {
        match self {
            Self::HighProtein => "high_protein",
            Self::LowCalorie  => "low_calorie",
            Self::Quick       => "quick",
            Self::Budget      => "budget",
            Self::None        => "none",
        }
    }
}

// ── Language Detection ───────────────────────────────────────────────────────

/// Detected language of the user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatLang {
    Ru,
    En,
    Pl,
    Uk,
}

impl ChatLang {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
            Self::Pl => "pl",
            Self::Uk => "uk",
        }
    }
}

// ── Score-based Intent Detection ─────────────────────────────────────────────

/// A scored keyword: (keyword, weight).
type ScoredKeyword = (&'static str, i32);

/// Single intent detection result with score.
#[derive(Debug, Clone)]
pub struct IntentScore {
    pub intent: Intent,
    pub score: i32,
}

/// Full parsed result: primary intent + all active intents + modifier.
#[derive(Debug, Clone)]
pub struct ParsedInput {
    /// Highest-scoring intent (primary).
    pub intent: Intent,
    /// All intents above threshold — enables multi-intent dispatch.
    pub intents: Vec<Intent>,
    /// Goal/context modifier detected from input.
    pub modifier: HealthModifier,
}

/// Detect primary intent (backward compatible).
pub fn detect_intent(input: &str) -> Intent {
    parse_input(input).intent
}

/// Full scored detection — returns intent + its score (for debug/logging).
pub fn detect_intent_scored(input: &str) -> IntentScore {
    const MIN_THRESHOLD: i32 = 2;
    let text = input.to_lowercase();
    let mut scores: [(Intent, i32); 8] = [
        (Intent::Greeting,       score_greeting(&text)),
        (Intent::Conversion,     score_conversion(&text)),
        (Intent::NutritionInfo,  score_nutrition(&text)),
        (Intent::ProductInfo,    score_product_info(&text)),
        (Intent::HealthyProduct, score_healthy(&text)),
        (Intent::Seasonality,    score_seasonality(&text)),
        (Intent::RecipeHelp,     score_recipe(&text)),
        (Intent::MealIdea,       score_meal_idea(&text)),
    ];
    apply_context_boosts(&text, &mut scores);
    let best = scores.iter().max_by_key(|(_, s)| *s).unwrap();
    if best.1 >= MIN_THRESHOLD {
        IntentScore { intent: best.0, score: best.1 }
    } else {
        IntentScore { intent: Intent::Unknown, score: 0 }
    }
}

/// Full parse: primary intent + multi-intents + goal modifier.
/// Use this in ChatEngine for full context.
pub fn parse_input(input: &str) -> ParsedInput {
    const MIN_THRESHOLD: i32 = 2;
    // Multi-intent secondary threshold — slightly lower to catch co-signals
    const SECONDARY_THRESHOLD: i32 = 2;

    let text = input.to_lowercase();

    let mut all_scores: [(Intent, i32); 8] = [
        (Intent::Greeting,       score_greeting(&text)),
        (Intent::Conversion,     score_conversion(&text)),
        (Intent::NutritionInfo,  score_nutrition(&text)),
        (Intent::ProductInfo,    score_product_info(&text)),
        (Intent::HealthyProduct, score_healthy(&text)),
        (Intent::Seasonality,    score_seasonality(&text)),
        (Intent::RecipeHelp,     score_recipe(&text)),
        (Intent::MealIdea,       score_meal_idea(&text)),
    ];

    apply_context_boosts(&text, &mut all_scores);

    // Best (primary)
    let best = all_scores.iter().max_by_key(|(_, s)| *s).unwrap();
    let primary = if best.1 >= MIN_THRESHOLD { best.0 } else { Intent::Unknown };

    // All intents above secondary threshold (multi-intent)
    let mut intents: Vec<Intent> = all_scores
        .iter()
        .filter(|(i, s)| *s >= SECONDARY_THRESHOLD && *i != primary)
        .map(|(i, _)| *i)
        .collect();

    // Don't include Unknown in multi-intent list
    if primary != Intent::Unknown {
        intents.insert(0, primary);
    }

    let modifier = detect_modifier(&text);

    ParsedInput {
        intent: primary,
        intents,
        modifier,
    }
}

/// Detect goal/context modifier — runs independently of intent scoring.
pub fn detect_modifier(text: &str) -> HealthModifier {
    // HighProtein patterns
    let high_protein: &[&str] = &[
        "на массу", "набрать мышц", "набрать мус", "для мышц", "мышечн",
        "on bulk", "bulk", "muscle", "high protein", "для белка", "протеин",
        "na masę", "na mase", "budowanie mięśni",
    ];
    if high_protein.iter().any(|kw| text.contains(kw)) {
        return HealthModifier::HighProtein;
    }

    // LowCalorie / cutting patterns
    let low_calorie: &[&str] = &[
        "сушиться", "сушка", "похудеть", "похудения", "для похудения",
        "диет", "сбросить вес", "lose weight", "weight loss", "cutting",
        "low calorie", "low cal", "mało kalorii", "odchudzanie", "schudnąć",
        // Ukrainian
        "схуднути", "схуднення", "для схуднення", "скинути вагу",
        "худнути", "худіти", "на дієті",
    ];
    if low_calorie.iter().any(|kw| text.contains(kw)) {
        return HealthModifier::LowCalorie;
    }

    // Quick patterns
    let quick: &[&str] = &[
        "быстр", "за 15 минут", "за 20 минут", "за 10 минут",
        "quick", "fast", "szybk", "szybkie", "на скорую", "скорый",
    ];
    if quick.iter().any(|kw| text.contains(kw)) {
        return HealthModifier::Quick;
    }

    // Budget patterns
    let budget: &[&str] = &[
        "бюджетн", "дешевле", "дешево", "дёшево", "cheap", "budget",
        "tanie", "tani", "недорого", "економ",
    ];
    if budget.iter().any(|kw| text.contains(kw)) {
        return HealthModifier::Budget;
    }

    HealthModifier::None
}

/// Detect language from input text.
pub fn detect_language(input: &str) -> ChatLang {
    let text = input.to_lowercase();

    // Polish-specific characters
    if text.contains('ą') || text.contains('ę') || text.contains('ś')
        || text.contains('ć') || text.contains('ź') || text.contains('ż')
        || text.contains('ł') || text.contains('ń') || text.contains('ó')
    {
        return ChatLang::Pl;
    }

    // Ukrainian-specific characters (ї, і, є, ґ)
    if text.contains('ї') || text.contains('ґ') || text.contains('є') {
        return ChatLang::Uk;
    }

    // Cyrillic = Russian (default cyrillic)
    if text.chars().any(|c| matches!(c, 'а'..='я' | 'А'..='Я')) {
        return ChatLang::Ru;
    }

    // Default: English
    ChatLang::En
}

// ── Per-intent scorers ────────────────────────────────────────────────────────

fn score_greeting(text: &str) -> i32 {
    let keywords: &[ScoredKeyword] = &[
        ("привет",        3), ("здравствуй",    3), ("hello",          3),
        ("cześć",         3), ("вітаю",          3), ("привіт",         3),
        ("hej",           3), ("witam",           3),
        ("добрый день",   2), ("добрый вечер",   2), ("доброе утро",    2),
        ("good morning",  2), ("good evening",   2), ("good afternoon", 2),
        ("dzień dobry",   2), ("добрий день",    2),
        ("хай",           1), ("здарова",        1), ("hey",            1),
        ("hi ",           1), ("yo ",            1),
    ];
    let base = if text.trim() == "hi" { 3 } else { 0 };
    base + sum_scores(text, keywords)
}

fn score_conversion(text: &str) -> i32 {
    let keywords: &[ScoredKeyword] = &[
        ("грамм",         3), ("граммов",        3), ("gram",           3),
        ("kilogram",      3), ("килограмм",      3), ("кг",             3),
        ("миллилитр",     3), ("milliliter",     3), ("мл ",            3),
        ("литр",          3), ("liter",          3),
        ("ложк",          2), ("tbsp",           2), ("tablespoon",     2),
        ("teaspoon",      2), ("стакан",          2), ("cup",            2),
        ("унц",           2), ("ounce",          2), ("oz",             2),
        ("łyżk",          2), ("szklank",        2),
        ("перевести",     2), ("convert",        2), ("przelicz",       2),
        ("перевод",       2), ("конвертировать", 2),
        ("в граммах",     1), ("в ложках",       1), ("в стаканах",     1),
        ("in grams",      1), ("in ounces",      1),
        ("сколько",       1), ("how many",       1), ("how much",       1),
        ("ile to",        1), ("ile ",           1),
    ];
    sum_scores(text, keywords)
}

fn score_nutrition(text: &str) -> i32 {
    let keywords: &[ScoredKeyword] = &[
        ("калори",        3), ("calori",         3), ("ккал",           3),
        ("kcal",          3), ("калорій",        3),
        ("пищевая ценность", 3),
        ("белок",         2), ("белки",          2), ("protein",        2),
        ("białk",         2), ("білок",          2), ("білки",          2),
        ("жиры",          2), ("жир",            2), ("fat",            2),
        ("tłuszcz",       2),
        ("углевод",       2), ("carb",           2), ("węglowodan",     2),
        ("нутри",         2), ("nutrition",      2), ("macro",          2),
        ("бжу",           2),
        ("витамин",       1), ("vitamin",        1), ("mineral",        1),
        ("микроэлемент",  1),
    ];
    sum_scores(text, keywords)
}

fn score_healthy(text: &str) -> i32 {
    let keywords: &[ScoredKeyword] = &[
        // Direct healthy-food requests
        ("полезн",        3), ("healthy",        3), ("zdrow",          3),
        ("корисн",        3), ("superfood",      3),
        ("самый полезный",3), ("most nutritious",3),
        ("здоров",        2), ("питательн",      2), ("nutritious",     2),
        ("beneficial",    2), ("pożyteczn",      2), ("witamin",        2),
        ("вітамін",       2), ("good for health",2), ("полезного",      2),
        ("антиоксидант",  1), ("antioxidant",    1), ("иммунитет",      1),
        ("immunity",      1), ("диет",           1), ("diet",           1),
        // Goal phrases → HealthyProduct (these MUST produce cards, not LLM fallback)
        ("похуд",         5), ("schudnąć",       5), ("lose weight",    5),
        ("хочу похудеть", 5), ("chcę schudnąć",  5), ("want to lose",   5),
        ("хочу схуднути", 5), ("сбросить вес",   5), ("скинуть вес",    5),
        ("схуднути",      5), ("схуднення",      5), ("скинути вагу",   5),
        ("худнути",       4), ("худіти",         4),
        ("сушк",          4), ("сушит",          4), ("жиросжиган",     4),
        ("для похудения", 5), ("на похудение",   5), ("на сушку",       4),
        ("na odchudzanie",5), ("для зниження",   4), ("znizit ves",     4),
        // Muscle / protein goals → also HealthyProduct
        ("набрать мышц",  5), ("набор массы",    5), ("на массу",       5),
        ("мышечн",        4), ("muscle",         4), ("на белок",       4),
        ("nabrać masy",   5), ("на протеин",     4), ("protein goal",   4),
        ("набрати масу",  5),
        // Generic "что есть" / "что поесть" → HealthyProduct
        ("что есть",      3), ("что поесть",     3), ("что съесть",     3),
        ("co jeść",       3), ("co zjeść",       3), ("що їсти",        3),
        ("що поїсти",     3), ("что кушать",     3),
        // "ещё" follow-up patterns → HealthyProduct
        ("ещё полезн",    4), ("ещё низкокалор", 4), ("ещё высокобелк", 4),
        ("ещё вариант",   3), ("more healthy",   3), ("więcej zdrow",   3),
        ("ще корисн",     4), ("ще низькокалор", 4), ("ще високобілк",  4),
    ];
    sum_scores(text, keywords)
}

fn score_seasonality(text: &str) -> i32 {
    let keywords: &[ScoredKeyword] = &[
        ("сезон",         3), ("season",         3), ("sezon",          3),
        ("в сезоне",      3), ("in season",      3),
        ("сезонный",      2), ("seasonal",       2), ("sezonow",        2),
        ("когда сезон",   2), ("when is season", 2), ("kiedy sezon",    2),
        ("коли сезон",    2),
        ("свежий",        1), ("fresh",          1), ("świeży",         1),
        ("урожай",        1), ("harvest",        1),
    ];
    sum_scores(text, keywords)
}

fn score_recipe(text: &str) -> i32 {
    let keywords: &[ScoredKeyword] = &[
        ("рецепт",        3), ("recipe",         3), ("przepis",        3),
        ("рецепти",       3),
        ("как приготовить",2), ("how to cook",   2), ("jak ugotować",   2),
        ("jak przygotować",2), ("як приготувати",2),
        ("как сделать",   2), ("how to make",    2),
        ("приготовление", 2), ("cooking",        2), ("gotowanie",      2),
        ("ингредиент",    1), ("ingredient",     1), ("składnik",       1),
        ("шаги",          1), ("steps",          1),
    ];
    sum_scores(text, keywords)
}

fn score_meal_idea(text: &str) -> i32 {
    let keywords: &[ScoredKeyword] = &[
        ("что приготовить",3), ("what to cook",  3), ("co ugotować",    3),
        ("що приготувати", 3), ("meal idea",     3), ("pomysł na",      3),
        ("dinner idea",   3), ("lunch idea",    3), ("breakfast idea", 3),
        ("план питания",  4), ("meal plan",     4), ("план харчування",4),
        ("plan posiłków", 4), ("рацион на день",4), ("plan dnia",      4),
        ("что поесть",    2), ("what to eat",   2), ("co zjeść",       2),
        ("що поїсти",     2), ("идея блюда",    2), ("ідея страви",    2),
        ("предложи блюдо",2), ("suggest a meal",2),
        ("приготовить",   2), ("cook",          1), ("ugotować",       2),
        ("приготувати",   2), ("готовить",       2),
        ("собрать день",  3), ("build my day",  3), ("ułóż dzień",     3),
        ("обед",          1), ("ужин",           1), ("завтрак",        1),
        ("lunch",         1), ("dinner",         1), ("breakfast",      1),
        ("obiad",         1), ("kolacja",        1), ("śniadanie",      1),
        ("на ужин",       2), ("на обед",        2), ("на завтрак",     2),
        ("for dinner",    2), ("for lunch",      2), ("for breakfast",  2),
        ("na obiad",      2), ("na kolację",     2), ("na śniadanie",   2),
        ("на вечерю",     2), ("на обід",        2), ("на сніданок",    2),
    ];
    sum_scores(text, keywords)
}

fn score_product_info(text: &str) -> i32 {
    let keywords: &[ScoredKeyword] = &[
        ("что такое",     3), ("what is",        3), ("co to jest",     3),
        ("що таке",       3), ("расскажи о",     3), ("tell me about",  3),
        ("расскажи про",  3), ("опиши",          3), ("describe",       3),
        ("что это",       2), ("what's",         2), ("czym jest",      2),
        ("о продукте",    2), ("about ",         2),
        ("подробнее о",   2), ("more about",     2),
        ("инфо",          1), ("info",           1), ("данные",         1),
        ("informacje",    1),
    ];
    sum_scores(text, keywords)
}

// ── Scoring helper ─────────────────────────────────────────────────────────────

fn sum_scores(text: &str, keywords: &[ScoredKeyword]) -> i32 {
    keywords.iter()
        .filter(|(kw, _)| text.contains(kw))
        .map(|(_, weight)| weight)
        .sum()
}

/// Context-aware score adjustments applied AFTER all individual scorers run.
///
/// Fixes:
/// - "хочу на массу, что поесть?" → MealIdea (goal + action verb = meal, not product list)
/// - "low calorie dinner for diet" → MealIdea (meal-time + diet = meal, not nutrition_info)
fn apply_context_boosts(text: &str, scores: &mut [(Intent, i32); 8]) {
    let has_goal = text.contains("на массу") || text.contains("похуд") || text.contains("сушк")
        || text.contains("диет") || text.contains("diet") || text.contains("muscle")
        || text.contains("lose weight") || text.contains("bulk") || text.contains("набрать")
        || text.contains("high protein") || text.contains("low calorie") || text.contains("low cal")
        || text.contains("schudnąć") || text.contains("na masę") || text.contains("схуднути")
        || text.contains("на білок") || text.contains("на белок");

    let has_action_verb = text.contains("поесть") || text.contains("приготовить") || text.contains("готовить")
        || text.contains("cook") || text.contains("eat") || text.contains("make")
        || text.contains("zjeść") || text.contains("ugotować") || text.contains("поїсти")
        || text.contains("приготувати") || text.contains("съесть") || text.contains("кушать");

    let has_meal_time = text.contains("ужин") || text.contains("обед") || text.contains("завтрак")
        || text.contains("dinner") || text.contains("lunch") || text.contains("breakfast")
        || text.contains("kolacja") || text.contains("obiad") || text.contains("śniadanie")
        || text.contains("вечеря") || text.contains("обід") || text.contains("сніданок");

    // Boost MealIdea when goal + action verb or goal + meal time
    if has_goal && (has_action_verb || has_meal_time) {
        for (intent, score) in scores.iter_mut() {
            if *intent == Intent::MealIdea {
                *score += 5;
            }
            // When the user clearly wants a meal (goal + action/time),
            // penalize HealthyProduct to avoid it stealing the intent
            if *intent == Intent::HealthyProduct && *score > 0 {
                *score = (*score - 3).max(0);
            }
        }
    }

    // Boost MealIdea when meal time + diet/goal words (e.g. "low calorie dinner for diet")
    if has_meal_time && has_goal {
        // Also penalize nutrition_info when user clearly wants a meal suggestion
        for (intent, score) in scores.iter_mut() {
            if *intent == Intent::NutritionInfo && *score > 0 {
                *score = (*score - 2).max(0);
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn greeting_ru()     { assert_eq!(detect_intent("Привет!"),                Intent::Greeting); }
    #[test] fn greeting_en()     { assert_eq!(detect_intent("Hello there"),            Intent::Greeting); }
    #[test] fn greeting_hi()     { assert_eq!(detect_intent("hi"),                     Intent::Greeting); }
    #[test] fn greeting_pl()     { assert_eq!(detect_intent("Cześć!"),                 Intent::Greeting); }

    #[test] fn healthy_ru()      { assert_eq!(detect_intent("какой продукт полезный"), Intent::HealthyProduct); }
    #[test] fn healthy_ru2()     { assert_eq!(detect_intent("что полезного поесть"),   Intent::HealthyProduct); }
    #[test] fn healthy_en()      { assert_eq!(detect_intent("healthy food please"),    Intent::HealthyProduct); }
    #[test] fn healthy_super()   { assert_eq!(detect_intent("superfood рекомендации"),Intent::HealthyProduct); }

    #[test] fn conv_ru()         { assert_eq!(detect_intent("200 грамм в ложках"),     Intent::Conversion); }
    #[test] fn conv_en()         { assert_eq!(detect_intent("convert 100g to ounces"),Intent::Conversion); }
    #[test] fn conv_how()        { assert_eq!(detect_intent("сколько ложек в стакане"),Intent::Conversion); }

    #[test] fn nutr_ru()         { assert_eq!(detect_intent("сколько калорий в яблоке"),Intent::NutritionInfo); }
    #[test] fn nutr_en()         { assert_eq!(detect_intent("protein in chicken"),     Intent::NutritionInfo); }
    #[test] fn nutr_bju()        { assert_eq!(detect_intent("бжу шпината"),            Intent::NutritionInfo); }

    #[test] fn prod_info_ru()    { assert_eq!(detect_intent("что такое лосось"),        Intent::ProductInfo); }
    #[test] fn prod_info_en()    { assert_eq!(detect_intent("what is spinach"),         Intent::ProductInfo); }
    #[test] fn prod_info_pl()    { assert_eq!(detect_intent("co to jest brokuł"),      Intent::ProductInfo); }
    #[test] fn prod_info_tell()  { assert_eq!(detect_intent("расскажи о шпинате"),     Intent::ProductInfo); }

    #[test] fn recipe_ru()       { assert_eq!(detect_intent("рецепт борща"),            Intent::RecipeHelp); }
    #[test] fn recipe_en()       { assert_eq!(detect_intent("how to cook pasta"),       Intent::RecipeHelp); }

    #[test] fn meal_ru()         { assert_eq!(detect_intent("что приготовить на ужин"), Intent::MealIdea); }
    #[test] fn meal_en()         { assert_eq!(detect_intent("dinner idea"),             Intent::MealIdea); }

    #[test] fn season_ru()       { assert_eq!(detect_intent("когда сезон лосося"),      Intent::Seasonality); }
    #[test] fn season_en()       { assert_eq!(detect_intent("is salmon in season"),     Intent::Seasonality); }

    // Goal phrases → must route to HealthyProduct (NOT Unknown/LLM)
    #[test] fn goal_lose_weight_ru()  { assert_eq!(detect_intent("хочу похудеть"),         Intent::HealthyProduct); }
    #[test] fn goal_lose_weight_pl()  { assert_eq!(detect_intent("chcę schudnąć"),         Intent::HealthyProduct); }
    #[test] fn goal_lose_weight_en()  { assert_eq!(detect_intent("I want to lose weight"), Intent::HealthyProduct); }
    #[test] fn goal_muscle_ru()       { assert_eq!(detect_intent("хочу набрать мышечную массу"), Intent::HealthyProduct); }
    #[test] fn goal_na_massu_ru()     { assert_eq!(detect_intent("что есть на массу"),     Intent::HealthyProduct); }
    #[test] fn goal_chto_est_ru()     { assert_eq!(detect_intent("что поесть"),             Intent::HealthyProduct); }
    #[test] fn goal_chto_est_pl()     { assert_eq!(detect_intent("co zjeść"),               Intent::HealthyProduct); }
    #[test] fn goal_sushka_ru()       { assert_eq!(detect_intent("питание на сушку"),       Intent::HealthyProduct); }

    #[test] fn unknown_garbage() { assert_eq!(detect_intent("asdfghjkl"),               Intent::Unknown); }

    // Score-based wins: "полезный белок" → HealthyProduct (+3) > NutritionInfo (+2)
    #[test] fn ambiguous_healthy_wins() {
        let r = detect_intent_scored("полезный белок в шпинате");
        assert_eq!(r.intent, Intent::HealthyProduct);
        assert!(r.score >= 3);
    }

    // Language detection
    #[test] fn lang_ru()  { assert_eq!(detect_language("привет мир"),       ChatLang::Ru); }
    #[test] fn lang_en()  { assert_eq!(detect_language("hello world"),      ChatLang::En); }
    #[test] fn lang_pl()  { assert_eq!(detect_language("cześć świat"),      ChatLang::Pl); }
    #[test] fn lang_uk()  { assert_eq!(detect_language("привіт світ їжа"), ChatLang::Uk); }

    // Single weak word below threshold → Unknown
    #[test] fn score_threshold() {
        assert_eq!(detect_intent("xyz abc def"), Intent::Unknown);
    }

    // ── Fix 2 & 3: Context boost tests ──────────────────────────────────
    // Goal + action verb → MealIdea (not HealthyProduct)
    #[test] fn goal_action_meal_ru() {
        assert_eq!(detect_intent("хочу на массу, что приготовить?"), Intent::MealIdea);
    }
    #[test] fn goal_action_meal_en() {
        assert_eq!(detect_intent("low calorie dinner for diet"), Intent::MealIdea);
    }
    #[test] fn goal_action_eat_ru() {
        assert_eq!(detect_intent("хочу похудеть, что поесть на ужин?"), Intent::MealIdea);
    }
    #[test] fn goal_action_muscle_en() {
        assert_eq!(detect_intent("high protein meal for dinner"), Intent::MealIdea);
    }
    #[test] fn goal_action_diet_lunch() {
        assert_eq!(detect_intent("diet lunch ideas"), Intent::MealIdea);
    }
}
