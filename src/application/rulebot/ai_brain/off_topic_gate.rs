//! Off-Topic Gate — 3-tier filter BEFORE LLM.
//!
//! Tier 1: LowQuality   — gibberish, spam, insults, tests → instant static reply, 0ms, $0
//! Tier 2: OutOfScope    — real questions but NOT about food → instant redirect, 0ms, $0
//! Tier 3: Borderline    — food-adjacent but outside our tools → smart redirect, 0ms, $0
//!
//! Only if all 3 tiers pass → we call LLM (Layer 2 AI Brain).
//! This saves ~20-30% of LLM calls.

use super::response_helpers::{redirect_suggestions, borderline_price_suggestions};
use crate::application::rulebot::intent_router::ChatLang;
use crate::application::rulebot::chat_response::ChatResponse;

/// Classification result for off-topic detection.
#[derive(Debug, PartialEq)]
pub(crate) enum OffTopicTier {
    /// Input is valid and may be food-related → proceed to AI Brain.
    Pass,
    /// Tier 1: Gibberish, spam, insults, test messages.
    LowQuality,
    /// Tier 2: Real question, completely unrelated to food/cooking.
    OutOfScope,
    /// Tier 3: Food-adjacent but outside our tools (price, restaurant, delivery).
    Borderline,
}

/// Classify input into one of 3 off-topic tiers or Pass.
/// Pure keyword/heuristic — zero LLM, zero async, zero cost.
pub(crate) fn classify_off_topic(input: &str) -> OffTopicTier {
    let t = input.to_lowercase();
    let trimmed = t.trim();

    // ── Tier 1: LowQuality ───────────────────────────────────────────────
    // Short gibberish
    if trimmed.len() < 2 {
        return OffTopicTier::LowQuality;
    }
    // Only non-alphabetic characters (excluding digits for things like "200г")
    let alpha_count = trimmed.chars().filter(|c| c.is_alphabetic()).count();
    if alpha_count == 0 && !trimmed.chars().any(|c| c.is_ascii_digit()) {
        return OffTopicTier::LowQuality;
    }
    // Very short with no food keywords
    if trimmed.len() < 4 && !has_any_food_keyword(&t) {
        return OffTopicTier::LowQuality;
    }
    // Insults, trolling, nonsense
    let garbage_patterns: &[&str] = &[
        "ты тупой", "ты дурак", "ты идиот", "ты бот", "ты глупый",
        "you're stupid", "you suck", "you're dumb", "you are stupid",
        "jesteś głupi", "jesteś durny",
        "ти тупий", "ти дурний",
        "лол", "lol", "хаха", "haha", "хехе", "hehe",
        "тест", "test", "проверка", "check",
        "абвг", "abcd", "qwer", "asdf", "йцук",
        "ааа", "zzz", "ооо",
    ];
    for pat in garbage_patterns {
        if trimmed == *pat || trimmed.starts_with(pat) {
            return OffTopicTier::LowQuality;
        }
    }
    // Repeated characters (ааааа, !!!!!!)
    if trimmed.len() >= 4 {
        let first_char = trimmed.chars().next().unwrap();
        if trimmed.chars().all(|c| c == first_char || c.is_whitespace()) {
            return OffTopicTier::LowQuality;
        }
    }

    // ── Tier 3: Borderline (check BEFORE OutOfScope, because these have food words) ──
    let borderline_patterns: &[&str] = &[
        // Price / cost
        "сколько стоит", "какая цена", "цена на", "почём",
        "how much does", "how much is", "what's the price", "price of",
        "ile kosztuje", "jaka cena", "cena za",
        "скільки коштує", "яка ціна",
        // Where to buy / restaurants
        "где купить", "где найти", "где продаётся", "где заказать",
        "where to buy", "where can i get", "where to find", "where to order",
        "gdzie kupić", "gdzie znaleźć", "gdzie zamówić",
        "де купити", "де знайти", "де замовити",
        // Restaurant / delivery
        "какой ресторан", "лучший ресторан", "доставка", "доставить",
        "best restaurant", "food delivery", "order food", "restaurant",
        "najlepsza restauracja", "dostawa jedzenia", "zamówić jedzenie",
        "найкращий ресторан", "доставка їжі", "замовити їжу",
    ];
    for pat in borderline_patterns {
        if t.contains(pat) {
            return OffTopicTier::Borderline;
        }
    }

    // ── Tier 2: OutOfScope ───────────────────────────────────────────────
    // If input has any food keyword → Pass (let AI Brain handle)
    if has_any_food_keyword(&t) {
        return OffTopicTier::Pass;
    }

    // Known non-food topics
    let out_of_scope_patterns: &[&str] = &[
        // Weather
        "погод", "weather", "temperatura", "дождь", "rain", "солнце", "sun",
        // Politics / news
        "президент", "president", "prezydent", "политик", "politic", "polity",
        "выборы", "election", "wybory", "вибори",
        "война", "war", "wojna", "війна",
        "новости", "news", "wiadomości", "новини",
        // Money / finance (non-food)
        "бензин", "gasoline", "benzyna", "бензин",
        "биткоин", "bitcoin", "crypto", "крипто",
        "акции", "stock", "akcje", "акції",
        "курс доллар", "exchange rate", "kurs walut", "курс валют",
        "зарплат", "salary", "pensja", "зарплат",
        // Tech
        "телефон", "phone", "iphone", "android", "компьютер", "computer",
        "програм", "software", "program",
        // Sports (non-nutrition)
        "футбол", "football", "soccer", "basketball", "баскетбол",
        "хоккей", "hockey", "матч", "match",
        // Entertainment
        "фильм", "movie", "film", "сериал", "serial", "игр", "game",
        "музык", "music", "muzyk", "песн", "song",
        // Other
        "машин", "car", "samochod", "авто",
        "квартир", "apartment", "mieszkan",
        "поезд", "train", "самолёт", "airplane", "flight",
    ];
    for pat in out_of_scope_patterns {
        if t.contains(pat) {
            return OffTopicTier::OutOfScope;
        }
    }

    // Default: let AI Brain handle it — might be a complex food question
    OffTopicTier::Pass
}

