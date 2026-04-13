//! Recipe Engine — hybrid AI + calculation architecture.
//!
//! Flow:
//!   1. Gemini → recognizes dish, returns structured DishSchema (ingredients + states + net grams)
//!   2. Backend → resolves slugs from cache, applies edible_yield, cooking_yield
//!   3. Backend → computes gross/net/output, КБЖУ per ingredient and total
//!   4. Response builder → renders recipe view (user) or tech-card (pro)
//!
//! Key principle: Gemini NEVER computes nutrition or weights.
//! It only provides the dish structure. All math is deterministic backend logic.

use serde::{Deserialize, Serialize};

use crate::infrastructure::IngredientCache;
use crate::infrastructure::ingredient_cache::IngredientData;
use crate::infrastructure::llm_adapter::LlmAdapter;
use super::intent_router::ChatLang;

// ── Types ────────────────────────────────────────────────────────────────────

/// What Gemini returns — dish structure only, no nutrition math.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DishSchema {
    pub dish_name: String,
    pub dish_name_local: Option<String>,
    pub servings: u8,
    pub items: Vec<DishItem>,
}

/// Single ingredient in the dish as recognized by Gemini.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DishItem {
    /// Hint for slug matching (e.g. "potato", "egg", "mayonnaise")
    pub slug_hint: String,
    /// Processing state (e.g. "boiled", "raw", "fried")
    pub state: String,
    /// Target net weight in grams (final weight in dish)
    pub net_target_g: f32,
    /// Role in dish: "base", "protein", "veg", "sauce", "acid", "spice"
    pub role: String,
}

/// Backend-resolved ingredient with gross/net calculations.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedIngredient {
    /// Matched product from cache (None if not found in DB)
    #[serde(skip)]
    pub product: Option<IngredientData>,
    /// Original slug hint from Gemini
    pub slug_hint: String,
    /// Resolved slug (may differ from hint after fuzzy matching)
    pub resolved_slug: Option<String>,
    /// Processing state
    pub state: String,
    /// Role in dish
    pub role: String,
    /// Gross weight (raw, before cleaning) in grams
    pub gross_g: f32,
    /// Net weight (after cleaning, before cooking) in grams
    pub cleaned_net_g: f32,
    /// Final net weight (after cooking) in grams — what goes on the plate
    pub cooked_net_g: f32,
    /// Nutrition for THIS ingredient's portion (based on raw gross weight)
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
    /// Sum of all cooked_net_g
    pub total_output_g: f32,
    /// Total КБЖУ for the whole dish
    pub total_kcal: u32,
    pub total_protein: f32,
    pub total_fat: f32,
    pub total_carbs: f32,
    /// Per-serving КБЖУ
    pub per_serving_kcal: u32,
    pub per_serving_protein: f32,
    pub per_serving_fat: f32,
    pub per_serving_carbs: f32,
    /// Unresolved ingredients (not found in DB)
    pub unresolved: Vec<String>,
}

// ── Gemini call ──────────────────────────────────────────────────────────────

/// Ask Gemini to recognize a dish and return structured schema.
/// Gemini does NOT compute nutrition — only structure.
pub async fn ask_gemini_dish_schema(
    llm: &LlmAdapter,
    user_input: &str,
    lang: ChatLang,
) -> Result<DishSchema, String> {
    let lang_name = match lang {
        ChatLang::Ru => "Russian",
        ChatLang::En => "English",
        ChatLang::Pl => "Polish",
        ChatLang::Uk => "Ukrainian",
    };

    let prompt = format!(
        r#"You are a professional chef. The user asks about a dish.
Identify the dish and return its canonical ingredient list as JSON.

USER INPUT: "{input}"
USER LANGUAGE: {lang_name}

RULES:
- Return ONLY valid JSON, no other text
- Each ingredient should have a simple English slug (for database matching)
- "state" must be one of: raw, boiled, fried, baked, grilled, steamed, smoked, frozen, dried, pickled
- "net_target_g" = approximate final weight per serving in the dish (grams)
- "role" = one of: protein, base, veg, sauce, acid, spice, dairy, garnish
- "dish_name" in English
- "dish_name_local" in {lang_name}
- If you don't recognize the dish, return {{"error": "unknown_dish"}}
- Keep it to the essential ingredients (5-10 max)
- Use realistic home cooking portions for 1 serving

OUTPUT FORMAT:
{{
  "dish_name": "Salad Olivier",
  "dish_name_local": "Салат Оливье",
  "servings": 1,
  "items": [
    {{"slug_hint": "potato", "state": "boiled", "net_target_g": 50, "role": "base"}},
    {{"slug_hint": "carrot", "state": "boiled", "net_target_g": 25, "role": "veg"}},
    {{"slug_hint": "egg", "state": "boiled", "net_target_g": 40, "role": "protein"}},
    {{"slug_hint": "pickled-cucumber", "state": "pickled", "net_target_g": 35, "role": "acid"}},
    {{"slug_hint": "mayonnaise", "state": "raw", "net_target_g": 30, "role": "sauce"}}
  ]
}}"#,
        input = user_input,
        lang_name = lang_name,
    );

    let raw = llm
        .groq_raw_request_with_model(&prompt, 600, "gemini-3-flash-preview")
        .await
        .map_err(|e| format!("Gemini error: {}", e))?;

    parse_dish_schema(&raw)
}

