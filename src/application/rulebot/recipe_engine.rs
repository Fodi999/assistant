//! Recipe Engine v2 — minimal AI, maximal backend control.
//!
//! Philosophy:
//!   Gemini = "what dish, which ingredients"  (50–100 tokens)
//!   Backend = everything else: state, role, grams, yield, КБЖУ
//!
//! Flow:
//!   1. Gemini → `{"dish":"borscht","items":["beet","cabbage","potato",…]}`
//!   2. Backend resolves slugs from IngredientCache
//!   3. Backend assigns role (meal_role), cooking method, portion grams
//!   4. Backend computes gross/net/yield/КБЖУ deterministically
//!   5. Response builder renders recipe-view or tech-card

use serde::{Deserialize, Serialize};

use crate::infrastructure::IngredientCache;
use crate::infrastructure::ingredient_cache::IngredientData;
use crate::infrastructure::llm_adapter::LlmAdapter;
use super::intent_router::ChatLang;
use super::meal_builder::CookMethod;
use super::response_builder::HealthGoal;

// ── Dish Cooking Profile ─────────────────────────────────────────────────────

/// The type of dish determines how every ingredient is cooked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DishType {
    Soup,       // borscht, ramen, pho, minestrone…
    Stew,       // goulash, ragout, curry…
    Salad,      // caesar, greek…
    StirFry,    // wok, pad thai…
    Grill,      // bbq, steaks…
    Bake,       // casserole, lasagna, pizza…
    Pasta,      // spaghetti, carbonara…
    Raw,        // tartare, sashimi…
    Default,    // unknown → old behaviour
}

impl DishType {
    /// Detect dish type from the English dish name returned by Gemini.
    pub fn detect(dish: &str) -> Self {
        let d = dish.to_lowercase();
        // Soups
        if d.contains("soup") || d.contains("borscht") || d.contains("borsch")
            || d.contains("ramen") || d.contains("pho") || d.contains("minestrone")
            || d.contains("chowder") || d.contains("consomme") || d.contains("gazpacho")
            || d.contains("bouillon") || d.contains("broth") || d.contains("ukha")
            || d.contains("shchi") || d.contains("solyanka") || d.contains("rassolnik")
            || d.contains("kharcho") || d.contains("tom yum") || d.contains("laksa")
            || d.contains("miso") { return DishType::Soup; }
        // Stews
        if d.contains("stew") || d.contains("ragout") || d.contains("goulash")
            || d.contains("curry") || d.contains("chili con") || d.contains("tagine")
            || d.contains("casserole") || d.contains("pot roast")
            || d.contains("braised") { return DishType::Stew; }
        // Salads
        if d.contains("salad") || d.contains("ceviche")
            || d.contains("coleslaw") || d.contains("tabouleh") { return DishType::Salad; }
        // Stir-fry / wok
        if d.contains("stir") || d.contains("wok") || d.contains("pad thai")
            || d.contains("fried rice") || d.contains("chow mein") { return DishType::StirFry; }
        // Grill
        if d.contains("grill") || d.contains("bbq") || d.contains("kebab")
            || d.contains("shashlik") || d.contains("steak")
            || d.contains("burger") { return DishType::Grill; }
        // Bake
        if d.contains("bake") || d.contains("lasagna") || d.contains("pizza")
            || d.contains("quiche") || d.contains("pie")
            || d.contains("gratin") { return DishType::Bake; }
        // Pasta
        if d.contains("pasta") || d.contains("spaghetti") || d.contains("carbonara")
            || d.contains("penne") || d.contains("fettuccine")
            || d.contains("macaroni") || d.contains("noodle") { return DishType::Pasta; }
        // Raw
        if d.contains("tartare") || d.contains("sashimi")
            || d.contains("carpaccio") { return DishType::Raw; }
        DishType::Default
    }

    /// The cooking method for a given role in this dish type.
    pub fn cook_method(&self, role: &str, product_type: &str, goal: HealthGoal) -> CookMethod {
        match self {
            DishType::Soup => match role {
                "protein" => CookMethod::Boil,
                "side"    => CookMethod::Boil,
                "base"    => CookMethod::Boil,
                "spice" | "condiment" => CookMethod::Raw,
                _ => CookMethod::Boil,
            },
            DishType::Stew => match role {
                "protein" => CookMethod::Boil, // тушёное = long boil/braise
                "side"    => CookMethod::Boil,
                "base"    => CookMethod::Boil,
                "spice" | "condiment" => CookMethod::Raw,
                _ => CookMethod::Boil,
            },
            DishType::Salad => CookMethod::Raw,
            DishType::StirFry => match role {
                "spice" | "condiment" => CookMethod::Raw,
                _ => CookMethod::Fry,
            },
            DishType::Grill => match role {
                "protein" => CookMethod::Grill,
                "side"    => CookMethod::Grill,
                "base"    => CookMethod::Boil, // rice/pasta always boiled
                "spice" | "condiment" => CookMethod::Raw,
                _ => CookMethod::Grill,
            },
            DishType::Bake => match role {
                "spice" | "condiment" => CookMethod::Raw,
                _ => CookMethod::Bake,
            },
            DishType::Pasta => match role {
                "protein" => CookMethod::Fry,
                "base"    => CookMethod::Boil,
                "side"    => CookMethod::Fry,
                "spice" | "condiment" => CookMethod::Raw,
                _ => CookMethod::Boil,
            },
            DishType::Raw => CookMethod::Raw,
            DishType::Default => CookMethod::for_ingredient(product_type, role, goal),
        }
    }
}