/// Quick check: does the input contain ANY food/cooking/nutrition keyword?
fn has_any_food_keyword(text: &str) -> bool {
    let food_keywords: &[&str] = &[
        // Food types (RU)
        "еда", "еду", "блюд", "продукт", "ингредиент", "овощ", "фрукт",
        "мясо", "рыб", "молок", "масло", "крупа", "хлеб", "сахар", "соль",
        "яйц", "сыр", "творог", "каш", "суп", "салат", "соус",
        // Cooking (RU)
        "готов", "варить", "жарить", "печь", "запек", "тушить", "рецепт",
        "кухн", "кулинар", "приготов", "нарезать", "смешать",
        // Nutrition (RU)
        "калори", "ккал", "белок", "белка", "жир", "углевод", "витамин",
        "питан", "диет", "похуд", "мышц", "сушк", "протеин",
        "кбжу", "нутриент", "пищев",
        // Products (RU) — common
        "курица", "курятин", "лосось", "шпинат", "брокколи", "миндаль",
        "помидор", "картофель", "картошк", "морковь", "лук", "чеснок",
        "рис", "говядина", "свинина", "бургер", "пицца", "макарон", "паста",
        // Food types (EN)
        "food", "dish", "product", "ingredient", "vegetable", "fruit",
        "meat", "fish", "milk", "butter", "bread", "sugar", "salt",
        "egg", "cheese", "soup", "salad", "sauce", "meal",
        // Cooking (EN)
        "cook", "bake", "fry", "roast", "grill", "boil", "recipe",
        "kitchen", "culinary", "prepare", "chop", "mix",
        // Nutrition (EN)
        "calori", "kcal", "protein", "fat", "carb", "vitamin",
        "nutrition", "diet", "weight loss", "muscle",
        // Products (EN) — common
        "chicken", "salmon", "spinach", "broccoli", "almond",
        "tomato", "potato", "rice", "beef", "pork", "burger", "pizza",
        // Food types (PL)
        "jedzeni", "potraw", "produkt", "składnik", "warzywo", "owoc",
        "mięso", "ryba", "mleko", "masło", "chleb", "cukier", "sól",
        "jajk", "ser", "zupa", "sałat", "sos", "posiłek",
        // Cooking (PL)
        "gotowa", "piec", "smażyć", "grillować", "przepis",
        "kuchni", "kulinar", "przygotow",
        // Nutrition (PL)
        "kalori", "kcal", "białk", "tłuszcz", "węglowodan", "witamin",
        "odżywcz", "diet", "odchudzani",
        // Food types (UK)
        "їжа", "їжу", "страва", "продукт", "інгредієнт", "овочі", "фрукти",
        "мясо", "риба", "молоко", "масло", "хліб", "цукор", "сіль",
        "яйце", "сир", "каша", "суп", "салат",
        // Cooking (UK)
        "готувати", "варити", "смажити", "пекти", "запікати", "рецепт",
        "кухня", "кулінар", "приготувати",
        // Nutrition (UK)
        "калорі", "ккал", "білок", "білка", "жир", "вуглевод", "вітамін",
        "харчуван", "дієт", "схудн",
        // Units (universal)
        "грам", "gram", "кг", "kg", "мл", "ml", "литр", "liter",
        "ложк", "tbsp", "tsp", "стакан", "cup", "унци", "oz",
    ];

    food_keywords.iter().any(|kw| text.contains(kw))
}