fn parse_dish_schema(raw: &str) -> Result<DishSchema, String> {
    // Extract JSON from potentially markdown-wrapped response
    let json_str = if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            &raw[start..=end]
        } else {
            raw
        }
    } else {
        raw
    };

    // Check for error response
    if json_str.contains("\"error\"") {
        return Err("Gemini couldn't recognize this dish".to_string());
    }

    serde_json::from_str::<DishSchema>(json_str)
        .map_err(|e| format!("Failed to parse dish schema: {} — raw: {}", e, &raw[..raw.len().min(200)]))
}

// ── Ingredient Resolution ────────────────────────────────────────────────────

/// Resolve Gemini's dish schema against our ingredient cache.
/// Computes gross/net/yield for each ingredient.
pub async fn resolve_dish(
    cache: &IngredientCache,
    schema: &DishSchema,
) -> TechCard {
    let mut ingredients = Vec::new();
    let mut unresolved = Vec::new();

    for item in &schema.items {
        let product = resolve_slug(cache, &item.slug_hint).await;

        if let Some(ref p) = product {
            let resolved = compute_ingredient(p, item);
            ingredients.push(resolved);
        } else {
            // Not in our DB — still include with estimates
            unresolved.push(item.slug_hint.clone());
            ingredients.push(ResolvedIngredient {
                product: None,
                slug_hint: item.slug_hint.clone(),
                resolved_slug: None,
                state: item.state.clone(),
                role: item.role.clone(),
                gross_g: item.net_target_g,
                cleaned_net_g: item.net_target_g,
                cooked_net_g: item.net_target_g,
                kcal: 0,
                protein_g: 0.0,
                fat_g: 0.0,
                carbs_g: 0.0,
            });
        }
    }

    // Totals
    let total_output_g: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
    let total_kcal: u32 = ingredients.iter().map(|i| i.kcal).sum();
    let total_protein: f32 = ingredients.iter().map(|i| i.protein_g).sum();
    let total_fat: f32 = ingredients.iter().map(|i| i.fat_g).sum();
    let total_carbs: f32 = ingredients.iter().map(|i| i.carbs_g).sum();

    let servings = schema.servings.max(1);
    let s = servings as f32;

    TechCard {
        dish_name: schema.dish_name.clone(),
        dish_name_local: schema.dish_name_local.clone(),
        servings,
        ingredients,
        total_output_g,
        total_kcal,
        total_protein,
        total_fat,
        total_carbs,
        per_serving_kcal: (total_kcal as f32 / s).round() as u32,
        per_serving_protein: (total_protein / s * 10.0).round() / 10.0,
        per_serving_fat: (total_fat / s * 10.0).round() / 10.0,
        per_serving_carbs: (total_carbs / s * 10.0).round() / 10.0,
        unresolved,
    }
}

/// Fuzzy-match a slug_hint against the ingredient cache.
async fn resolve_slug(cache: &IngredientCache, hint: &str) -> Option<IngredientData> {
    let hint_lower = hint.to_lowercase().replace(' ', "-");

    // 1. Exact slug match
    if let Some(p) = cache.get(&hint_lower).await {
        return Some(p);
    }

    // 2. Try common slug variations
    let variations = vec![
        hint_lower.clone(),
        format!("{}s", hint_lower),            // egg → eggs
        hint_lower.trim_end_matches('s').to_string(), // eggs → egg
        hint_lower.replace("chicken", "chicken-breast"),
        hint_lower.replace("beef", "beef"),
        hint_lower.replace("pork", "pork"),
    ];

    for v in &variations {
        if let Some(p) = cache.get(v).await {
            return Some(p);
        }
    }

    // 3. Substring match against all products
    let all = cache.all().await;
    for p in &all {
        if p.slug.contains(&hint_lower) || hint_lower.contains(&p.slug) {
            return Some(p.clone());
        }
        // Also match against English name
        if p.name_en.to_lowercase().contains(&hint_lower) {
            return Some(p.clone());
        }
    }

    None
}