// ── Cooking Steps ────────────────────────────────────────────────────────────

/// A simple cooking step (pure logic, no LLM).
#[derive(Debug, Clone, Serialize)]
pub struct CookingStep {
    pub step: u8,
    pub text: String,
    pub time_min: Option<u16>,
}

// ── Types ────────────────────────────────────────────────────────────────────

/// Minimal schema from Gemini — just dish name + ingredient slugs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DishSchema {
    pub dish: String,
    #[serde(default)]
    pub dish_local: Option<String>,
    pub items: Vec<String>,
}

/// Backend-resolved ingredient with full calculations.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedIngredient {
    #[serde(skip)]
    pub product: Option<IngredientData>,
    pub slug_hint: String,
    pub resolved_slug: Option<String>,
    pub state: String,
    pub role: String,
    pub gross_g: f32,
    pub cleaned_net_g: f32,
    pub cooked_net_g: f32,
    pub kcal: u32,
    pub protein_g: f32,
    pub fat_g: f32,
    pub carbs_g: f32,
}

/// Full resolved recipe / tech-card.
#[derive(Debug, Clone, Serialize)]
pub struct TechCard {
    pub dish_name: String,
    pub dish_name_local: Option<String>,
    /// Improved display name: "Классический борщ с говядиной"
    pub display_name: Option<String>,
    pub dish_type: String,
    pub servings: u8,
    pub ingredients: Vec<ResolvedIngredient>,
    pub steps: Vec<CookingStep>,
    pub total_output_g: f32,
    pub total_kcal: u32,
    pub total_protein: f32,
    pub total_fat: f32,
    pub total_carbs: f32,
    pub per_serving_kcal: u32,
    pub per_serving_protein: f32,
    pub per_serving_fat: f32,
    pub per_serving_carbs: f32,
    pub unresolved: Vec<String>,
}

// ── Gemini call (minimal — 50-100 tokens) ────────────────────────────────────

/// Ask Gemini for ONLY the dish name + ingredient list. Nothing else.
pub async fn ask_gemini_dish_schema(
    llm: &LlmAdapter,
    user_input: &str,
    lang: ChatLang,
) -> Result<DishSchema, String> {
    let lang_label = match lang {
        ChatLang::Ru => "Russian",
        ChatLang::En => "English",
        ChatLang::Pl => "Polish",
        ChatLang::Uk => "Ukrainian",
    };

    let prompt = format!(
        r#"Identify the dish. Return ONLY JSON, no other text.
dish = English name. dish_local = name in {lang}. items = ingredient slugs (English, max 8).
If unknown: {{"dish":"unknown","items":[]}}

User: "{input}"

Example: {{"dish":"borscht","dish_local":"Борщ","items":["beet","cabbage","potato","carrot","onion","beef","garlic","tomato-paste"]}}"#,
        input = user_input,
        lang = lang_label,
    );

    let raw = llm
        // Thinking models use ~80% of max_tokens for chain-of-thought
        .groq_raw_request_with_model(&prompt, 4000, "gemini-3-flash-preview")
        .await
        .map_err(|e| format!("Gemini error: {e}"))?;

    parse_dish_schema(&raw)
}

fn parse_dish_schema(raw: &str) -> Result<DishSchema, String> {
    let json_str = extract_json(raw)
        .ok_or_else(|| format!("No JSON found in: {}", &raw[..raw.len().min(100)]))?;

    let schema: DishSchema = serde_json::from_str(json_str)
        .map_err(|e| format!("JSON parse error: {e} — raw: {}", &raw[..raw.len().min(150)]))?;

    if schema.dish == "unknown" || schema.items.is_empty() {
        return Err("Gemini couldn't recognize this dish".into());
    }

    Ok(schema)
}

/// Extract first {...} from raw text (strips markdown fences etc.)
fn extract_json(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end >= start { Some(&raw[start..=end]) } else { None }
}

// ── Backend Intelligence: resolve, assign roles, portions, cook methods ──────