// ── Static responses ─────────────────────────────────────────────────────────

/// Build a static response for LowQuality input — no LLM, no cost.
pub(crate) fn respond_low_quality(lang: ChatLang) -> ChatResponse {
    let text = match lang {
        ChatLang::Ru => "Я кулинарный помощник ChefOS 🍳\n\nМогу помочь с блюдами, продуктами и расчётами.\n\nНапример: «что приготовить из курицы?»",
        ChatLang::En => "I'm ChefOS — your culinary assistant 🍳\n\nI can help with recipes, products and nutrition.\n\nTry: \"what can I cook with chicken?\"",
        ChatLang::Pl => "Jestem ChefOS — Twój asystent kulinarny 🍳\n\nMogę pomóc z przepisami, produktami i obliczeniami.\n\nNp: «co ugotować z kurczaka?»",
        ChatLang::Uk => "Я кулінарний помічник ChefOS 🍳\n\nМожу допомогти з рецептами, продуктами та розрахунками.\n\nНаприклад: «що приготувати з курки?»",
    };

    let mut resp = ChatResponse::text_only(
        text,
        crate::application::rulebot::intent_router::Intent::Unknown,
        lang,
        0,
    );
    resp.suggestions = redirect_suggestions(lang);
    resp
}

/// Build a static response for OutOfScope input — redirect to our domain.
pub(crate) fn respond_out_of_scope(lang: ChatLang) -> ChatResponse {
    let text = match lang {
        ChatLang::Ru => "Я специализируюсь на кулинарии и продуктах 🧑‍🍳\n\nМогу помочь с:\n• подбором рецептов\n• расчётом КБЖУ\n• составлением меню\n• конвертацией единиц\n\nНапример: «что полезного поесть?»",
        ChatLang::En => "I specialize in food and cooking 🧑‍🍳\n\nI can help with:\n• finding recipes\n• nutrition data (calories, protein)\n• meal planning\n• unit conversion\n\nTry: \"healthy meal ideas\"",
        ChatLang::Pl => "Specjalizuję się w kuchni i produktach 🧑‍🍳\n\nMogę pomóc z:\n• doborem przepisów\n• obliczeniem makroskładników\n• ułożeniem menu\n• przeliczaniem jednostek\n\nNp: «co zdrowego zjeść?»",
        ChatLang::Uk => "Я спеціалізуюся на кулінарії та продуктах 🧑‍🍳\n\nМожу допомогти з:\n• підбором рецептів\n• розрахунком КБЖУ\n• складанням меню\n• конвертацією одиниць\n\nНаприклад: «що корисного з'їсти?»",
    };

    let mut resp = ChatResponse::text_only(
        text,
        crate::application::rulebot::intent_router::Intent::Unknown,
        lang,
        0,
    );
    resp.suggestions = redirect_suggestions(lang);
    resp
}