/// Compute gross/net/nutrition for a single resolved ingredient.
fn compute_ingredient(product: &IngredientData, item: &DishItem) -> ResolvedIngredient {
    let net_target = item.net_target_g;

    // ── Edible yield: gross → cleaned net ──
    // Default edible yield: 100% for most, lower for items with peel
    let edible_yield = edible_yield_for(&product.product_type, &product.slug);

    // ── Cooking yield: cleaned net → cooked net ──
    let cooking_yield = cooking_yield_for(&product.product_type, &item.state);

    // The user wants `net_target_g` on the plate (cooked).
    // Work backwards:
    //   cooked_net = cleaned_net × cooking_yield
    //   cleaned_net = gross × edible_yield
    // So:
    //   cleaned_net = cooked_net / cooking_yield
    //   gross = cleaned_net / edible_yield
    let cleaned_net = net_target / cooking_yield;
    let gross = cleaned_net / edible_yield;

    // Nutrition is calculated on RAW gross weight (what you buy)
    // The nutrition in DB is per 100g raw product
    let kcal = product.kcal_for(gross);
    let protein_g = product.protein_for(gross);
    let fat_g = product.fat_for(gross);
    let carbs_g = product.carbs_for(gross);

    ResolvedIngredient {
        product: Some(product.clone()),
        slug_hint: item.slug_hint.clone(),
        resolved_slug: Some(product.slug.clone()),
        state: item.state.clone(),
        role: item.role.clone(),
        gross_g: (gross * 10.0).round() / 10.0,
        cleaned_net_g: (cleaned_net * 10.0).round() / 10.0,
        cooked_net_g: (net_target * 10.0).round() / 10.0,
        kcal,
        protein_g,
        fat_g,
        carbs_g,
    }
}

/// Edible yield percent (0.0–1.0): what remains after peeling/cleaning.
/// Based on standard culinary ratios.
fn edible_yield_for(product_type: &str, slug: &str) -> f32 {
    // Specific products with significant waste
    let specific = match slug {
        s if s.contains("potato") => Some(0.80),   // 20% peel
        s if s.contains("carrot") => Some(0.82),   // 18% peel
        s if s.contains("onion") => Some(0.84),    // 16% skin
        s if s.contains("garlic") => Some(0.62),   // 38% skin
        s if s.contains("shrimp") => Some(0.65),   // 35% shell
        s if s.contains("walnut") || s.contains("almond") || s.contains("pistachio") => Some(0.55),
        s if s.contains("banana") => Some(0.64),   // 36% peel
        s if s.contains("lemon") || s.contains("orange") => Some(0.65),
        s if s.contains("avocado") => Some(0.67),
        s if s.contains("pumpkin") => Some(0.70),
        s if s.contains("cabbage") => Some(0.80),
        _ => None,
    };

    if let Some(y) = specific {
        return y;
    }

    // By product type
    match product_type {
        "fish" => 0.80,        // head, bones, skin
        "meat" => 0.92,        // minimal trim
        "seafood" => 0.70,     // shell, head
        "vegetable" => 0.88,   // average
        "fruit" => 0.82,       // peel, seeds
        "mushroom" => 0.95,    // minimal waste
        _ => 1.0,              // dairy, grain, oil, etc.
    }
}

/// Cooking yield (0.0–2.5): weight change factor during cooking.
/// < 1.0 = loses weight, > 1.0 = absorbs water.
fn cooking_yield_for(product_type: &str, state: &str) -> f32 {
    match state {
        "raw" | "frozen" | "pickled" => 1.0,
        "boiled" => match product_type {
            "grain" | "legume" => 2.2,       // rice/pasta absorb water
            "meat" => 0.75,
            "fish" | "seafood" => 0.82,
            "vegetable" => 0.90,
            "eggs" => 0.97,                  // minimal loss
            _ => 0.90,
        },
        "fried" => match product_type {
            "meat" => 0.68,
            "fish" | "seafood" => 0.72,
            "vegetable" | "mushroom" => 0.78,
            "eggs" => 0.92,
            _ => 0.80,
        },
        "baked" => match product_type {
            "meat" => 0.72,
            "fish" => 0.80,
            _ => 0.82,
        },
        "grilled" => match product_type {
            "meat" => 0.70,
            "fish" => 0.75,
            "vegetable" => 0.82,
            _ => 0.78,
        },
        "steamed" => match product_type {
            "fish" | "seafood" => 0.88,
            "vegetable" => 0.92,
            _ => 0.88,
        },
        "smoked" => 0.70,
        "dried" => 0.25,
        _ => 1.0,
    }
}