/// Resolve a minimal dish schema into a full TechCard.
/// ALL intelligence (roles, grams, states, yields) lives here — not in Gemini.
pub async fn resolve_dish(
    cache: &IngredientCache,
    schema: &DishSchema,
    goal: HealthGoal,
) -> TechCard {
    let dish_type = DishType::detect(&schema.dish);
    tracing::info!("🍳 DishType: {:?} for '{}'", dish_type, schema.dish);

    let mut ingredients = Vec::new();
    let mut unresolved = Vec::new();

    for slug_hint in &schema.items {
        match resolve_slug(cache, slug_hint).await {
            Some(product) => {
                let resolved = build_ingredient_for_dish(&product, slug_hint, goal, dish_type);
                ingredients.push(resolved);
            }
            None => {
                unresolved.push(slug_hint.clone());
                ingredients.push(ResolvedIngredient {
                    product: None,
                    slug_hint: slug_hint.clone(),
                    resolved_slug: None,
                    state: "raw".into(),
                    role: "other".into(),
                    gross_g: 0.0, cleaned_net_g: 0.0, cooked_net_g: 0.0,
                    kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
                });
            }
        }
    }

    let total_output: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
    let total_kcal: u32 = ingredients.iter().map(|i| i.kcal).sum();
    let total_protein: f32 = ingredients.iter().map(|i| i.protein_g).sum();
    let total_fat: f32 = ingredients.iter().map(|i| i.fat_g).sum();
    let total_carbs: f32 = ingredients.iter().map(|i| i.carbs_g).sum();

    // Generate cooking steps
    let steps = generate_steps(&ingredients, dish_type, ChatLang::Ru);

    // Build improved display name
    let display_name = build_display_name(schema, &ingredients, dish_type);

    TechCard {
        dish_name: schema.dish.clone(),
        dish_name_local: schema.dish_local.clone(),
        display_name: Some(display_name),
        dish_type: format!("{:?}", dish_type).to_lowercase(),
        servings: 1,
        ingredients,
        steps,
        total_output_g: total_output,
        total_kcal,
        total_protein,
        total_fat,
        total_carbs,
        per_serving_kcal: total_kcal,
        per_serving_protein: round1(total_protein),
        per_serving_fat: round1(total_fat),
        per_serving_carbs: round1(total_carbs),
        unresolved,
    }
}

/// Build a fully-resolved ingredient from a cache product.
/// Backend decides: role, cooking method (based on dish type!), portion, yield, nutrition.
fn build_ingredient(product: &IngredientData, slug_hint: &str, goal: HealthGoal) -> ResolvedIngredient {
    build_ingredient_for_dish(product, slug_hint, goal, DishType::Default)
}

/// Build ingredient with dish-aware cooking method.
fn build_ingredient_for_dish(
    product: &IngredientData,
    slug_hint: &str,
    goal: HealthGoal,
    dish_type: DishType,
) -> ResolvedIngredient {
    let role = override_role(product);
    // 🔴 FIX: Use dish-aware cooking method instead of "in vacuum"
    let mut method = dish_type.cook_method(role, &product.product_type, goal);

    // 🔴 Soup/Stew aromatics: onion & carrot → Saute (зажарка)
    if matches!(dish_type, DishType::Soup | DishType::Stew) && role == "side" {
        let slug = product.slug.as_str();
        if slug.contains("onion") || slug.contains("carrot") {
            method = CookMethod::Saute;
        }
    }

    let state = method_to_state(&method);

    let cooked_portion = recipe_portion(product, role);
    let yield_factor = method.yield_factor(&product.product_type);
    let cleaned_net = cooked_portion / yield_factor;
    let edible = edible_yield_for(&product.product_type, &product.slug);
    let gross = cleaned_net / edible;

    ResolvedIngredient {
        product: Some(product.clone()),
        slug_hint: slug_hint.to_string(),
        resolved_slug: Some(product.slug.clone()),
        state: state.into(),
        role: role.into(),
        gross_g: round1(gross),
        cleaned_net_g: round1(cleaned_net),
        cooked_net_g: round1(cooked_portion),
        kcal: product.kcal_for(gross),
        protein_g: product.protein_for(gross),
        fat_g: product.fat_for(gross),
        carbs_g: product.carbs_for(gross),
    }
}

/// Aromatics, oils, condiments that meal_role() misclassifies as "side".
/// Returns the corrected role.
fn override_role<'a>(product: &IngredientData) -> &'static str {
    // Slug-level overrides for aromatic "vegetables"
    let slug = product.slug.as_str();
    match slug {
        "garlic" | "ginger" | "chili" | "chili-pepper" | "horseradish"
        | "turmeric" | "lemongrass" | "shallot" => return "spice",
        "salt" | "pepper" | "cumin" | "paprika" | "cinnamon" | "nutmeg"
        | "coriander" | "bay-leaf" | "saffron" | "vanilla" => return "spice",
        "olive-oil" | "sunflower-oil" | "butter" | "ghee" | "coconut-oil"
        | "sesame-oil" => return "oil",
        "soy-sauce" | "vinegar" | "mustard" | "ketchup" | "mayo"
        | "tomato-paste" | "fish-sauce" | "worcestershire" => return "condiment",
        _ => {}
    }
    // product_type-level overrides
    match product.product_type.as_str() {
        "oil" | "fat" => "oil",
        "spice" | "herb" | "seasoning" => "spice",
        "condiment" | "sauce" => "condiment",
        _ => product.meal_role(),
    }
}

