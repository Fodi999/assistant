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
use super::cooking_rules::{self, IngredientRole, DishRule, StepType};
use super::food_pairing;

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
    /// Delegates to cooking_rules for DDD rule lookup.
    pub fn cook_method(&self, role: &str, slug: &str, _product_type: &str, _goal: HealthGoal) -> CookMethod {
        let rule = cooking_rules::load_rule(*self);
        let ingredient_role = IngredientRole::from_str_role(role, slug);
        cooking_rules::method_for_role(&rule, ingredient_role)
    }
}

// ── Cooking Steps ────────────────────────────────────────────────────────────

/// A simple cooking step (pure logic, no LLM).
#[derive(Debug, Clone, Serialize)]
pub struct CookingStep {
    pub step: u8,
    pub text: String,
    pub time_min: Option<u16>,
    /// Cooking temperature in °C if relevant (sear=200, bake=180, etc.)
    pub temp_c: Option<u16>,
    /// Short chef tip for this step (localized)
    pub tip: Option<String>,
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
    pub total_gross_g: f32,
    pub total_kcal: u32,
    pub total_protein: f32,
    pub total_fat: f32,
    pub total_carbs: f32,
    pub per_serving_kcal: u32,
    pub per_serving_protein: f32,
    pub per_serving_fat: f32,
    pub per_serving_carbs: f32,
    pub unresolved: Vec<String>,
    // ── Dish context (v2) ──
    /// "easy" | "medium" | "hard"
    pub complexity: String,
    /// "balanced" | "high_protein" | "low_calorie"
    pub goal: String,
    /// Allergen/intolerance flags present in the dish, e.g. ["gluten", "lactose", "nuts"]
    pub allergens: Vec<String>,
    /// Diet tags, e.g. ["vegetarian", "vegan", "pescatarian"]
    pub tags: Vec<String>,
}

// ── Gemini call (minimal — 50-100 tokens) ────────────────────────────────────