/// Build a smart response for Borderline input — don't reject, REDIRECT.
pub(crate) fn respond_borderline(input: &str, lang: ChatLang) -> ChatResponse {
    let t = input.to_lowercase();

    // Detect what kind of borderline: price / restaurant / delivery
    let is_price = t.contains("стоит") || t.contains("цена") || t.contains("price")
        || t.contains("kosztuje") || t.contains("cena") || t.contains("коштує")
        || t.contains("почём");
    let is_restaurant = t.contains("ресторан") || t.contains("restaurant") || t.contains("restauracj")
        || t.contains("кафе") || t.contains("cafe");
    let is_delivery = t.contains("доставк") || t.contains("deliver") || t.contains("dostaw")
        || t.contains("заказать") || t.contains("order") || t.contains("zamówi");

    let text = if is_price {
        match lang {
            ChatLang::Ru => "Цена зависит от магазина и региона 🏷️\n\nНо я могу:\n• рассчитать **себестоимость** блюда по ингредиентам\n• показать **калорийность**\n• предложить **рецепт**\n\nХочешь рассчитать себестоимость?",
            ChatLang::En => "Prices vary by store and region 🏷️\n\nBut I can:\n• calculate **ingredient cost** of a dish\n• show **nutrition data**\n• suggest a **recipe**\n\nWant me to calculate the cost?",
            ChatLang::Pl => "Cena zależy od sklepu i regionu 🏷️\n\nAle mogę:\n• obliczyć **koszt składników** potrawy\n• pokazać **wartości odżywcze**\n• zaproponować **przepis**\n\nChcesz obliczyć koszt?",
            ChatLang::Uk => "Ціна залежить від магазину та регіону 🏷️\n\nАле я можу:\n• розрахувати **собівартість** страви\n• показати **калорійність**\n• запропонувати **рецепт**\n\nХочеш розрахувати собівартість?",
        }
    } else if is_restaurant || is_delivery {
        match lang {
            ChatLang::Ru => "Я не работаю с ресторанами и доставкой 🍽️\n\nНо могу помочь приготовить **дома** — часто вкуснее и дешевле!\n\n• рецепты с расчётом КБЖУ\n• подбор ингредиентов\n• план питания\n\nЧто хочешь приготовить?",
            ChatLang::En => "I don't cover restaurants or delivery 🍽️\n\nBut I can help you cook **at home** — often tastier and cheaper!\n\n• recipes with nutrition data\n• ingredient selection\n• meal planning\n\nWhat would you like to cook?",
            ChatLang::Pl => "Nie zajmuję się restauracjami i dostawą 🍽️\n\nAle mogę pomóc gotować **w domu** — często smaczniej i taniej!\n\n• przepisy z makroskładnikami\n• dobór składników\n• plan posiłków\n\nCo chcesz ugotować?",
            ChatLang::Uk => "Я не працюю з ресторанами та доставкою 🍽️\n\nАле можу допомогти приготувати **вдома** — часто смачніше та дешевше!\n\n• рецепти з КБЖУ\n• підбір інгредієнтів\n• план харчування\n\nЩо хочеш приготувати?",
        }
    } else {
        // Generic borderline
        match lang {
            ChatLang::Ru => "Это немного за пределами моих навыков 🤔\n\nНо вот чем могу помочь:\n• **рецепты** — от простых до сложных\n• **КБЖУ** — калории, белки, жиры\n• **план питания** — на день или неделю\n\nПопробуй: «что приготовить на ужин?»",
            ChatLang::En => "That's a bit outside my expertise 🤔\n\nBut here's what I can do:\n• **recipes** — from simple to advanced\n• **nutrition** — calories, protein, macros\n• **meal plans** — daily or weekly\n\nTry: \"dinner ideas\"",
            ChatLang::Pl => "To trochę poza moimi umiejętnościami 🤔\n\nAle oto czym mogę pomóc:\n• **przepisy** — od prostych do zaawansowanych\n• **makroskładniki** — kalorie, białko, tłuszcz\n• **plan posiłków** — na dzień lub tydzień\n\nSpróbuj: «co ugotować na obiad?»",
            ChatLang::Uk => "Це трохи поза моїми навичками 🤔\n\nАле ось чим можу допомогти:\n• **рецепти** — від простих до складних\n• **КБЖУ** — калорії, білки, жири\n• **план харчування** — на день або тиждень\n\nСпробуй: «що приготувати на вечерю?»",
        }
    };

    let suggestions = if is_price {
        borderline_price_suggestions(lang)
    } else {
        redirect_suggestions(lang)
    };

    let mut resp = ChatResponse::text_only(
        text,
        crate::application::rulebot::intent_router::Intent::Unknown,
        lang,
        0,
    );
    resp.suggestions = suggestions;
    resp
}