// ── Text formatting ──────────────────────────────────────────────────────────

/// Format tech-card as user-friendly text.
pub fn format_recipe_text(card: &TechCard, lang: ChatLang) -> String {
    let dish = card.dish_name_local.as_deref()
        .unwrap_or(&card.dish_name);

    let mut lines = Vec::new();

    // Header
    let servings_label = match lang {
        ChatLang::Ru => format!("🍽 **{}** ({} порц.)\n", dish, card.servings),
        ChatLang::En => format!("🍽 **{}** ({} serving{})\n", dish, card.servings, if card.servings > 1 { "s" } else { "" }),
        ChatLang::Pl => format!("🍽 **{}** ({} porcj{})\n", dish, card.servings, if card.servings > 1 { "e" } else { "a" }),
        ChatLang::Uk => format!("🍽 **{}** ({} порц.)\n", dish, card.servings),
    };
    lines.push(servings_label);

    // Ingredients table
    let header_label = match lang {
        ChatLang::Ru => "📋 **Ингредиенты:**",
        ChatLang::En => "📋 **Ingredients:**",
        ChatLang::Pl => "📋 **Składniki:**",
        ChatLang::Uk => "📋 **Інгредієнти:**",
    };
    lines.push(header_label.to_string());

    for ing in &card.ingredients {
        let name = ing.product.as_ref()
            .map(|p| p.name(lang.code()).to_string())
            .unwrap_or_else(|| ing.slug_hint.clone());

        let state_label = state_label_short(&ing.state, lang);

        // Show: Name (state) — gross_g → net_g
        if (ing.gross_g - ing.cooked_net_g).abs() > 2.0 {
            match lang {
                ChatLang::Ru => lines.push(format!(
                    "• {} ({}) — {:.0}г брутто → {:.0}г нетто",
                    name, state_label, ing.gross_g, ing.cooked_net_g
                )),
                ChatLang::En => lines.push(format!(
                    "• {} ({}) — {:.0}g gross → {:.0}g net",
                    name, state_label, ing.gross_g, ing.cooked_net_g
                )),
                ChatLang::Pl => lines.push(format!(
                    "• {} ({}) — {:.0}g brutto → {:.0}g netto",
                    name, state_label, ing.gross_g, ing.cooked_net_g
                )),
                ChatLang::Uk => lines.push(format!(
                    "• {} ({}) — {:.0}г брутто → {:.0}г нетто",
                    name, state_label, ing.gross_g, ing.cooked_net_g
                )),
            }
        } else {
            lines.push(format!("• {} ({}) — {:.0}г", name, state_label, ing.gross_g));
        }
    }

    // Output
    lines.push(String::new());
    match lang {
        ChatLang::Ru => {
            lines.push(format!("📊 **Выход:** {:.0}г", card.total_output_g));
            lines.push(format!(
                "🔥 **На порцию:** {} ккал • Б {:.0}г • Ж {:.0}г • У {:.0}г",
                card.per_serving_kcal, card.per_serving_protein,
                card.per_serving_fat, card.per_serving_carbs
            ));
        }
        ChatLang::En => {
            lines.push(format!("📊 **Output:** {:.0}g", card.total_output_g));
            lines.push(format!(
                "🔥 **Per serving:** {} kcal • P {:.0}g • F {:.0}g • C {:.0}g",
                card.per_serving_kcal, card.per_serving_protein,
                card.per_serving_fat, card.per_serving_carbs
            ));
        }
        ChatLang::Pl => {
            lines.push(format!("📊 **Wydajność:** {:.0}g", card.total_output_g));
            lines.push(format!(
                "🔥 **Na porcję:** {} kcal • B {:.0}g • T {:.0}g • W {:.0}g",
                card.per_serving_kcal, card.per_serving_protein,
                card.per_serving_fat, card.per_serving_carbs
            ));
        }
        ChatLang::Uk => {
            lines.push(format!("📊 **Вихід:** {:.0}г", card.total_output_g));
            lines.push(format!(
                "🔥 **На порцію:** {} ккал • Б {:.0}г • Ж {:.0}г • В {:.0}г",
                card.per_serving_kcal, card.per_serving_protein,
                card.per_serving_fat, card.per_serving_carbs
            ));
        }
    }

    // Unresolved warning
    if !card.unresolved.is_empty() {
        lines.push(String::new());
        let warn = match lang {
            ChatLang::Ru => format!("⚠️ Не найдено в базе: {}", card.unresolved.join(", ")),
            ChatLang::En => format!("⚠️ Not in database: {}", card.unresolved.join(", ")),
            ChatLang::Pl => format!("⚠️ Brak w bazie: {}", card.unresolved.join(", ")),
            ChatLang::Uk => format!("⚠️ Не знайдено в базі: {}", card.unresolved.join(", ")),
        };
        lines.push(warn);
    }

    lines.join("\n")
}

