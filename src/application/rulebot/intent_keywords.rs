//! Intent Keywords — DDD data layer for intent detection.
//!
//! All keyword tables live here. The intent_router imports them and
//! runs scoring logic. This keeps data separate from algorithms.
//!
//! Each table: `&[(&str, i32)]` — keyword + weight.
//! Higher weight = stronger signal for that intent.
//!
//! ```text
//! intent_keywords.rs  → WHAT words mean (data)
//! intent_router.rs    → HOW to score/route (logic)
//! goal_modifier.rs    → WHICH goal the user wants (context)
//! ```

/// A scored keyword: (keyword, weight).
pub type ScoredKeyword = (&'static str, i32);

// ═══════════════════════════════════════════════════════════════════════════════
//  GREETING
// ═══════════════════════════════════════════════════════════════════════════════

pub const GREETING: &[ScoredKeyword] = &[
    ("привет",        3), ("здравствуй",    3), ("hello",          3),
    ("cześć",         3), ("вітаю",          3), ("привіт",         3),
    ("hej",           3), ("witam",           3),
    ("добрый день",   2), ("добрый вечер",   2), ("доброе утро",    2),
    ("good morning",  2), ("good evening",   2), ("good afternoon", 2),
    ("dzień dobry",   2), ("добрий день",    2),
    ("хай",           1), ("здарова",        1), ("hey",            1),
    ("hi ",           1), ("yo ",            1),
];

// ═══════════════════════════════════════════════════════════════════════════════
//  CONVERSION
// ═══════════════════════════════════════════════════════════════════════════════

pub const CONVERSION: &[ScoredKeyword] = &[
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

// ═══════════════════════════════════════════════════════════════════════════════
//  NUTRITION INFO
// ═══════════════════════════════════════════════════════════════════════════════

pub const NUTRITION: &[ScoredKeyword] = &[
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

// ═══════════════════════════════════════════════════════════════════════════════
//  HEALTHY PRODUCT
// ═══════════════════════════════════════════════════════════════════════════════

pub const HEALTHY: &[ScoredKeyword] = &[
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
    // ── New: comfort / fiber / lowcarb signals that also show products ──
    ("сытн",          3), ("comfort food",   3), ("сытное",         3),
    ("клетчатк",      3), ("fiber",          3), ("high fiber",     3),
    ("błonnik",       3), ("клітковин",      3),
    ("без углевод",   3), ("low carb",       3), ("кето",           3),
    ("keto",          3), ("bez węglowodan", 3), ("без вуглевод",   3),
];

// ═══════════════════════════════════════════════════════════════════════════════
//  SEASONALITY
// ═══════════════════════════════════════════════════════════════════════════════

pub const SEASONALITY: &[ScoredKeyword] = &[
    ("сезон",         3), ("season",         3), ("sezon",          3),
    ("в сезоне",      3), ("in season",      3),
    ("сезонный",      2), ("seasonal",       2), ("sezonow",        2),
    ("когда сезон",   2), ("when is season", 2), ("kiedy sezon",    2),
    ("коли сезон",    2),
    ("свежий",        1), ("fresh",          1), ("świeży",         1),
    ("урожай",        1), ("harvest",        1),
];

// ═══════════════════════════════════════════════════════════════════════════════
//  RECIPE HELP
// ═══════════════════════════════════════════════════════════════════════════════

pub const RECIPE: &[ScoredKeyword] = &[
    ("рецепт",        3), ("recipe",         3), ("przepis",        3),
    ("рецепти",       3),
    ("как приготовить",2), ("how to cook",   2), ("jak ugotować",   2),
    ("jak przygotować",2), ("як приготувати",2),
    ("как сделать",   2), ("how to make",    2),
    ("приготовление", 2), ("cooking",        2), ("gotowanie",      2),
    ("ингредиент",    1), ("ingredient",     1), ("składnik",       1),
    ("шаги",          1), ("steps",          1),
    // ── Imperative verbs (RU) — "приготовь борщ", "сделай салат" ──
    ("приготовь",     3), ("сделай",         3), ("свари",          3),
    ("пожарь",        3), ("потуши",         3), ("запеки",         3),
    ("сготовь",       3), ("зажарь",         3),
    // ── Imperative verbs (UK) ──
    ("приготуй",      3), ("зроби",          3), ("звари",          3),
    ("підсмаж",       3), ("потуш",          3), ("запечи",         3),
    // ── Imperative verbs (EN) ──
    ("cook ",         2), ("make ",          2), ("prepare ",       2),
    ("bake ",         2), ("grill ",         2), ("fry ",           2),
    ("stew ",         2), ("roast ",         2), ("boil ",          2),
    // ── Imperative verbs (PL) ──
    ("ugotuj",        3), ("zrób",           3), ("usmaż",         3),
    ("upiecz",        3), ("przygotuj",      3),
    // ── Goal-based recipe queries (from suggestions) ──
    ("лёгкое блюдо",  3), ("лёгкий рецепт", 3), ("диетическ",    2),
    ("легку страву",   3), ("легкий рецепт",  3),
    ("light dish",     3), ("light recipe",   3),
    ("lekkie danie",   3), ("lekki przepis",  3),
    ("высокобелков",   2), ("high protein recipe", 3),
    ("блюдо с ",       2), ("dish with ",     2), ("danie z ",      2),
    ("страву з ",      2),
    // ── New: modifier-aware recipe queries ──
    ("быстрый рецепт", 3), ("quick recipe",   3), ("szybki przepis", 3),
    ("бюджетный рецепт",3), ("cheap recipe",  3), ("tani przepis",   3),
    ("сытное блюдо",   3), ("comfort dish",   3),
    ("без углеводов",  2), ("low carb dish",  2), ("keto recipe",    3),
    ("богат клетчатк", 2), ("high fiber dish", 2),
];

// ═══════════════════════════════════════════════════════════════════════════════
//  MEAL IDEA
// ═══════════════════════════════════════════════════════════════════════════════

pub const MEAL_IDEA: &[ScoredKeyword] = &[
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

// ═══════════════════════════════════════════════════════════════════════════════
//  PRODUCT INFO
// ═══════════════════════════════════════════════════════════════════════════════

pub const PRODUCT_INFO: &[ScoredKeyword] = &[
    ("что такое",     3), ("what is",        3), ("co to jest",     3),
    ("що таке",       3), ("расскажи о",     3), ("tell me about",  3),
    ("расскажи про",  3), ("опиши",          3), ("describe",       3),
    ("что это",       2), ("what's",         2), ("czym jest",      2),
    ("о продукте",    2), ("about ",         2),
    ("подробнее о",   2), ("more about",     2),
    ("инфо",          1), ("info",           1), ("данные",         1),
    ("informacje",    1),
];

// ═══════════════════════════════════════════════════════════════════════════════
//  CONTEXT BOOST — goal/action/meal-time signals
// ═══════════════════════════════════════════════════════════════════════════════

/// Goal keywords — detected in the text to enable context boosts.
pub const GOAL_SIGNALS: &[&str] = &[
    "на массу", "похуд", "сушк", "диет", "diet", "muscle",
    "lose weight", "bulk", "набрать", "high protein", "low calorie", "low cal",
    "schudnąć", "na masę", "схуднути", "на білок", "на белок",
    // New goals
    "сытн", "comfort", "клетчатк", "fiber", "błonnik",
    "кето", "keto", "low carb", "без углевод", "без вуглевод",
    "лёгк", "легк", "light",
];

/// Action verbs — "что поесть", "приготовить"
pub const ACTION_SIGNALS: &[&str] = &[
    "поесть", "приготовить", "готовить", "cook", "eat", "make",
    "zjeść", "ugotować", "поїсти", "приготувати", "съесть", "кушать",
];

/// Meal-time words — "ужин", "обед", "dinner"
pub const MEAL_TIME_SIGNALS: &[&str] = &[
    "ужин", "обед", "завтрак", "dinner", "lunch", "breakfast",
    "kolacja", "obiad", "śniadanie", "вечеря", "обід", "сніданок",
];