/// Ask Gemini for ONLY the dish name + ingredient list. Nothing else.
pub async fn ask_gemini_dish_schema(
    llm: &LlmAdapter,
    user_input: &str,
    lang: ChatLang,
    goal: HealthGoal,
) -> Result<DishSchema, String> {
    let lang_label = match lang {
        ChatLang::Ru => "Russian",
        ChatLang::En => "English",
        ChatLang::Pl => "Polish",
        ChatLang::Uk => "Ukrainian",
    };

    let goal_hint = match goal {
        HealthGoal::LowCalorie  => "\nThis is a LOW-CALORIE / DIET version. Pick lean ingredients: vegetables, lean fish/poultry, skip heavy sauces and fatty items. No cherry, no cream, no sugar.",
        HealthGoal::HighProtein => "\nThis is a HIGH-PROTEIN version. Prefer protein-rich ingredients: chicken breast, beef, eggs, legumes.",
        HealthGoal::Balanced    => "",
    };

    let prompt = format!(
        r#"Identify the dish. Return ONLY JSON, no other text.
dish = English name. dish_local = name in {lang}. items = ingredient slugs (English, max 8).
Use only realistic, classic ingredients for this dish. No exotic or random items.{goal_hint}
If unknown: {{"dish":"unknown","items":[]}}

User: "{input}"

Example: {{"dish":"borscht","dish_local":"Борщ","items":["beet","cabbage","potato","carrot","onion","beef","garlic","tomato-paste"]}}"#,
        input = user_input,
        lang = lang_label,
        goal_hint = goal_hint,
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
    lang: ChatLang,
) -> TechCard {
    let dish_type = DishType::detect(&schema.dish);
    let rule = cooking_rules::load_rule(dish_type);
    tracing::info!("🍳 DishType: {:?} for '{}'", dish_type, schema.dish);

    // ── 1. Food Pairing Filter: remove absurd combinations ──────────────
    let (filtered_items, removed) = food_pairing::filter_ingredients(&schema.items, dish_type);
    if !removed.is_empty() {
        tracing::warn!("🚫 Removed ingredients: {:?}", removed);
    }

    let mut ingredients = Vec::new();
    let mut unresolved = Vec::new();

    for slug_hint in &filtered_items {
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

    // ── 2. Auto-insert implicit ingredients (Liquid for soup, Oil for sauté) ──
    auto_insert_implicit(&mut ingredients, dish_type, cache, goal).await;

    // ── 3. Constraint Engine: enforce culinary laws ─────────────────────
    let servings_estimate = {
        let total: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
        let target = match dish_type {
            DishType::Soup | DishType::Stew => 350.0_f32,
            DishType::Salad | DishType::Raw    => 250.0,
            _                                   => 300.0,
        };
        ((total / target).round() as u8).max(1)
    };

    let mut snapshots: Vec<cooking_rules::IngredientSnapshot> = ingredients.iter().map(|i| {
        let slug = i.resolved_slug.as_deref().unwrap_or(&i.slug_hint);
        cooking_rules::IngredientSnapshot {
            slug: slug.to_string(),
            role: IngredientRole::from_str_role(&i.role, slug),
            gross_g: i.gross_g,
            fat_g: i.fat_g,
            protein_g: i.protein_g,
            kcal: i.kcal,
        }
    }).collect();

    let violations = cooking_rules::apply_constraints(&rule, &mut snapshots, servings_estimate);

    // Apply constraint fixes back to actual ingredients
    if !violations.is_empty() {
        for v in &violations {
            if v.auto_fixed {
                tracing::info!("🔧 Constraint auto-fix: {}", v.message);
            } else {
                tracing::warn!("⚠️ Constraint warning: {}", v.message);
            }
        }

        // Sync oil/fat changes back from snapshots
        for (ing, snap) in ingredients.iter_mut().zip(snapshots.iter()) {
            let slug = ing.resolved_slug.as_deref().unwrap_or(&ing.slug_hint);
            let role = IngredientRole::from_str_role(&ing.role, slug);
            if role == IngredientRole::Oil && (ing.gross_g - snap.gross_g).abs() > 0.1 {
                ing.gross_g = snap.gross_g;
                ing.cleaned_net_g = snap.gross_g;
                ing.cooked_net_g = snap.gross_g;
                ing.fat_g = snap.fat_g;
                ing.kcal = snap.kcal;
            }
        }

        // If constraint engine added water (RequiresLiquid auto-fix),
        // check if it's already in ingredients (auto_insert_implicit may have added it)
        let has_liquid = ingredients.iter().any(|i| i.role == "liquid");
        let snap_has_water = snapshots.iter().any(|s| s.slug == "water" && s.role == IngredientRole::Liquid);
        if snap_has_water && !has_liquid {
            let water_product = resolve_slug(cache, "water").await;
            ingredients.push(ResolvedIngredient {
                product: water_product,
                slug_hint: "water".into(),
                resolved_slug: Some("water".into()),
                state: "boiled".into(),
                role: "liquid".into(),
                gross_g: 300.0, cleaned_net_g: 300.0, cooked_net_g: 300.0,
                kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
            });
        }
    }

    // ── 4. Compute totals ───────────────────────────────────────────────
    let total_gross: f32 = ingredients.iter().map(|i| i.gross_g).sum();
    let total_output: f32 = ingredients.iter().map(|i| i.cooked_net_g).sum();
    let total_kcal: u32 = ingredients.iter().map(|i| i.kcal).sum();
    let total_protein: f32 = ingredients.iter().map(|i| i.protein_g).sum();
    let total_fat: f32 = ingredients.iter().map(|i| i.fat_g).sum();
    let total_carbs: f32 = ingredients.iter().map(|i| i.carbs_g).sum();

    // ── 5. Generate cooking steps ───────────────────────────────────────
    let steps = generate_steps(&ingredients, dish_type, lang);

    // Build improved display name (with goal prefix)
    let display_name = build_display_name(schema, &ingredients, dish_type, goal, lang);

    // ── 5b. Dish context: complexity, goal label, allergens, tags ────────
    let complexity = compute_complexity(&steps);
    let goal_label = match goal {
        HealthGoal::HighProtein => "high_protein",
        HealthGoal::LowCalorie  => "low_calorie",
        HealthGoal::Balanced    => "balanced",
    }.to_string();
    let allergens = detect_allergens(&ingredients);
    let tags = detect_diet_tags(&ingredients);

    // ── 6. Auto-portion: split into realistic servings (~300–400g each) ──
    let portion_target = match dish_type {
        DishType::Soup | DishType::Stew => 350.0_f32,
        DishType::Salad | DishType::Raw    => 250.0,
        _                                   => 300.0,
    };
    let servings = ((total_output / portion_target).round() as u8).max(1);
    let per_kcal = (total_kcal as f32 / servings as f32).round() as u32;
    let per_prot = round1(total_protein / servings as f32);
    let per_fat  = round1(total_fat / servings as f32);
    let per_carb = round1(total_carbs / servings as f32);

    TechCard {
        dish_name: schema.dish.clone(),
        dish_name_local: schema.dish_local.clone(),
        display_name: Some(display_name),
        dish_type: format!("{:?}", dish_type).to_lowercase(),
        servings,
        ingredients,
        steps,
        total_output_g: total_output,
        total_gross_g: total_gross,
        total_kcal,
        total_protein,
        total_fat,
        total_carbs,
        per_serving_kcal: per_kcal,
        per_serving_protein: per_prot,
        per_serving_fat: per_fat,
        per_serving_carbs: per_carb,
        unresolved,
        complexity,
        goal: goal_label,
        allergens,
        tags,
    }
}

// ── Dish context helpers ─────────────────────────────────────────────────────

/// Complexity: easy (<= 4 steps && <= 20 min), hard (>= 8 steps || >= 60 min), else medium.
fn compute_complexity(steps: &[CookingStep]) -> String {
    let total_min: u16 = steps.iter().filter_map(|s| s.time_min).sum();
    let n = steps.len();
    if n <= 4 && total_min <= 20 {
        "easy".into()
    } else if n >= 8 || total_min >= 60 {
        "hard".into()
    } else {
        "medium".into()
    }
}

/// Detect allergens/intolerances from ingredient product_types and slugs.
fn detect_allergens(ingredients: &[ResolvedIngredient]) -> Vec<String> {
    let mut flags = Vec::new();

    let has = |f: &dyn Fn(&ResolvedIngredient) -> bool| ingredients.iter().any(f);

    // Gluten: wheat, flour, pasta, bread, barley, rye, oats
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        s.contains("wheat") || s.contains("flour") || s.contains("pasta")
            || s.contains("bread") || s.contains("barley") || s.contains("rye")
            || s.contains("oat") || s.contains("spaghetti") || s.contains("noodle")
            || s.contains("couscous") || s.contains("semolina")
            || (pt == "grain" && !s.contains("rice") && !s.contains("corn") && !s.contains("buckwheat") && !s.contains("quinoa"))
    }) {
        flags.push("gluten".into());
    }

    // Lactose: dairy products
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        pt == "dairy" || s.contains("milk") || s.contains("cream") || s.contains("cheese")
            || s.contains("butter") || s.contains("yogurt") || s.contains("kefir")
            || s.contains("sour-cream") || s.contains("smetana")
    }) {
        flags.push("lactose".into());
    }

    // Nuts
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        pt == "nut" || s.contains("almond") || s.contains("walnut") || s.contains("cashew")
            || s.contains("peanut") || s.contains("hazelnut") || s.contains("pecan")
            || s.contains("pistachio") || s.contains("macadamia") || s.contains("pine-nut")
    }) {
        flags.push("nuts".into());
    }

    // Eggs
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        s.contains("egg")
    }) {
        flags.push("eggs".into());
    }

    // Fish
    if has(&|i| {
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        pt == "fish"
    }) {
        flags.push("fish".into());
    }

    // Shellfish / Seafood
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        let pt = i.product.as_ref().map(|p| p.product_type.as_str()).unwrap_or("");
        pt == "seafood" || s.contains("shrimp") || s.contains("prawn") || s.contains("crab")
            || s.contains("lobster") || s.contains("mussel") || s.contains("oyster")
            || s.contains("squid") || s.contains("octopus") || s.contains("clam")
    }) {
        flags.push("shellfish".into());
    }

    // Soy
    if has(&|i| {
        let s = i.slug_hint.to_lowercase();
        s.contains("soy") || s.contains("tofu") || s.contains("edamame") || s.contains("tempeh")
    }) {
        flags.push("soy".into());
    }

    flags
}