fn state_label_short<'a>(state: &'a str, lang: ChatLang) -> &'a str {
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
        ("frozen", ChatLang::Ru) => "заморож.", ("frozen", ChatLang::En) => "frozen",
        ("frozen", ChatLang::Pl) => "mrożony", ("frozen", ChatLang::Uk) => "заморож.",
        ("dried", ChatLang::Ru) => "сушёный", ("dried", ChatLang::En) => "dried",
        ("dried", ChatLang::Pl) => "suszony", ("dried", ChatLang::Uk) => "сушений",
        ("pickled", ChatLang::Ru) => "маринов.", ("pickled", ChatLang::En) => "pickled",
        ("pickled", ChatLang::Pl) => "marynowany", ("pickled", ChatLang::Uk) => "маринов.",
        _ => state,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_dish_schema() {
        let json = r#"{
            "dish_name": "Salad Olivier",
            "dish_name_local": "Салат Оливье",
            "servings": 1,
            "items": [
                {"slug_hint": "potato", "state": "boiled", "net_target_g": 50, "role": "base"},
                {"slug_hint": "egg", "state": "boiled", "net_target_g": 40, "role": "protein"}
            ]
        }"#;
        let schema = parse_dish_schema(json).unwrap();
        assert_eq!(schema.dish_name, "Salad Olivier");
        assert_eq!(schema.items.len(), 2);
        assert_eq!(schema.items[0].slug_hint, "potato");
        assert_eq!(schema.items[0].state, "boiled");
        assert_eq!(schema.items[0].net_target_g, 50.0);
    }

    #[test]
    fn parse_markdown_wrapped_json() {
        let raw = "```json\n{\"dish_name\": \"Test\", \"servings\": 1, \"items\": []}\n```";
        let schema = parse_dish_schema(raw).unwrap();
        assert_eq!(schema.dish_name, "Test");
    }

    #[test]
    fn edible_yield_potato() {
        let y = edible_yield_for("vegetable", "potato");
        assert!((y - 0.80).abs() < 0.01, "potato yield should be ~80%, got {}", y);
    }

    #[test]
    fn cooking_yield_boiled_grain() {
        let y = cooking_yield_for("grain", "boiled");
        assert!(y > 2.0, "boiled grain should absorb water, yield > 2.0, got {}", y);
    }

    #[test]
    fn cooking_yield_raw_is_1() {
        let y = cooking_yield_for("meat", "raw");
        assert!((y - 1.0).abs() < 0.01);
    }

    #[test]
    fn compute_ingredient_potato_boiled() {
        let product = IngredientData {
            slug: "potatoes".into(),
            name_en: "Potatoes".into(),
            name_ru: "Картофель".into(),
            name_pl: "Ziemniaki".into(),
            name_uk: "Картопля".into(),
            calories_per_100g: 77.0,
            protein_per_100g: 2.0,
            fat_per_100g: 0.1,
            carbs_per_100g: 17.5,
            image_url: None,
            product_type: "vegetable".into(),
            density_g_per_ml: None,
        };

        let item = DishItem {
            slug_hint: "potato".into(),
            state: "boiled".into(),
            net_target_g: 50.0,
            role: "base".into(),
        };

        let resolved = compute_ingredient(&product, &item);

        // 50g cooked net target
        // cooking yield for boiled vegetable = 0.90
        // cleaned_net = 50 / 0.90 ≈ 55.6g
        // edible yield for potato = 0.80
        // gross = 55.6 / 0.80 ≈ 69.4g
        assert!(resolved.gross_g > 60.0 && resolved.gross_g < 80.0,
            "potato gross should be ~69g, got {:.1}", resolved.gross_g);
        assert!(resolved.cooked_net_g == 50.0);
        assert!(resolved.kcal > 0, "should have calories");
    }
}