/// Recipe-specific portion (grams of cooked food on plate).
/// Smaller than standalone meal — this is one ingredient in a dish.
fn recipe_portion(product: &IngredientData, role: &str) -> f32 {
    match role {
        "protein" => match product.product_type.as_str() {
            "meat" | "fish" | "seafood" => 100.0,
            "eggs" => 60.0,
            "dairy" => 50.0,
            _ => 80.0,
        },
        "base" => match product.product_type.as_str() {
            "grain" | "legume" => 60.0,
            _ => 80.0,
        },
        "side" => match product.product_type.as_str() {
            "vegetable" | "mushroom" => 50.0,
            "fruit" => 40.0,
            _ => 50.0,
        },
        "spice" => 5.0,
        "oil" => 15.0,
        "condiment" => 15.0,
        _ => 30.0,
    }
}

fn method_to_state(method: &CookMethod) -> &'static str {
    match method {
        CookMethod::Grill => "grilled",
        CookMethod::Bake => "baked",
        CookMethod::Boil => "boiled",
        CookMethod::Steam => "steamed",
        CookMethod::Fry => "fried",
        CookMethod::Saute => "sauteed",
        CookMethod::Raw => "raw",
    }
}

// ── Cooking Steps Generation (pure logic, no LLM) ───────────────────────────