/// Detect diet tags: vegan, vegetarian, pescatarian, etc.
fn detect_diet_tags(ingredients: &[ResolvedIngredient]) -> Vec<String> {
    let mut tags = Vec::new();

    let types: Vec<&str> = ingredients
        .iter()
        .filter_map(|i| i.product.as_ref().map(|p| p.product_type.as_str()))
        .collect();

    let slugs: Vec<String> = ingredients.iter().map(|i| i.slug_hint.to_lowercase()).collect();

    let has_meat = types.iter().any(|t| *t == "meat");
    let has_fish = types.iter().any(|t| *t == "fish");
    let has_seafood = types.iter().any(|t| *t == "seafood");
    let has_dairy = types.iter().any(|t| *t == "dairy");
    let has_eggs = slugs.iter().any(|s| s.contains("egg"));

    if !has_meat && !has_fish && !has_seafood && !has_dairy && !has_eggs {
        tags.push("vegan".into());
        tags.push("vegetarian".into());
    } else if !has_meat && !has_fish && !has_seafood {
        tags.push("vegetarian".into());
    } else if !has_meat && (has_fish || has_seafood) {
        tags.push("pescatarian".into());
    }

    // High-protein: > 25g protein per serving
    // (this is checked later in response, but we have the data)

    // Low-carb: < 20g carbs per serving
    // We don't have per-serving here, computed after, so skip

    tags
}

/// Build a fully-resolved ingredient from a cache product.
/// Backend decides: role, cooking method (based on dish type!), portion, yield, nutrition.
fn build_ingredient(product: &IngredientData, slug_hint: &str, goal: HealthGoal) -> ResolvedIngredient {
    build_ingredient_for_dish(product, slug_hint, goal, DishType::Default)
}

