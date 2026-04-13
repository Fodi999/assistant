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
    pub servings: u8,
    pub ingredients: Vec<ResolvedIngredient>,
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
        .groq_raw_request_with_model(&prompt, 400, "gemini-3-flash-preview")
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
    let mut ingredients = Vec::new();
    let mut unresolved = Vec::new();

    for slug_hint in &schema.items {
        match resolve_slug(cache, slug_hint).await {
            Some(product) => {
                let resolved = build_ingredient(&product, slug_hint, goal);
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

    TechCard {
        dish_name: schema.dish.clone(),
        dish_name_local: schema.dish_local.clone(),
        servings: 1,
        ingredients,
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
/// Backend decides: role, cooking method, portion, yield, nutrition.
fn build_ingredient(product: &IngredientData, slug_hint: &str, goal: HealthGoal) -> ResolvedIngredient {
    // Override role for aromatics that meal_role() classifies as "side"
    let role = override_role(product);
    let method = CookMethod::for_ingredient(&product.product_type, role, goal);
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
        CookMethod::Raw => "raw",
    }
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
    let dish = card.dish_name_local.as_deref().unwrap_or(&card.dish_name);
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