/// Generate cooking steps based on dish type and resolved ingredients.
fn generate_steps(ingredients: &[ResolvedIngredient], dish_type: DishType, _lang: ChatLang) -> Vec<CookingStep> {
    let protein: Vec<&ResolvedIngredient> = ingredients.iter().filter(|i| i.role == "protein").collect();
    let sides: Vec<&ResolvedIngredient> = ingredients.iter().filter(|i| i.role == "side").collect();
    let bases: Vec<&ResolvedIngredient> = ingredients.iter().filter(|i| i.role == "base").collect();
    let spices: Vec<&ResolvedIngredient> = ingredients.iter().filter(|i| i.role == "spice" || i.role == "condiment").collect();

    let name_of = |ing: &ResolvedIngredient| -> String {
        ing.product.as_ref()
            .map(|p| accusative_case(&p.name_ru))
            .unwrap_or_else(|| ing.slug_hint.clone())
    };

    let names_joined = |ings: &[&ResolvedIngredient]| -> String {
        ings.iter()
            .map(|i| name_of(i).to_lowercase())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let mut steps = Vec::new();
    let mut step_num: u8 = 0;
    let mut add = |text: String, time: Option<u16>| {
        step_num += 1;
        steps.push(CookingStep { step: step_num, text, time_min: time });
    };

    match dish_type {
        DishType::Soup | DishType::Stew => {
            if !protein.is_empty() {
                let verb = if dish_type == DishType::Soup { "Отварить" } else { "Потушить" };
                add(format!("{} {} до готовности", verb, names_joined(&protein)), Some(40));
            }
            // Separate aromatics (onion, carrot → зажарка) from other sides
            let aromatics: Vec<&ResolvedIngredient> = sides.iter()
                .filter(|i| i.state == "sauteed")
                .copied()
                .collect();
            // Зажарка: sauté aromatics first
            if !aromatics.is_empty() {
                let names: String = aromatics.iter().map(|i| name_of(i).to_lowercase()).collect::<Vec<_>>().join(" и ");
                add(format!("Сделать зажарку: спассеровать {} на масле до золотистости", names), Some(7));
            }
            // Root vegetables (potato, beet — not carrot which is in зажарка)
            let roots: Vec<&ResolvedIngredient> = sides.iter()
                .filter(|i| i.state != "sauteed")
                .filter(|i| {
                    let slug = i.resolved_slug.as_deref().unwrap_or("");
                    slug.contains("potato") || slug.contains("beet")
                })
                .copied()
                .collect();
            let leafy: Vec<&ResolvedIngredient> = sides.iter()
                .filter(|i| i.state != "sauteed")
                .filter(|i| {
                    let slug = i.resolved_slug.as_deref().unwrap_or("");
                    !slug.contains("potato") && !slug.contains("beet")
                })
                .copied()
                .collect();
            if !roots.is_empty() {
                let names: String = roots.iter().map(|i| name_of(i).to_lowercase()).collect::<Vec<_>>().join(", ");
                add(format!("Добавить {}, варить", names), Some(15));
            }
            if !leafy.is_empty() {
                let names: String = leafy.iter().map(|i| name_of(i).to_lowercase()).collect::<Vec<_>>().join(", ");
                add(format!("Добавить {}", names), Some(10));
            }
            if !aromatics.is_empty() {
                add("Добавить зажарку в суп".into(), Some(2));
            }
            if !bases.is_empty() {
                add(format!("Добавить {}", names_joined(&bases)), Some(10));
            }
            if !spices.is_empty() {
                add(format!("Добавить {}, довести до вкуса", names_joined(&spices)), Some(5));
            }
            add("Дать настояться 5 минут, подавать".into(), Some(5));
        }
        DishType::Salad => {
            if !sides.is_empty() {
                add(format!("Нарезать {}", names_joined(&sides)), None);
            }
            if !protein.is_empty() {
                add(format!("Нарезать {}", names_joined(&protein)), None);
            }
            add("Смешать все ингредиенты".into(), None);
            if !spices.is_empty() {
                add(format!("Заправить {}", names_joined(&spices)), None);
            }
        }
        DishType::StirFry => {
            add("Разогреть масло в воке на сильном огне".into(), Some(2));
            if !protein.is_empty() {
                add(format!("Обжарить {} до корочки", names_joined(&protein)), Some(5));
            }
            if !sides.is_empty() {
                add(format!("Добавить {}, обжаривать", names_joined(&sides)), Some(5));
            }
            if !spices.is_empty() {
                add(format!("Добавить {}", names_joined(&spices)), Some(2));
            }
            if !bases.is_empty() {
                add(format!("Подать с {}", names_joined(&bases)), None);
            }
        }
        DishType::Grill => {
            if !protein.is_empty() {
                add(format!("Замариновать {}", names_joined(&protein)), Some(30));
            }
            add("Разогреть гриль до высокой температуры".into(), Some(5));
            if !protein.is_empty() {
                add(format!("Обжарить {} на гриле", names_joined(&protein)), Some(10));
            }
            if !sides.is_empty() {
                add(format!("Гриль: {}", names_joined(&sides)), Some(8));
            }
        }
        DishType::Bake => {
            add("Разогреть духовку до 180°C".into(), Some(10));
            let all_names: Vec<String> = protein.iter().chain(sides.iter())
                .map(|i| name_of(i).to_lowercase())
                .collect();
            if !all_names.is_empty() {
                add(format!("Подготовить {}", all_names.join(", ")), None);
            }
            add("Запекать до готовности".into(), Some(30));
        }
        DishType::Pasta => {
            if !bases.is_empty() {
                add(format!("Отварить {} до al dente", names_joined(&bases)), Some(10));
            }
            if !protein.is_empty() {
                add(format!("Обжарить {}", names_joined(&protein)), Some(8));
            }
            if !sides.is_empty() {
                add(format!("Добавить {}", names_joined(&sides)), Some(5));
            }
            add("Соединить с пастой, перемешать".into(), Some(2));
            if !spices.is_empty() {
                add(format!("Добавить {}", names_joined(&spices)), None);
            }
        }
        _ => {
            // Default: simple cook order
            if !protein.is_empty() {
                add(format!("Приготовить {}", names_joined(&protein)), Some(15));
            }
            if !bases.is_empty() {
                add(format!("Приготовить {}", names_joined(&bases)), Some(10));
            }
            if !sides.is_empty() {
                add(format!("Приготовить {}", names_joined(&sides)), Some(10));
            }
            if !spices.is_empty() {
                add(format!("Добавить {}", names_joined(&spices)), None);
            }
        }
    }

    steps
}

// ── Display Name Builder ─────────────────────────────────────────────────────

/// Build an improved display name: "Классический борщ с говядиной"
fn build_display_name(schema: &DishSchema, ingredients: &[ResolvedIngredient], _dish_type: DishType) -> String {
    let dish_local = schema.dish_local.as_deref().unwrap_or(&schema.dish);

    // Find the main protein
    let protein_name = ingredients.iter()
        .find(|i| i.role == "protein")
        .and_then(|i| i.product.as_ref())
        .map(|p| p.name_ru.clone());

    if let Some(protein) = protein_name {
        // "Борщ" + "Говядина" → "Классический борщ с говядиной"
        let with_protein = instrumental_case(&protein);
        format!("{} с {}", dish_local, with_protein)
    } else {
        dish_local.to_string()
    }
}

/// Very simple Russian instrumental case for common proteins.
/// "Говядина" → "говядиной", "Курица" → "курицей"
fn instrumental_case(name: &str) -> String {
    let lower = name.to_lowercase();
    // -а → -ой
    if lower.ends_with('а') {
        return format!("{}ой", &lower[..lower.len() - 'а'.len_utf8()]);
    }
    // -я → -ей
    if lower.ends_with('я') {
        return format!("{}ей", &lower[..lower.len() - 'я'.len_utf8()]);
    }
    // -ь (feminine) → -ью
    if lower.ends_with('ь') {
        return format!("{}ью", &lower[..lower.len() - 'ь'.len_utf8()]);
    }
    // consonant (masculine) → +ом
    format!("{}ом", lower)
}

/// Russian accusative case for recipe steps.
/// "Говядина" → "говядину", "Морковь" → "морковь", "Картофель" → "картофель"
fn accusative_case(name: &str) -> String {
    let lower = name.to_lowercase();
    // -а → -у (feminine: говядина→говядину, свекла→свеклу, капуста→капусту)
    if lower.ends_with('а') {
        return format!("{}у", &lower[..lower.len() - 'а'.len_utf8()]);
    }
    // -я → -ю (feminine: курица uses -а not -я, but: свинья→свинью)
    if lower.ends_with('я') {
        return format!("{}ю", &lower[..lower.len() - 'я'.len_utf8()]);
    }
    // -ь, -й, consonant: accusative = nominative for inanimate (морковь, лук, помидор, чеснок)
    lower
}

// ── Slug Resolution ──────────────────────────────────────────────────────────

async fn resolve_slug(cache: &IngredientCache, hint: &str) -> Option<IngredientData> {
    let h = hint.to_lowercase().replace(' ', "-");

    // 1. Exact
    if let Some(p) = cache.get(&h).await { return Some(p); }

    // 2. Plural/singular
    let singular = h.trim_end_matches('s');
    let plural = format!("{h}s");
    if let Some(p) = cache.get(singular).await { return Some(p); }
    if let Some(p) = cache.get(&plural).await { return Some(p); }

    // 3. Common rewrites
    let rewrites: &[(&str, &str)] = &[
        ("beet", "beetroot"),
        ("chicken", "chicken-breast"),
        ("tomato-paste", "tomato"),
        ("sour-cream", "cream"),
        ("bell-pepper", "pepper"),
        ("green-onion", "onion"),
        ("spring-onion", "onion"),
        ("scallion", "onion"),
        ("cilantro", "coriander"),
        ("cornstarch", "corn"),
        ("stock", "chicken-breast"),
        ("broth", "chicken-breast"),
    ];
    for (from, to) in rewrites {
        if h.contains(from) {
            if let Some(p) = cache.get(to).await { return Some(p); }
        }
    }

    // 4. Substring match
    let all = cache.all().await;
    for p in &all {
        if p.slug.contains(&h) || h.contains(&p.slug) {
            return Some(p.clone());
        }
        if p.name_en.to_lowercase().contains(&h) {
            return Some(p.clone());
        }
    }

    None
}

// ── Yield Tables ─────────────────────────────────────────────────────────────

fn edible_yield_for(product_type: &str, slug: &str) -> f32 {
    let specific = match slug {
        s if s.contains("potato") => Some(0.80),
        s if s.contains("carrot") => Some(0.82),
        s if s.contains("onion") => Some(0.84),
        s if s.contains("garlic") => Some(0.62),
        s if s.contains("shrimp") => Some(0.65),
        s if s.contains("walnut") || s.contains("almond") || s.contains("pistachio") => Some(0.55),
        s if s.contains("banana") => Some(0.64),
        s if s.contains("lemon") || s.contains("orange") => Some(0.65),
        s if s.contains("avocado") => Some(0.67),
        s if s.contains("pumpkin") => Some(0.70),
        s if s.contains("cabbage") => Some(0.80),
        _ => None,
    };
    if let Some(y) = specific { return y; }

    match product_type {
        "fish" => 0.80,
        "meat" => 0.92,
        "seafood" => 0.70,
        "vegetable" => 0.88,
        "fruit" => 0.82,
        "mushroom" => 0.95,
        _ => 1.0,
    }
}

fn round1(v: f32) -> f32 { (v * 10.0).round() / 10.0 }

// ── Text Formatting ──────────────────────────────────────────────────────────

pub fn format_recipe_text(card: &TechCard, lang: ChatLang) -> String {
    // Use improved display name if available
    let dish = card.display_name.as_deref()
        .unwrap_or_else(|| card.dish_name_local.as_deref().unwrap_or(&card.dish_name));
    let mut out = Vec::new();

    match lang {
        ChatLang::Ru => out.push(format!("🍽 **{}** (1 порция)\n", dish)),
        ChatLang::En => out.push(format!("🍽 **{}** (1 serving)\n", dish)),
        ChatLang::Pl => out.push(format!("🍽 **{}** (1 porcja)\n", dish)),
        ChatLang::Uk => out.push(format!("🍽 **{}** (1 порція)\n", dish)),
    }

    match lang {
        ChatLang::Ru => out.push("📋 **Ингредиенты:**".into()),
        ChatLang::En => out.push("📋 **Ingredients:**".into()),
        ChatLang::Pl => out.push("📋 **Składniki:**".into()),
        ChatLang::Uk => out.push("📋 **Інгредієнти:**".into()),
    }

    for ing in &card.ingredients {
        let name = ing.product.as_ref()
            .map(|p| p.name(lang.code()).to_string())
            .unwrap_or_else(|| ing.slug_hint.clone());
        let st = state_label(&ing.state, lang);

        if (ing.gross_g - ing.cooked_net_g).abs() > 2.0 {
            out.push(format!("• {} ({}) — {:.0}г → {:.0}г", name, st, ing.gross_g, ing.cooked_net_g));
        } else {
            out.push(format!("• {} ({}) — {:.0}г", name, st, ing.gross_g));
        }
    }

    // Cooking steps
    if !card.steps.is_empty() {
        out.push(String::new());
        match lang {
            ChatLang::Ru => out.push("👨‍🍳 **Как приготовить:**".into()),
            ChatLang::En => out.push("👨‍🍳 **How to cook:**".into()),
            ChatLang::Pl => out.push("👨‍🍳 **Jak przygotować:**".into()),
            ChatLang::Uk => out.push("👨‍🍳 **Як приготувати:**".into()),
        }
        for step in &card.steps {
            let time_str = step.time_min
                .map(|m| format!(" (~{} мин)", m))
                .unwrap_or_default();
            out.push(format!("{}. {}{}", step.step, step.text, time_str));
        }
    }

    out.push(String::new());
    match lang {
        ChatLang::Ru => {
            out.push(format!("📊 **Выход:** {:.0}г", card.total_output_g));
            out.push(format!("🔥 **КБЖУ:** {} ккал • Б {:.0}г • Ж {:.0}г • У {:.0}г",
                card.per_serving_kcal, card.per_serving_protein,
                card.per_serving_fat, card.per_serving_carbs));
        }
        ChatLang::En => {
            out.push(format!("📊 **Output:** {:.0}g", card.total_output_g));
            out.push(format!("🔥 **CPFC:** {} kcal • P {:.0}g • F {:.0}g • C {:.0}g",
                card.per_serving_kcal, card.per_serving_protein,
                card.per_serving_fat, card.per_serving_carbs));
        }
        ChatLang::Pl => {
            out.push(format!("📊 **Wydajność:** {:.0}g", card.total_output_g));
            out.push(format!("🔥 **KBWT:** {} kcal • B {:.0}g • T {:.0}g • W {:.0}g",
                card.per_serving_kcal, card.per_serving_protein,
                card.per_serving_fat, card.per_serving_carbs));
        }
        ChatLang::Uk => {
            out.push(format!("📊 **Вихід:** {:.0}г", card.total_output_g));
            out.push(format!("🔥 **КБЖВ:** {} ккал • Б {:.0}г • Ж {:.0}г • В {:.0}г",
                card.per_serving_kcal, card.per_serving_protein,
                card.per_serving_fat, card.per_serving_carbs));
        }
    }

    if !card.unresolved.is_empty() {
        out.push(String::new());
        match lang {
            ChatLang::Ru => out.push(format!("⚠️ Не в базе: {}", card.unresolved.join(", "))),
            ChatLang::En => out.push(format!("⚠️ Not in DB: {}", card.unresolved.join(", "))),
            ChatLang::Pl => out.push(format!("⚠️ Brak w bazie: {}", card.unresolved.join(", "))),
            ChatLang::Uk => out.push(format!("⚠️ Нема в базі: {}", card.unresolved.join(", "))),
        }
    }

    out.join("\n")
}

fn state_label<'a>(state: &'a str, lang: ChatLang) -> &'a str {
    match (state, lang) {
        ("raw", ChatLang::Ru) => "сырой", ("raw", ChatLang::En) => "raw",
        ("raw", ChatLang::Pl) => "surowy", ("raw", ChatLang::Uk) => "сирий",
        ("boiled", ChatLang::Ru) => "варёный", ("boiled", ChatLang::En) => "boiled",
        ("boiled", ChatLang::Pl) => "gotowany", ("boiled", ChatLang::Uk) => "варений",
        ("fried", ChatLang::Ru) => "жареный", ("fried", ChatLang::En) => "fried",
        ("fried", ChatLang::Pl) => "smażony", ("fried", ChatLang::Uk) => "смажений",
        ("sauteed", ChatLang::Ru) => "пассерованный", ("sauteed", ChatLang::En) => "sautéed",
        ("sauteed", ChatLang::Pl) => "podsmażony", ("sauteed", ChatLang::Uk) => "спасерований",
        ("baked", ChatLang::Ru) => "запечённый", ("baked", ChatLang::En) => "baked",
        ("baked", ChatLang::Pl) => "pieczony", ("baked", ChatLang::Uk) => "запечений",
        ("grilled", ChatLang::Ru) => "гриль", ("grilled", ChatLang::En) => "grilled",
        ("grilled", ChatLang::Pl) => "grillowany", ("grilled", ChatLang::Uk) => "гриль",
        ("steamed", ChatLang::Ru) => "на пару", ("steamed", ChatLang::En) => "steamed",
        ("steamed", ChatLang::Pl) => "na parze", ("steamed", ChatLang::Uk) => "на парі",
        ("smoked", ChatLang::Ru) => "копчёный", ("smoked", ChatLang::En) => "smoked",
        ("smoked", ChatLang::Pl) => "wędzony", ("smoked", ChatLang::Uk) => "копчений",
        _ => state,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_schema() {
        let json = r#"{"dish":"borscht","dish_local":"Борщ","items":["beet","cabbage","potato","beef"]}"#;
        let s = parse_dish_schema(json).unwrap();
        assert_eq!(s.dish, "borscht");
        assert_eq!(s.items.len(), 4);
        assert_eq!(s.items[0], "beet");
    }

    #[test]
    fn parse_markdown_wrapped() {
        let raw = "```json\n{\"dish\":\"test\",\"items\":[\"a\",\"b\"]}\n```";
        let s = parse_dish_schema(raw).unwrap();
        assert_eq!(s.dish, "test");
        assert_eq!(s.items.len(), 2);
    }

    #[test]
    fn parse_unknown_dish_errors() {
        let json = r#"{"dish":"unknown","items":[]}"#;
        assert!(parse_dish_schema(json).is_err());
    }

    #[test]
    fn edible_yield_potato() {
        let y = edible_yield_for("vegetable", "potato");
        assert!((y - 0.80).abs() < 0.01);
    }

    #[test]
    fn edible_yield_default() {
        let y = edible_yield_for("dairy", "milk");
        assert!((y - 1.0).abs() < 0.01);
    }

    #[test]
    fn build_ingredient_beef() {
        let product = IngredientData {
            slug: "beef".into(),
            name_en: "Beef".into(),
            name_ru: "Говядина".into(),
            name_pl: "Wołowina".into(),
            name_uk: "Яловичина".into(),
            calories_per_100g: 250.0,
            protein_per_100g: 26.0,
            fat_per_100g: 15.0,
            carbs_per_100g: 0.0,
            image_url: None,
            product_type: "meat".into(),
            density_g_per_ml: None,
        };
        let resolved = build_ingredient(&product, "beef", HealthGoal::Balanced);

        assert_eq!(resolved.role, "protein");
        assert!(resolved.state == "grilled" || resolved.state == "baked",
            "meat protein should be grilled/baked, got {}", resolved.state);
        assert!((resolved.cooked_net_g - 100.0).abs() < 1.0);
        assert!(resolved.gross_g > resolved.cooked_net_g);
        assert!(resolved.kcal > 0);
        assert!(resolved.protein_g > 20.0);
    }

    #[test]
    fn build_ingredient_vegetable_is_side() {
        let product = IngredientData {
            slug: "beet".into(),
            name_en: "Beet".into(),
            name_ru: "Свёкла".into(),
            name_pl: "Burak".into(),
            name_uk: "Буряк".into(),
            calories_per_100g: 43.0,
            protein_per_100g: 1.6,
            fat_per_100g: 0.2,
            carbs_per_100g: 9.6,
            image_url: None,
            product_type: "vegetable".into(),
            density_g_per_ml: None,
        };
        let resolved = build_ingredient(&product, "beet", HealthGoal::Balanced);

        assert_eq!(resolved.role, "side");
        assert_eq!(resolved.cooked_net_g, 50.0);
        assert!(resolved.gross_g > 55.0);
    }

    #[test]
    fn recipe_portions_are_reasonable() {
        let meat = IngredientData {
            slug: "chicken-breast".into(), name_en: "Chicken".into(),
            name_ru: "".into(), name_pl: "".into(), name_uk: "".into(),
            calories_per_100g: 165.0, protein_per_100g: 31.0,
            fat_per_100g: 3.6, carbs_per_100g: 0.0, image_url: None,
            product_type: "meat".into(), density_g_per_ml: None,
        };
        assert_eq!(recipe_portion(&meat, "protein"), 100.0);

        let oil = IngredientData {
            slug: "olive-oil".into(), name_en: "Olive Oil".into(),
            name_ru: "".into(), name_pl: "".into(), name_uk: "".into(),
            calories_per_100g: 884.0, protein_per_100g: 0.0,
            fat_per_100g: 100.0, carbs_per_100g: 0.0, image_url: None,
            product_type: "oil".into(), density_g_per_ml: None,
        };
        assert_eq!(recipe_portion(&oil, "oil"), 15.0);
    }

    #[test]
    fn garlic_is_spice_not_side() {
        let garlic = IngredientData {
            slug: "garlic".into(), name_en: "Garlic".into(),
            name_ru: "Чеснок".into(), name_pl: "Czosnek".into(), name_uk: "Часник".into(),
            calories_per_100g: 149.0, protein_per_100g: 6.4,
            fat_per_100g: 0.5, carbs_per_100g: 33.0, image_url: None,
            product_type: "vegetable".into(), density_g_per_ml: None,
        };
        let resolved = build_ingredient(&garlic, "garlic", HealthGoal::Balanced);
        assert_eq!(resolved.role, "spice", "garlic should be spice, not {}", resolved.role);
        assert_eq!(resolved.cooked_net_g, 5.0, "garlic should be 5g, not {}", resolved.cooked_net_g);
    }

    #[test]
    fn extract_json_from_markdown() {
        let raw = "Sure!\n```json\n{\"dish\":\"x\",\"items\":[]}\n```\nDone.";
        let j = extract_json(raw).unwrap();
        assert!(j.starts_with('{') && j.ends_with('}'));
    }
}