/// Build ingredient with dish-aware cooking method.
/// Uses DDD cooking_rules to determine method by (dish_type, role, slug).
fn build_ingredient_for_dish(
    product: &IngredientData,
    slug_hint: &str,
    goal: HealthGoal,
    dish_type: DishType,
) -> ResolvedIngredient {
    let role = override_role(product);
    // DDD: cooking method resolved via rules (aromatics → Saute in soup, etc.)
    let method = dish_type.cook_method(role, &product.slug, &product.product_type, goal);

    let state = method_to_state(&method);

    let cooked_portion = recipe_portion_goal(product, role, goal);
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

/// Goal-aware portion: LowCalorie = less oil/meat, more veg.
/// HighProtein = more protein, same oil. Balanced = default.
fn recipe_portion_goal(product: &IngredientData, role: &str, goal: HealthGoal) -> f32 {
    let base = recipe_portion(product, role);
    match goal {
        HealthGoal::LowCalorie => match role {
            "oil"     => 5.0,        // 15g → 5g (минимум масла)
            "protein" => base * 0.8, // 100g → 80g
            "side"    => base * 1.2, // больше овощей
            "condiment" => base * 0.5, // меньше соуса
            _ => base,
        },
        HealthGoal::HighProtein => match role {
            "protein" => base * 1.3, // 100g → 130g
            "side"    => base * 0.8, // чуть меньше овощей
            _ => base,
        },
        HealthGoal::Balanced => base,
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

// ── Auto-insert Implicit Ingredients ─────────────────────────────────────────

/// Automatically add implicit ingredients that a recipe logically needs but
/// Gemini doesn't include (water for soup, oil for sauté).
async fn auto_insert_implicit(
    ingredients: &mut Vec<ResolvedIngredient>,
    dish_type: DishType,
    cache: &IngredientCache,
    goal: HealthGoal,
) {
    // Pre-compute flags before mutating the vec (borrow checker)
    let has_liquid = ingredients.iter().any(|i| i.role == "liquid");
    let has_oil = ingredients.iter().any(|i| i.role == "oil");
    let has_oil_slug = ingredients.iter().any(|i| {
        i.resolved_slug.as_deref() == Some("sunflower-oil")
            || i.resolved_slug.as_deref() == Some("olive-oil")
            || i.slug_hint == "sunflower-oil"
            || i.slug_hint == "olive-oil"
    });
    let has_saute = ingredients.iter().any(|i| i.state == "sauteed");

    // ── Soup/Stew: add water (300ml per serving) if no liquid ───────────
    if matches!(dish_type, DishType::Soup | DishType::Stew) && !has_liquid {
        // Try to resolve from cache for localized name
        let water_product = resolve_slug(cache, "water").await;
        ingredients.push(ResolvedIngredient {
            product: water_product,
            slug_hint: "water".into(),
            resolved_slug: Some("water".into()),
            state: "boiled".into(),
            role: "liquid".into(),
            gross_g: 300.0,
            cleaned_net_g: 300.0,
            cooked_net_g: 300.0,
            kcal: 0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
        });
    }

    // ── Sauté dishes: add cooking oil if none present ───────────────────
    if has_saute && !has_oil && !has_oil_slug {
        let portion = match goal {
            HealthGoal::LowCalorie => 5.0_f32,  // minimal oil for diet
            _ => 15.0_f32,                       // 15g = ~1 tbsp
        };
        // Try to resolve from cache for accurate nutrition
        if let Some(oil) = resolve_slug(cache, "sunflower-oil").await {
            ingredients.push(ResolvedIngredient {
                product: Some(oil.clone()),
                slug_hint: "sunflower-oil".into(),
                resolved_slug: Some(oil.slug.clone()),
                state: "raw".into(),
                role: "oil".into(),
                gross_g: portion,
                cleaned_net_g: portion,
                cooked_net_g: portion,
                kcal: oil.kcal_for(portion),
                protein_g: oil.protein_for(portion),
                fat_g: oil.fat_for(portion),
                carbs_g: oil.carbs_for(portion),
            });
        } else {
            // Fallback: generic oil entry with estimated nutrition
            let fallback_kcal = (portion * 9.0) as u32; // ~9 kcal/g for oil
            ingredients.push(ResolvedIngredient {
                product: None,
                slug_hint: "sunflower-oil".into(),
                resolved_slug: Some("sunflower-oil".into()),
                state: "raw".into(),
                role: "oil".into(),
                gross_g: portion,
                cleaned_net_g: portion,
                cooked_net_g: portion,
                kcal: fallback_kcal, protein_g: 0.0, fat_g: portion, carbs_g: 0.0,
            });
        }
    }
}

// ── Cooking Steps Generation (pure logic, no LLM) ───────────────────────────

/// Generate cooking steps driven by DishRule (DDD: rules as data).
/// Iterates the rule's step sequence; for each step, collects matching ingredients,
/// skips the step if no ingredients match, otherwise generates text.
fn generate_steps(ingredients: &[ResolvedIngredient], dish_type: DishType, lang: ChatLang) -> Vec<CookingStep> {
    let rule = cooking_rules::load_rule(dish_type);
    let lang_code = lang.code();

    // ── Classify ingredients by DDD role ─────────────────────────────────
    let classify = |ing: &ResolvedIngredient| -> IngredientRole {
        let slug = ing.resolved_slug.as_deref().unwrap_or(&ing.slug_hint);
        IngredientRole::from_str_role(&ing.role, slug)
    };

    let by_role = |target: IngredientRole| -> Vec<&ResolvedIngredient> {
        ingredients.iter().filter(|i| classify(i) == target).collect()
    };

    // Helper: pick ingredient name by language + apply grammar
    // `case` controls Russian declension: accusative for most steps, instrumental for Dress
    let name_of_case = |ing: &ResolvedIngredient, case: &str| -> String {
        ing.product.as_ref()
            .map(|p| {
                let raw = match lang {
                    ChatLang::En => &p.name_en,
                    ChatLang::Pl => &p.name_pl,
                    ChatLang::Uk => &p.name_uk,
                    ChatLang::Ru => &p.name_ru,
                };
                match lang {
                    ChatLang::Ru => match case {
                        "instr" => instrumental_phrase(raw),
                        _       => accusative_phrase(raw),
                    },
                    _ => raw.to_lowercase(),
                }
            })
            .unwrap_or_else(|| ing.slug_hint.clone())
    };

    let name_of = |ing: &ResolvedIngredient| -> String {
        name_of_case(ing, "acc")
    };

    let sep = match lang {
        ChatLang::Ru => " и ",
        ChatLang::En => " and ",
        ChatLang::Pl => " i ",
        ChatLang::Uk => " і ",
    };

    let names_of = |ings: &[&ResolvedIngredient], join: &str| -> String {
        ings.iter()
            .map(|i| name_of(i))
            .collect::<Vec<_>>()
            .join(join)
    };

    let names_of_case = |ings: &[&ResolvedIngredient], join: &str, case: &str| -> String {
        ings.iter()
            .map(|i| name_of_case(i, case))
            .collect::<Vec<_>>()
            .join(join)
    };

    // ── Walk the rule's step sequence ────────────────────────────────────
    let mut steps = Vec::new();
    let mut step_num: u8 = 0;

    // Localized chef tips
    let tip_text = |key: &str| -> Option<String> {
        let t = match (key, lang) {
            ("foam", ChatLang::Ru) => "Снимайте пену для прозрачного бульона",
            ("foam", ChatLang::En) => "Skim foam for a clear broth",
            ("foam", ChatLang::Pl) => "Zbieraj pianę dla przejrzystego bulionu",
            ("foam", ChatLang::Uk) => "Знімайте піну для прозорого бульйону",
            ("golden", ChatLang::Ru) => "До золотистого цвета, не пережаривайте",
            ("golden", ChatLang::En) => "Until golden, don't over-brown",
            ("golden", ChatLang::Pl) => "Do złotego koloru, nie przypalaj",
            ("golden", ChatLang::Uk) => "До золотистого кольору, не пересмажуйте",
            ("sear_first", ChatLang::Ru) => "Обжарьте мясо до корочки перед тушением",
            ("sear_first", ChatLang::En) => "Sear meat before braising for depth",
            ("sear_first", ChatLang::Pl) => "Obsmaż mięso przed duszeniem",
            ("sear_first", ChatLang::Uk) => "Обсмажте м'ясо перед тушкуванням",
            ("smoking", ChatLang::Ru) => "Масло должно слегка дымиться",
            ("smoking", ChatLang::En) => "Oil should be lightly smoking",
            ("smoking", ChatLang::Pl) => "Olej powinien się lekko dymić",
            ("smoking", ChatLang::Uk) => "Олія повинна ледь димитися",
            ("no_move", ChatLang::Ru) => "Не двигайте — дайте корочке сформироваться",
            ("no_move", ChatLang::En) => "Don't move — let the crust form",
            ("no_move", ChatLang::Pl) => "Nie ruszaj — pozwól się zrumienić",
            ("no_move", ChatLang::Uk) => "Не рухайте — дайте скоринці сформуватись",
            ("rest_after", ChatLang::Ru) => "Дайте отдохнуть 5 мин перед нарезкой",
            ("rest_after", ChatLang::En) => "Rest 5 min before cutting",
            ("rest_after", ChatLang::Pl) => "Odczekaj 5 min przed krojeniem",
            ("rest_after", ChatLang::Uk) => "Дайте відпочити 5 хв перед нарізкою",
            ("al_dente", ChatLang::Ru) => "Al dente — варите на 1 мин меньше",
            ("al_dente", ChatLang::En) => "Al dente — cook 1 min less than package",
            ("al_dente", ChatLang::Pl) => "Al dente — gotuj 1 min krócej",
            ("al_dente", ChatLang::Uk) => "Al dente — варіть на 1 хв менше",
            ("check_color", ChatLang::Ru) => "Проверяйте готовность по цвету корочки",
            ("check_color", ChatLang::En) => "Check doneness by crust color",
            ("check_color", ChatLang::Pl) => "Sprawdzaj gotowość po kolorze skórki",
            ("check_color", ChatLang::Uk) => "Перевіряйте готовність за кольором скоринки",
            _ => return None,
        };
        Some(t.to_string())
    };

    let mut add = |text: String, time: Option<u16>, temp_c: Option<u16>, tip_key: Option<&str>| {
        step_num += 1;
        let tip = tip_key.and_then(|k| tip_text(k));
        steps.push(CookingStep { step: step_num, text, time_min: time, temp_c, tip });
    };

    for step_rule in &rule.steps {
        // Collect ingredients that match ANY of the step's roles
        let matching: Vec<&ResolvedIngredient> = step_rule.roles.iter()
            .flat_map(|r| by_role(*r))
            .collect();

        // For steps that need ingredients: skip if none
        let needs_ingredients = matches!(step_rule.step,
            StepType::BoilProtein | StepType::BraiseProtein | StepType::SearProtein
            | StepType::GrillProtein | StepType::MarinateProtein
            | StepType::SauteAromatics | StepType::AddRoots | StepType::AddVegetables
            | StepType::AddAromatics | StepType::BoilBase | StepType::AddBase
            | StepType::AddLiquid | StepType::AddSpices | StepType::ChopAll | StepType::Dress
        );

        if needs_ingredients && matching.is_empty() {
            continue;
        }

        // Special handling for AddRoots: split vegetables into root vs leafy
        if step_rule.step == StepType::AddRoots {
            let roots: Vec<&ResolvedIngredient> = matching.iter()
                .filter(|i| {
                    let slug = i.resolved_slug.as_deref().unwrap_or("");
                    cooking_rules::is_root_vegetable(slug)
                })
                .copied()
                .collect();
            if !roots.is_empty() {
                let names = names_of(&roots, ", ");
                add(cooking_rules::step_text(StepType::AddRoots, &names, lang_code), step_rule.time_min, step_rule.temp_c, step_rule.tip);
            }
            continue;
        }

        // Special handling for AddVegetables in soup/stew: only non-root vegetables
        if step_rule.step == StepType::AddVegetables
            && matches!(dish_type, DishType::Soup | DishType::Stew)
        {
            let leafy: Vec<&ResolvedIngredient> = matching.iter()
                .filter(|i| {
                    let slug = i.resolved_slug.as_deref().unwrap_or("");
                    !cooking_rules::is_root_vegetable(slug)
                })
                .copied()
                .collect();
            if !leafy.is_empty() {
                let names = names_of(&leafy, ", ");
                add(cooking_rules::step_text(StepType::AddVegetables, &names, lang_code), step_rule.time_min, step_rule.temp_c, step_rule.tip);
            }
            continue;
        }

        // SauteAromatics: join with localized "and" (not ", ")
        // Dress: use instrumental case ("заправить майонезом", not "заправить майонез")
        let names = match step_rule.step {
            StepType::SauteAromatics => names_of(&matching, sep),
            StepType::Dress          => names_of_case(&matching, ", ", "instr"),
            _                        => names_of(&matching, ", "),
        };

        add(cooking_rules::step_text(step_rule.step, &names, lang_code), step_rule.time_min, step_rule.temp_c, step_rule.tip);
    }

    steps
}
// ── Display Name Builder ─────────────────────────────────────────────────────

/// Build an improved display name with goal prefix (multilingual):
///   Ru: "Лёгкий борщ с говядиной"
///   En: "Light borscht with beef"
///   Pl: "Lekki barszcz z wołowiną"
///   Uk: "Легкий борщ з яловичиною"
fn build_display_name(
    schema: &DishSchema,
    ingredients: &[ResolvedIngredient],
    _dish_type: DishType,
    goal: HealthGoal,
    lang: ChatLang,
) -> String {
    let dish_local = schema.dish_local.as_deref().unwrap_or(&schema.dish);

    // Detect if dish name already contains "with" preposition
    let has_with = {
        let d = dish_local.to_lowercase();
        d.contains(" с ") || d.contains(" with ") || d.contains(" z ") || d.contains(" з ")
    };

    // Base name with protein
    let base_name = if has_with {
        dish_local.to_string()
    } else {
        let protein_name = ingredients.iter()
            .find(|i| i.role == "protein")
            .and_then(|i| i.product.as_ref())
            .map(|p| match lang {
                ChatLang::Ru => instrumental_case(&p.name_ru),
                ChatLang::En => p.name_en.to_lowercase(),
                ChatLang::Pl => instrumental_case_pl(&p.name_pl),
                ChatLang::Uk => instrumental_case_uk(&p.name_uk),
            });

        if let Some(protein) = protein_name {
            let prep = match lang {
                ChatLang::Ru => "с",
                ChatLang::En => "with",
                ChatLang::Pl => "z",
                ChatLang::Uk => "з",
            };
            format!("{} {} {}", dish_local, prep, protein)
        } else {
            dish_local.to_string()
        }
    };

    // Goal prefix
    match (goal, lang) {
        (HealthGoal::LowCalorie, ChatLang::Ru) => format!("Лёгкий {}", lowercase_first(&base_name)),
        (HealthGoal::LowCalorie, ChatLang::En) => format!("Light {}", lowercase_first(&base_name)),
        (HealthGoal::LowCalorie, ChatLang::Pl) => format!("Lekki {}", lowercase_first(&base_name)),
        (HealthGoal::LowCalorie, ChatLang::Uk) => format!("Легкий {}", lowercase_first(&base_name)),

        (HealthGoal::HighProtein, ChatLang::Ru) => format!("Высокобелковый {}", lowercase_first(&base_name)),
        (HealthGoal::HighProtein, ChatLang::En) => format!("High-protein {}", lowercase_first(&base_name)),
        (HealthGoal::HighProtein, ChatLang::Pl) => format!("Wysokobiałkowy {}", lowercase_first(&base_name)),
        (HealthGoal::HighProtein, ChatLang::Uk) => format!("Високобілковий {}", lowercase_first(&base_name)),

        (HealthGoal::Balanced, _) => base_name,
    }
}

/// Lowercase first char: "Борщ" → "борщ" (for prefix composition)
fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Russian instrumental case for a single word.
/// Handles adjectives (-ая→-ой, -ые→-ыми, -ый→-ым) and nouns (-а→-ой, -о→-ом, etc.)
fn instrumental_word(word: &str) -> String {
    let lower = word.to_lowercase();
    // ── Adjective endings (check BEFORE nouns) ──
    if lower.ends_with("ая") {
        return format!("{}ой", &lower[..lower.len() - "ая".len()]);
    }
    if lower.ends_with("яя") {
        return format!("{}ей", &lower[..lower.len() - "яя".len()]);
    }
    if lower.ends_with("ое") || lower.ends_with("ее") {
        return format!("{}ым", &lower[..lower.len() - "ое".len()]);
    }
    if lower.ends_with("ые") {
        return format!("{}ыми", &lower[..lower.len() - "ые".len()]);
    }
    if lower.ends_with("ие") {
        return format!("{}ими", &lower[..lower.len() - "ие".len()]);
    }
    if lower.ends_with("ый") || lower.ends_with("ой") {
        return format!("{}ым", &lower[..lower.len() - "ый".len()]);
    }
    if lower.ends_with("ий") {
        return format!("{}им", &lower[..lower.len() - "ий".len()]);
    }
    // ── Noun endings ──
    // Neuter plural in -а (яйца) → -ами (яйцами)
    if is_neuter_plural_a(&lower) {
        return format!("{}ми", lower);  // яйца→яйцами
    }
    if lower.ends_with('а') {
        return format!("{}ой", &lower[..lower.len() - 'а'.len_utf8()]);
    }
    if lower.ends_with('я') {
        return format!("{}ей", &lower[..lower.len() - 'я'.len_utf8()]);
    }
    if lower.ends_with('о') {
        return format!("{}ом", &lower[..lower.len() - 'о'.len_utf8()]);
    }
    if lower.ends_with('ь') {
        return format!("{}ью", &lower[..lower.len() - 'ь'.len_utf8()]);
    }
    // -ец → -цем (перец→перцем)
    if lower.ends_with("ец") {
        return format!("{}цем", &lower[..lower.len() - "ец".len()]);
    }
    format!("{}ом", lower)
}

/// Russian accusative case for a single word.
/// Adjectives: -ая→-ую. Nouns: -а→-у, -я→-ю, inanimate unchanged.
fn accusative_word(word: &str) -> String {
    let lower = word.to_lowercase();
    // ── Adjective ending: -ая → -ую (пшеничная→пшеничную) ──
    if lower.ends_with("ая") {
        return format!("{}ую", &lower[..lower.len() - "ая".len()]);
    }
    if lower.ends_with("яя") {
        return format!("{}юю", &lower[..lower.len() - "яя".len()]);
    }
    // Other adj endings (-ое/-ые/-ий/-ый) → unchanged for inanimate
    // ── Neuter plural in -а (яйца, яблока) → unchanged (inanimate) ──
    if is_neuter_plural_a(&lower) {
        return lower;
    }
    // ── Noun endings ──
    if lower.ends_with('а') {
        return format!("{}у", &lower[..lower.len() - 'а'.len_utf8()]);
    }
    if lower.ends_with('я') {
        return format!("{}ю", &lower[..lower.len() - 'я'.len_utf8()]);
    }
    // Inanimate: acc = nom (морковь, лук, чеснок, яйцо, масло, перец)
    lower
}

/// Neuter plural nouns ending in -а that should NOT get -у in accusative.
/// "яйца" (egg-pl), "яблока" (apple-pl neuter form in compound names)
fn is_neuter_plural_a(word: &str) -> bool {
    matches!(word, "яйца" | "яблока" | "молока" | "масла")
}

/// Apply Russian accusative case to a compound name (word by word).
/// "Пшеничная мука" → "пшеничную муку"
/// "Куриные яйца" → "куриные яйца" (inanimate → unchanged)
fn accusative_phrase(name: &str) -> String {
    name.split_whitespace()
        .map(|w| accusative_word(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Apply Russian instrumental case to a compound name (word by word).
/// "Майонез" → "майонезом", "Подсолнечное масло" → "подсолнечным маслом"
fn instrumental_phrase(name: &str) -> String {
    name.split_whitespace()
        .map(|w| instrumental_word(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Russian instrumental case for display name (protein in "борщ с говядиной").
fn instrumental_case(name: &str) -> String {
    instrumental_phrase(name)
}

/// Ukrainian instrumental case for display name.
/// "Яловичина" → "яловичиною", "Курка" → "куркою"
fn instrumental_case_uk(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.ends_with('а') {
        return format!("{}ою", &lower[..lower.len() - 'а'.len_utf8()]);
    }
    if lower.ends_with('я') {
        return format!("{}ею", &lower[..lower.len() - 'я'.len_utf8()]);
    }
    if lower.ends_with('ь') {
        return format!("{}ю", &lower[..lower.len() - 'ь'.len_utf8()]);
    }
    format!("{}ом", lower)
}

/// Polish instrumental case for display name (narzędnik).
/// Used in "z [czym]": "z kurczakiem", "z wołowiną", "z łososiem".
/// Handles common Polish food noun endings.
fn instrumental_case_pl(name: &str) -> String {
    let lower = name.to_lowercase();

    // Multi-word: apply to each word (adj + noun: "Pierś z kurczaka" → "piersią z kurczaka")
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.len() > 1 {
        // For compound Polish names like "Pierś z kurczaka" — just decline the first word
        // and keep the rest (the prepositional phrase stays in genitive)
        let first = instrumental_word_pl(words[0]);
        return format!("{} {}", first, words[1..].join(" "));
    }

    instrumental_word_pl(&lower)
}

/// Polish instrumental case for a single word.
fn instrumental_word_pl(word: &str) -> String {
    let w = word.to_lowercase();

    // ── Adjective endings ──
    // -y → -ym (surowy → surowym) — masc adj
    // -i → -im (drobiowy → drobiowym — but -i is rare)
    // -a → -ą (gotowana → gotowaną) — fem adj
    // -e → -ym (świeże → świeżym) — neut adj

    // ── Noun endings (food-specific) ──
    // Special irregulars first
    if w == "kurczak" { return "kurczakiem".into(); }
    if w == "łosoś"  { return "łososiem".into(); }
    if w == "dorsz"   { return "dorszem".into(); }

    // Feminine -a → -ą: wołowina → wołowiną, cielęcina → cielęciną
    if w.ends_with('a') {
        return format!("{}ą", &w[..w.len() - 'a'.len_utf8()]);
    }
    // Feminine -ść → -ścią: pierś → piersią (but pierś ends in ś not ść)
    // Soft consonant ś/ń/ć/ź → +ą sometimes, but for food:
    if w.ends_with("ś") {
        return format!("{}ią", w);   // pierś → piersią
    }
    // Neuter -o → -em: mięso → mięsem (rare in display name context)
    if w.ends_with('o') {
        return format!("{}em", &w[..w.len() - 'o'.len_utf8()]);
    }
    // Masculine hard consonant → -em: kurczak → kurczakiem, dorsz → dorszem
    // Masculine -ek → -kiem: kurczak is matched above; indyk → indykiem
    if w.ends_with("ek") {
        return format!("{}kiem", &w[..w.len() - "ek".len()]);
    }
    if w.ends_with("ak") {
        return format!("{}iem", w);   // kurczak → kurczakiem (handled above), but general
    }
    // Default masculine: +em
    format!("{}em", w)
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
        ("celery-root", "celery"),
        ("celeriac", "celery"),
        ("water", "mineral-water"),
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

/// Human-friendly time: 89→" ⏱ ~1 ч 30 мин", 25→" ⏱ ~25 мин", 0→""
fn fmt_time(min: u16) -> String {
    if min == 0 { return String::new(); }
    // Round to nearest 5
    let rounded = ((min as f32 / 5.0).round() as u16).max(5);
    if rounded < 60 {
        format!(" ⏱ ~{} мин", rounded)
    } else {
        let h = rounded / 60;
        let m = rounded % 60;
        if m == 0 { format!(" ⏱ ~{} ч", h) }
        else { format!(" ⏱ ~{} ч {} мин", h, m) }
    }
}

// ── Text Formatting ──────────────────────────────────────────────────────────

pub fn format_recipe_text(card: &TechCard, lang: ChatLang) -> String {
    // ── Minimal text — the card UI is the main content ──
    // Only emit a short intro line + unresolved warnings.
    // All ingredients, steps, КБЖУ are rendered by the frontend RecipeCard.
    let dish = card.display_name.as_deref()
        .unwrap_or_else(|| card.dish_name_local.as_deref().unwrap_or(&card.dish_name));

    let total_time: u16 = card.steps.iter().filter_map(|s| s.time_min).sum();
    let time_str = fmt_time(total_time);

    let intro = match lang {
        ChatLang::Ru => format!(
            "🍽 **{}** — {} порц. • ~{:.0}г •{} {} ккал на порцию",
            dish, card.servings, card.total_output_g / card.servings as f32,
            time_str, card.per_serving_kcal,
        ),
        ChatLang::En => format!(
            "🍽 **{}** — {} serv. • ~{:.0}g •{} {} kcal/serv",
            dish, card.servings, card.total_output_g / card.servings as f32,
            time_str, card.per_serving_kcal,
        ),
        ChatLang::Pl => format!(
            "🍽 **{}** — {} porcji • ~{:.0}g •{} {} kcal/porcja",
            dish, card.servings, card.total_output_g / card.servings as f32,
            time_str, card.per_serving_kcal,
        ),
        ChatLang::Uk => format!(
            "🍽 **{}** — {} порц. • ~{:.0}г •{} {} ккал/порція",
            dish, card.servings, card.total_output_g / card.servings as f32,
            time_str, card.per_serving_kcal,
        ),
    };

    let mut out = vec![intro];

    if !card.unresolved.is_empty() {
        let warn = match lang {
            ChatLang::Ru => format!("⚠️ Не в базе: {}", card.unresolved.join(", ")),
            ChatLang::En => format!("⚠️ Not in DB: {}", card.unresolved.join(", ")),
            ChatLang::Pl => format!("⚠️ Brak w bazie: {}", card.unresolved.join(", ")),
            ChatLang::Uk => format!("⚠️ Нема в базі: {}", card.unresolved.join(", ")),
        };
        out.push(warn);
    }

    out.join("\n")
}

pub fn state_label<'a>(state: &'a str, lang: ChatLang) -> &'a str {
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

/// Russian state label with gender agreement.
/// "Свекла" (fem) → "варёная", "Лук" (masc) → "пассерованный", "Молоко" (neut) → "варёное"
fn state_label_ru(state: &str, name_ru: &str) -> String {
    let gender = ru_gender(name_ru);
    match state {
        "raw" => match gender { 'f' => "сырая", 'n' => "сырое", _ => "сырой" }.into(),
        "boiled" => match gender { 'f' => "варёная", 'n' => "варёное", _ => "варёный" }.into(),
        "fried" => match gender { 'f' => "жареная", 'n' => "жареное", _ => "жареный" }.into(),
        "sauteed" => match gender { 'f' => "пассерованная", 'n' => "пассерованное", _ => "пассерованный" }.into(),
        "baked" => match gender { 'f' => "запечённая", 'n' => "запечённое", _ => "запечённый" }.into(),
        "grilled" => "гриль".into(),
        "steamed" => "на пару".into(),
        "smoked" => match gender { 'f' => "копчёная", 'n' => "копчёное", _ => "копчёный" }.into(),
        _ => state.into(),
    }
}

/// Guess Russian grammatical gender from the nominative form.
/// -а/-я → feminine, -о/-е → neuter, else → masculine.
/// Special case: some food words ending in -ь are feminine (морковь, фасоль…).
fn ru_gender(name: &str) -> char {
    let lower = name.to_lowercase();
    let lower = lower.trim();

    // Words ending in -ь need special handling: could be masc or fem
    if lower.ends_with('ь') {
        // Feminine food nouns ending in -ь
        const FEM_SOFT: &[&str] = &[
            "морковь", "фасоль", "соль", "ваниль", "зелень",
            "форель", "печень", "стручковая фасоль",
        ];
        for w in FEM_SOFT {
            if lower == *w { return 'f'; }
        }
        // Default for -ь: masculine (картофель, имбирь, миндаль, щавель…)
        return 'm';
    }

    if lower.ends_with('а') || lower.ends_with('я') { 'f' }
    else if lower.ends_with('о') || lower.ends_with('е') || lower.ends_with('ё') { 'n' }
    else { 'm' }
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

    #[test]
    fn ru_gender_feminine_soft_sign() {
        // Морковь, фасоль — feminine, ending in -ь
        assert_eq!(ru_gender("Морковь"), 'f');
        assert_eq!(ru_gender("Фасоль"), 'f');
        assert_eq!(ru_gender("Форель"), 'f');
        assert_eq!(ru_gender("Печень"), 'f');
        // Картофель, имбирь — masculine, ending in -ь
        assert_eq!(ru_gender("Картофель"), 'm');
        assert_eq!(ru_gender("Имбирь"), 'm');
    }

    #[test]
    fn state_label_morkov_feminine() {
        let label = state_label_ru("sauteed", "Морковь");
        assert_eq!(label, "пассерованная");
    }

    #[test]
    fn state_label_kartoshka_feminine() {
        // Картошка ends in -а → feminine
        let label = state_label_ru("boiled", "Картошка");
        assert_eq!(label, "варёная");
    }

    // ═══ Russian Grammar: compound names ═════════════════════════════════

    #[test]
    fn accusative_simple_nouns() {
        assert_eq!(accusative_word("Говядина"), "говядину");
        assert_eq!(accusative_word("Свинина"), "свинину");
        assert_eq!(accusative_word("Мука"), "муку");
        assert_eq!(accusative_word("Морковь"), "морковь");  // inanimate
        assert_eq!(accusative_word("Лук"), "лук");          // inanimate
        assert_eq!(accusative_word("Чеснок"), "чеснок");    // inanimate
    }

    #[test]
    fn accusative_adjective_aya() {
        // -ая → -ую
        assert_eq!(accusative_word("Пшеничная"), "пшеничную");
        assert_eq!(accusative_word("Каменная"), "каменную");
    }

    #[test]
    fn accusative_compound_names() {
        assert_eq!(accusative_phrase("Пшеничная мука"), "пшеничную муку");
        assert_eq!(accusative_phrase("Соль каменная"), "соль каменную");  // adj after noun
        assert_eq!(accusative_phrase("Куриные яйца"), "куриные яйца");   // inanimate pl → unchanged
        assert_eq!(accusative_phrase("Чёрный перец"), "чёрный перец");    // inanimate m → unchanged
        assert_eq!(accusative_phrase("Говядина"), "говядину");
    }

    #[test]
    fn instrumental_simple_nouns() {
        assert_eq!(instrumental_word("Говядина"), "говядиной");
        assert_eq!(instrumental_word("Майонез"), "майонезом");
        assert_eq!(instrumental_word("Масло"), "маслом");
        assert_eq!(instrumental_word("Морковь"), "морковью");
        assert_eq!(instrumental_word("Перец"), "перцем");
    }

    #[test]
    fn instrumental_adjectives() {
        assert_eq!(instrumental_word("Подсолнечное"), "подсолнечным");
        assert_eq!(instrumental_word("Куриные"), "куриными");
        assert_eq!(instrumental_word("Чёрный"), "чёрным");
        assert_eq!(instrumental_word("Пшеничная"), "пшеничной");
    }

    #[test]
    fn instrumental_compound_names() {
        assert_eq!(instrumental_phrase("Подсолнечное масло"), "подсолнечным маслом");
        assert_eq!(instrumental_phrase("Майонез"), "майонезом");
        assert_eq!(instrumental_phrase("Говядина"), "говядиной");
        assert_eq!(instrumental_phrase("Чёрный перец"), "чёрным перцем");
        assert_eq!(instrumental_phrase("Куриные яйца"), "куриными яйцами");
    }
}
