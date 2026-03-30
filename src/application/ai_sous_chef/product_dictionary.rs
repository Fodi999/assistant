//! Product Dictionary — single source of truth for names, types, units, defaults.
//!
//! AI is NOT the brain. Dictionary + lookup tables = truth.
//! AI is only a helper for descriptions + nutrition values.
//!
//! Input normalization: "fresh salmon fillet" → "salmon"
//! Validation: reject garbage AI names before saving to dictionary.

use serde::Serialize;

// ── Resolved product info (from DB dictionary + lookup tables) ───────

/// Everything we know about a product WITHOUT asking AI
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedProduct {
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub product_type: String,
    pub unit: String,
    pub density_g_per_ml: Option<f64>,
    pub typical_portion_g: Option<f64>,
    pub shelf_life_days: Option<i32>,
    /// true = names came from AI translation (pending in dictionary), false = dictionary hit
    pub names_from_ai: bool,
}

// ══════════════════════════════════════════════════════════════════════
// INPUT NORMALIZATION — strip adjectives, percentages, weights
// "fresh salmon fillet" → "salmon"
// "milk 3.2% 1l" → "milk"
// "куриное филе" → "курица"
// ══════════════════════════════════════════════════════════════════════

/// Normalize input to base ingredient name for dictionary lookup.
/// Strips adjectives, percentages, weights, fillet/fresh/raw.
pub fn normalize_ingredient_name(input: &str) -> String {
    let input = input.trim().to_lowercase();

    // Remove patterns: "3.2%", "1l", "500g", "1kg", etc.
    let re_numbers = regex_lite::Regex::new(r"\d+[.,]?\d*\s*(%|ml|l|g|kg|oz|lb)\b").unwrap();
    let cleaned = re_numbers.replace_all(&input, "");

    // English stop-words (adjectives that pollute dictionary)
    let en_stop = [
        "fresh", "raw", "dried", "frozen", "organic", "natural", "whole",
        "fillet", "fillets", "boneless", "skinless", "smoked", "salted",
        "canned", "pickled", "grilled", "fried", "boiled", "steamed",
        "ground", "minced", "sliced", "chopped", "diced", "grated",
        "large", "small", "medium", "extra", "premium", "quality",
        "baby", "young", "wild", "farm", "farmed",
    ];

    // Russian stop-words
    let ru_stop = [
        "свежий", "свежая", "свежее", "свежие",
        "сырой", "сырая", "сырое", "сырые",
        "сушёный", "сушёная", "сушёное", "сушёные",
        "замороженный", "замороженная", "замороженное", "замороженные",
        "филе", "фарш", "кусок", "кусочки",
        "копчёный", "копчёная", "копчёное", "копчёные",
        "солёный", "солёная", "солёное", "солёные",
        "куриное", "куриная", "куриный", "куриные",
        "говяжий", "говяжья", "говяжье", "говяжьи",
        "свиной", "свиная", "свиное", "свиные",
    ];

    let mut words: Vec<&str> = cleaned.split_whitespace().collect();

    // Remove stop-words
    words.retain(|w| {
        !en_stop.iter().any(|s| w == s)
            && !ru_stop.iter().any(|s| w == s)
    });

    let result = words.join(" ").trim().to_string();

    // If everything was stripped, return original trimmed input
    if result.is_empty() {
        input.trim().to_string()
    } else {
        // Capitalize first letter
        let mut chars = result.chars();
        match chars.next() {
            None => result,
            Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// AI NAME VALIDATION — reject garbage before saving to dictionary
// ══════════════════════════════════════════════════════════════════════

/// Validate AI-generated ingredient name. Returns true if name is acceptable.
/// Rejects: too long, contains numbers/%, adjectives, empty, garbage.
pub fn is_valid_ingredient_name(name: &str) -> bool {
    let name = name.trim();

    // Must not be empty
    if name.is_empty() {
        return false;
    }

    // Must be <= 40 chars (ingredient names are short)
    if name.len() > 40 {
        return false;
    }

    // Must not contain numbers or %
    if name.chars().any(|c| c.is_ascii_digit()) || name.contains('%') {
        return false;
    }

    // Must not contain forbidden English words (adjectives/modifiers)
    let forbidden_en = [
        "fillet", "fresh", "raw", "frozen", "organic", "dried",
        "smoked", "grilled", "fried", "boiled", "boneless", "skinless",
        "canned", "pickled", "ground", "minced", "sliced",
    ];
    let lower = name.to_lowercase();
    if forbidden_en.iter().any(|w| lower.contains(w)) {
        return false;
    }

    // Must not contain forbidden Russian words
    let forbidden_ru = ["филе", "фарш", "кусок", "копчён", "солён", "сушён"];
    if forbidden_ru.iter().any(|w| lower.contains(w)) {
        return false;
    }

    // Must not be a single character
    if name.chars().count() < 2 {
        return false;
    }

    true
}

// ── Default density by product type ─────────────────────────────────

pub fn default_density(product_type: &str) -> Option<f64> {
    match product_type {
        "fish"    => Some(1.05),
        "seafood" => Some(1.05),
        "meat"    => Some(1.05),
        "poultry" => Some(1.05),
        "dairy"   => Some(1.03),
        "vegetable" => Some(0.90),
        "fruit"     => Some(0.95),
        "grain"     => Some(0.75),
        "legume"    => Some(0.80),
        "nut"       => Some(0.65),
        "spice"     => Some(0.55),
        "oil"       => Some(0.92),
        "beverage"  => Some(1.00),
        _ => None,
    }
}

// ── Default shelf life by product type (days) ───────────────────────

pub fn default_shelf_life(product_type: &str) -> Option<i32> {
    match product_type {
        "fish"      => Some(3),
        "seafood"   => Some(3),
        "meat"      => Some(5),
        "poultry"   => Some(3),
        "dairy"     => Some(14),
        "vegetable" => Some(7),
        "fruit"     => Some(7),
        "grain"     => Some(365),
        "legume"    => Some(365),
        "nut"       => Some(180),
        "spice"     => Some(730),
        "oil"       => Some(365),
        "beverage"  => Some(7),
        _ => None,
    }
}

// ── Default typical portion by product type (grams) ─────────────────

pub fn default_portion(product_type: &str) -> Option<f64> {
    match product_type {
        "fish"      => Some(150.0),
        "seafood"   => Some(150.0),
        "meat"      => Some(150.0),
        "poultry"   => Some(150.0),
        "dairy"     => Some(200.0),
        "vegetable" => Some(150.0),
        "fruit"     => Some(150.0),
        "grain"     => Some(80.0),
        "legume"    => Some(80.0),
        "nut"       => Some(30.0),
        "spice"     => Some(5.0),
        "oil"       => Some(15.0),
        "beverage"  => Some(250.0),
        _ => None,
    }
}

// ── Unit by product type ────────────────────────────────────────────

pub fn unit_for_type(product_type: &str, name_en_lower: &str) -> &'static str {
    // Eggs are special
    if name_en_lower.contains("egg") {
        return "pcs";
    }
    match product_type {
        "oil" | "beverage" => "l",
        "dairy" => {
            if name_en_lower.contains("milk")
                || name_en_lower.contains("cream")
                || name_en_lower.contains("kefir")
                || name_en_lower.contains("yogurt")
            {
                "l"
            } else {
                "kg"
            }
        }
        "spice" => "g",
        _ => "kg",
    }
}

// ── Product type inference from name keywords ───────────────────────

/// Infer product_type from English + Russian name keywords.
/// Returns None if no strong match — caller must fall back to "other".
pub fn infer_product_type(name_en_lower: &str, name_ru_lower: &str) -> Option<&'static str> {
    // Fish
    let fish_kw = [
        "salmon", "tuna", "cod", "trout", "carp", "herring", "mackerel",
        "sardine", "bass", "perch", "pike", "catfish", "tilapia", "halibut",
        "swordfish", "anchovy", "crucian", "bream", "zander", "walleye",
        "haddock", "sole", "eel", "sturgeon", "pollock", "flounder",
        "snapper", "grouper", "mullet", "char",
    ];
    let fish_kw_ru = [
        "лосось", "тунец", "треска", "форель", "карп", "карась", "сельдь",
        "скумбрия", "сардина", "окунь", "щука", "сом", "тилапия", "палтус",
        "анчоус", "лещ", "судак", "сёмга", "горбуша", "кета", "минтай",
        "пикша", "камбала", "угорь", "осётр", "хек", "навага", "ставрида",
        "дорадо", "сибас", "муксун", "налим", "голец", "кефаль", "корюшка",
        "вугор",
    ];
    if fish_kw.iter().any(|k| name_en_lower.contains(k))
        || fish_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("fish");
    }

    // Seafood (shellfish)
    let seafood_kw = [
        "shrimp", "prawn", "lobster", "crab", "squid", "octopus", "mussel",
        "oyster", "clam", "scallop", "calamari",
    ];
    let seafood_kw_ru = [
        "креветка", "лобстер", "краб", "кальмар", "осьминог", "мидия",
        "устрица", "гребешок",
    ];
    if seafood_kw.iter().any(|k| name_en_lower.contains(k))
        || seafood_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("seafood");
    }

    // Meat
    let meat_kw = [
        "beef", "pork", "lamb", "veal", "rabbit", "venison", "bacon", "ham",
        "sausage", "salami", "prosciutto", "brisket", "steak",
    ];
    let meat_kw_ru = [
        "говядина", "свинина", "баранина", "телятина", "кролик", "оленина",
        "бекон", "ветчина", "колбаса", "фарш",
    ];
    if meat_kw.iter().any(|k| name_en_lower.contains(k))
        || meat_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("meat");
    }

    // Poultry
    let poultry_kw = [
        "chicken", "turkey", "duck", "goose", "quail",
    ];
    let poultry_kw_ru = [
        "курица", "курин", "индейка", "утка", "гусь", "перепел",
        "грудка", "бедро куриное", "филе куриное",
    ];
    if poultry_kw.iter().any(|k| name_en_lower.contains(k))
        || poultry_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("poultry");
    }

    // Dairy
    let dairy_kw = [
        "milk", "cheese", "yogurt", "yoghurt", "butter", "cream", "kefir",
        "cottage", "ricotta", "mozzarella", "cheddar", "parmesan", "brie",
        "camembert", "feta", "gouda", "mascarpone",
    ];
    let dairy_kw_ru = [
        "молоко", "сыр", "йогурт", "масло сливочное", "сливки", "кефир",
        "творог", "сметана", "рикотта", "моцарелла", "ряженка",
    ];
    if dairy_kw.iter().any(|k| name_en_lower.contains(k))
        || dairy_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("dairy");
    }

    // Egg
    if name_en_lower.contains("egg") && !name_en_lower.contains("eggplant")
        || name_ru_lower.contains("яйц")
    {
        return Some("egg");
    }

    // Fruit
    let fruit_kw = [
        "apple", "banana", "orange", "grape", "lemon", "lime", "mango",
        "peach", "pear", "plum", "cherry", "strawberry", "blueberry",
        "raspberry", "watermelon", "melon", "kiwi", "pineapple", "coconut",
        "apricot", "fig", "pomegranate", "papaya", "passion fruit",
        "avocado", "grapefruit", "tangerine", "nectarine", "cranberry",
        "blackberry", "currant", "gooseberry",
    ];
    let fruit_kw_ru = [
        "яблоко", "банан", "апельсин", "виноград", "лимон", "лайм",
        "манго", "персик", "груша", "слива", "вишня", "клубника",
        "черника", "малина", "арбуз", "дыня", "киви", "ананас", "кокос",
        "абрикос", "инжир", "гранат", "авокадо", "грейпфрут", "мандарин",
        "нектарин", "клюква", "ежевика", "смородина", "крыжовник",
    ];
    if fruit_kw.iter().any(|k| name_en_lower.contains(k))
        || fruit_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("fruit");
    }

    // Oil (MUST be checked BEFORE vegetable — "olive oil" contains "olive")
    let oil_kw = ["olive oil", "sunflower oil", "coconut oil", "sesame oil",
                   "canola oil", "vegetable oil", "avocado oil", "flaxseed oil",
                   "rapeseed oil", "peanut oil", "corn oil", "grapeseed oil",
                   "truffle oil", "walnut oil"];
    let oil_kw_ru = ["оливковое масло", "подсолнечное масло", "кокосовое масло",
                      "кунжутное масло", "рапсовое масло", "льняное масло",
                      "растительное масло", "арахисовое масло", "кукурузное масло",
                      "трюфельное масло"];
    if oil_kw.iter().any(|k| name_en_lower.contains(k))
        || oil_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("oil");
    }

    // Vegetable
    let veg_kw = [
        "carrot", "potato", "tomato", "onion", "garlic", "pepper", "bell pepper",
        "cucumber", "cabbage", "broccoli", "spinach", "lettuce", "celery",
        "zucchini", "eggplant", "corn", "peas", "beet", "radish", "turnip",
        "asparagus", "artichoke", "cauliflower", "kale", "leek",
        "pumpkin", "squash", "sweet potato", "olive", "caper",
    ];
    let veg_kw_ru = [
        "морковь", "картофель", "помидор", "лук", "чеснок", "перец",
        "огурец", "капуста", "брокколи", "шпинат", "салат", "сельдерей",
        "кабачок", "баклажан", "кукуруза", "горох", "свёкла", "редис",
        "спаржа", "артишок", "цветная капуста", "тыква",
        "оливк", "маслин", "каперс",
    ];
    if veg_kw.iter().any(|k| name_en_lower.contains(k))
        || veg_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("vegetable");
    }

    // Grain
    let grain_kw = [
        "rice", "wheat", "oat", "barley", "buckwheat", "quinoa",
        "pasta", "noodle", "bread", "flour", "couscous", "bulgur",
        "millet", "rye", "semolina", "cornmeal",
    ];
    let grain_kw_ru = [
        "рис", "пшеница", "овёс", "ячмень", "гречка", "киноа",
        "паста", "макароны", "хлеб", "мука", "кускус", "булгур",
        "пшено", "рожь", "манка",
    ];
    if grain_kw.iter().any(|k| name_en_lower.contains(k))
        || grain_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("grain");
    }

    // Legume
    let legume_kw = [
        "lentil", "chickpea", "bean", "soybean", "tofu", "tempeh",
        "edamame", "mung",
    ];
    let legume_kw_ru = [
        "чечевица", "нут", "фасоль", "соя", "тофу", "темпе", "маш",
    ];
    if legume_kw.iter().any(|k| name_en_lower.contains(k))
        || legume_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("legume");
    }

    // Nut
    let nut_kw = [
        "almond", "walnut", "cashew", "pistachio", "hazelnut", "pecan",
        "macadamia", "brazil nut", "pine nut", "chestnut",
        "sunflower seed", "pumpkin seed", "chia", "flax", "sesame",
    ];
    let nut_kw_ru = [
        "миндаль", "грецкий", "кешью", "фисташк", "фундук", "пекан",
        "макадамия", "кедровый", "каштан", "семечк", "чиа", "лён", "кунжут",
    ];
    if nut_kw.iter().any(|k| name_en_lower.contains(k))
        || nut_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("nut");
    }

    // Spice (includes baking agents — they live in "Spices & Herbs" category)
    let spice_kw = [
        "cinnamon", "turmeric", "cumin", "paprika", "basil", "oregano",
        "thyme", "rosemary", "dill", "parsley", "bay leaf", "saffron",
        "cardamom", "clove", "nutmeg", "ginger", "vanilla", "coriander",
        "mint", "sage", "tarragon", "chili",
        "baking powder", "baking soda", "yeast", "gelatin", "agar",
        "cornstarch", "starch", "pectin",
    ];
    let spice_kw_ru = [
        "корица", "куркума", "тмин", "паприка", "базилик", "орегано",
        "тимьян", "розмарин", "укроп", "петрушка", "лавр", "шафран",
        "кардамон", "гвоздика", "мускатный", "имбирь", "ваниль", "кориандр",
        "мята", "шалфей", "эстрагон", "чили",
        "разрыхлитель", "сода", "дрожжи", "желатин", "агар",
        "крахмал", "пектин",
    ];
    if spice_kw.iter().any(|k| name_en_lower.contains(k))
        || spice_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("spice");
    }

    // Sweetener / Condiment / Sweets
    let condiment_kw = ["honey", "sugar", "maple syrup", "soy sauce", "vinegar",
                         "mustard", "ketchup", "mayonnaise", "hot sauce", "worcestershire",
                         "chocolate", "cocoa bean", "molasses", "syrup", "stevia"];
    let condiment_kw_ru = ["мёд", "сахар", "кленовый сироп", "соевый соус", "уксус",
                            "горчица", "кетчуп", "майонез", "шоколад", "патока", "сироп"];
    if condiment_kw.iter().any(|k| name_en_lower.contains(k))
        || condiment_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("condiment");
    }

    // Bakery / Bread
    let bakery_kw = ["bread", "baguette", "bun", "croissant", "wrap", "tortilla", "pita", "bagel"];
    let bakery_kw_ru = ["хлеб", "багет", "булочка", "круассан", "лепешка", "тортилья", "питак"];
    if bakery_kw.iter().any(|k| name_en_lower.contains(k))
        || bakery_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("bread");
    }

    // Beverage
    let beverage_kw = [
        "beer", "wine", "juice", "tea", "coffee", "cocoa", "lemonade",
        "kombucha", "kvass", "cider", "sake", "mead", "smoothie",
        "matcha", "espresso", "water",
    ];
    let beverage_kw_ru = [
        "пиво", "вино", "сок", "чай", "кофе", "какао", "лимонад",
        "комбуча", "квас", "сидр", "сакэ", "медовуха",
    ];
    if beverage_kw.iter().any(|k| name_en_lower.contains(k))
        || beverage_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("beverage");
    }

    // Mushroom (before vegetable, so it doesn't get caught by veg "mushroom")
    let mushroom_kw = [
        "mushroom", "champignon", "shiitake", "portobello", "oyster mushroom",
        "chanterelle", "porcini", "truffle", "enoki", "maitake", "morel",
    ];
    let mushroom_kw_ru = [
        "гриб", "шампиньон", "шиитаке", "портобелло", "вёшенка",
        "лисичк", "белый гриб", "трюфель", "опят", "маслят",
    ];
    if mushroom_kw.iter().any(|k| name_en_lower.contains(k))
        || mushroom_kw_ru.iter().any(|k| name_ru_lower.contains(k))
    {
        return Some("mushroom");
    }

    None // truly unknown
}
