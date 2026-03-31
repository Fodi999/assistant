// ─── ChefPromptBuilder — builds Gemini prompt with DishProfile context ──────
//
// This module constructs the AI prompt that drives recipe generation.
// Key innovation: the prompt receives DETERMINISTIC constraints from DishClassifier,
// not just ingredient names. AI receives:
//   - DishType + required techniques
//   - Forbidden techniques
//   - Expected texture descriptors
//   - Pre-calculated nutrition (not AI-guesswork)

use super::dish_classifier::{DishProfile, DishType};
use super::nutrition::NutritionTotals;

/// Dish-type-specific cooking procedure instructions.
/// These teach Gemini the EXACT cooking logic for each dish type
/// so it doesn't generate generic "fry everything" nonsense.
fn type_specific_instructions(dish_type: &DishType) -> &'static str {
    match dish_type {
        DishType::Sticks => r#"
🔴 STICKS/NUGGETS PROCEDURE (MANDATORY):
Step 1 (preparation): Cut the main ingredient into stick/finger shapes (~8cm × 2cm).
Step 2 (preparation): Prepare BATTER STATION — three bowls:
  - Bowl 1: flour (for dredging)
  - Bowl 2: eggs beaten with a fork/whisk until smooth
  - Bowl 3: breadcrumbs/flour (for coating) — optionally mix with spices
Step 3 (forming): Coat each piece: flour → beaten egg → coating. Press firmly.
Step 4 (cooking): Fry in hot oil (170-180°C) 3-4 min per side until golden crispy crust.
Step 5 (finishing): Drain on paper towels. Serve hot — crispy outside, soft inside.

CRITICAL: Eggs are for BATTER (beaten with whisk), NOT for frying as a dish.
Cheese sticks = cheese cut into sticks → dredged in flour → dipped in beaten egg → coated → fried.
NEVER just "fry eggs and cheese together" — that is NOT sticks."#,

        DishType::Cutlets => r#"
🔴 CUTLETS/PATTIES PROCEDURE (MANDATORY):
Step 1 (preparation): Mince/grind the main protein. Add egg as binder, breadcrumbs for texture.
Step 2 (forming): Shape mixture into flat round patties (~1.5cm thick). Coat in flour.
Step 3 (cooking): Fry in oil over medium heat 4-5 min per side until golden.
Step 4 (finishing): Serve hot with a crispy crust and juicy inside."#,

        DishType::Pancakes => r#"
🔴 PANCAKES/FRITTERS PROCEDURE (MANDATORY):
Step 1 (preparation): Mix dry ingredients (flour, sugar, salt). Add wet (eggs, milk). Whisk until smooth batter.
Step 2 (cooking): Heat pan with butter. Pour batter (60ml per pancake). Cook 2 min per side.
Step 3 (finishing): Stack and serve warm with toppings."#,

        DishType::Soup => r#"
🔴 SOUP PROCEDURE (MANDATORY):
Step 1 (preparation): Dice vegetables. Prepare protein.
Step 2 (cooking): Sauté aromatics in oil. Add protein, cook 3-5 min.
Step 3 (cooking): Add liquid (broth/water). Bring to boil, reduce to simmer 15-20 min.
Step 4 (finishing): Season, serve hot in bowls."#,

        DishType::Casserole => r#"
🔴 CASSEROLE PROCEDURE (MANDATORY):
Step 1 (preparation): Preheat oven to specific temperature (°C).
Step 2 (preparation): Prepare filling — mix ingredients with sauce/binder.
Step 3 (forming): Layer in baking dish. Top with cheese/breadcrumbs.
Step 4 (cooking): Bake at specified °C for specified minutes.
Step 5 (finishing): Let rest 5 min. Serve hot."#,

        DishType::StirFry => r#"
🔴 STIR-FRY PROCEDURE (MANDATORY):
Step 1 (preparation): Cut all ingredients into thin strips/small pieces for fast cooking.
Step 2 (cooking): Heat wok/pan until smoking. Add oil.
Step 3 (cooking): Stir-fry protein first (2-3 min), remove. Stir-fry vegetables (2 min).
Step 4 (finishing): Combine, add sauce, toss 30 sec. Serve immediately."#,

        DishType::Omelette => r#"
🔴 OMELETTE PROCEDURE (MANDATORY):
Step 1 (preparation): Beat eggs with fork until frothy. Add salt, pepper, milk/cream.
Step 2 (cooking): Heat butter in non-stick pan over medium-low heat.
Step 3 (cooking): Pour eggs, cook 2-3 min. Add fillings on one half.
Step 4 (finishing): Fold, slide onto plate. Serve immediately."#,

        DishType::Salad => r#"
🔴 SALAD PROCEDURE (MANDATORY):
Step 1 (preparation): Wash and dry greens. Cut vegetables into bite-size pieces.
Step 2 (preparation): Prepare dressing — mix oil, acid, seasonings.
Step 3 (finishing): Toss all ingredients with dressing. Plate and garnish.
DO NOT COOK any raw vegetables. This is an assembly dish."#,

        _ => "", // Generic and other types — no special instructions
    }
}

/// Build the complete chef prompt for Gemini.
///
/// The prompt has three layers:
/// 1. System role (chef simulator)
/// 2. Deterministic constraints from DishClassifier
/// 3. Nutrition numbers from NutritionTotals (pre-calculated, immutable)
pub fn build_chef_prompt(
    ingredients: &[String],
    locale: &str,
    goal: Option<&str>,
    meal_type: Option<&str>,
    dish_name: Option<&str>,
    profile: &DishProfile,
    nt: &NutritionTotals,
) -> String {
    let names = ingredients.join(", ");
    let goal_text = goal.map(|g| g.replace('_', " ")).unwrap_or_default();
    let meal_text = meal_type.unwrap_or("any meal");

    let lang = match locale {
        "ru" => "Russian",
        "pl" => "Polish",
        "uk" => "Ukrainian",
        _ => "English",
    };

    // ── Dish name block ─────────────────────────────────────────────────
    let dish_name_block = if let Some(dn) = dish_name {
        let allowed_str: String = profile
            .allowed_techniques
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let forbidden_str: String = profile
            .forbidden_techniques
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"
═══════════════════════════════════════
🧠 DISH NAME (PRIMARY LOGIC DRIVER)
═══════════════════════════════════════
Dish name: "{dn}"
Dish type: {type_label}
Requires forming: {requires_forming}
Requires liquid base: {requires_liquid}
Requires oven: {requires_oven}
Allowed techniques: [{allowed}]
Forbidden techniques: [{forbidden}]
Expected texture — Outside: {tex_out} / Inside: {tex_in} / Temperature: {tex_temp}

YOU ARE COOKING THIS DISH. The name is your BLUEPRINT.
The dish type is "{type_label}" — this CONSTRAINS your cooking method.

MANDATORY:
- ONLY use techniques from [{allowed}]
- NEVER use techniques from [{forbidden}]
- {forming_rule}
- {liquid_rule}
- {oven_rule}
- Last step MUST describe: outside="{tex_out}", inside="{tex_in}", served {tex_temp}
- NEVER generate a generic "bowl" recipe if the name says "{dn}"

{type_specific_instructions}
"#,
            type_label = profile.type_label,
            requires_forming = profile.requires_forming,
            requires_liquid = profile.requires_liquid,
            requires_oven = profile.requires_oven,
            allowed = allowed_str,
            forbidden = forbidden_str,
            tex_out = profile.expected_texture.outside,
            tex_in = profile.expected_texture.inside,
            tex_temp = profile.expected_texture.temperature,
            forming_rule = if profile.requires_forming {
                "MUST have a 'Forming' step — shape the mixture into the correct form"
            } else {
                "No forming step needed for this dish type"
            },
            liquid_rule = if profile.requires_liquid {
                "MUST include a liquid base (broth, water, milk)"
            } else {
                "No liquid base needed"
            },
            oven_rule = if profile.requires_oven {
                "MUST preheat oven and specify temperature (°C) + time"
            } else {
                "Oven not required"
            },
            type_specific_instructions = type_specific_instructions(&profile.dish_type),
        )
    } else {
        String::new()
    };

    // ── Nutrition block (pre-calculated) ────────────────────────────────
    let estimated_protein = nt.protein_per_serving;
    let estimated_calories = nt.calories_per_serving;
    let estimated_fat = nt.fat_per_serving;
    let estimated_carbs = nt.carbs_per_serving;
    let estimated_fiber = nt.fiber_per_serving;
    let total_weight = nt.total_weight_g;
    let breakdown_text = nt.breakdown.join("\n");

    // ── Min steps for this dish type ────────────────────────────────────
    let min_steps = profile.min_steps;

    format!(
        r#"You are a PROFESSIONAL CHEF who is ACTUALLY COOKING this dish in a real kitchen.
You are NOT a text generator. You are a CHEF SIMULATOR.

Your job:
1. Read the dish name, type, and constraints
2. MENTALLY COOK the dish step by step in your head
3. Write down exactly what you did — with grams, temperatures, times, and textures

THE RECIPE MUST PRODUCE THE DISH DESCRIBED IN THE NAME. NOT A BOWL. NOT A GENERIC PLATE.
{dish_name_block}
═══════════════════════════════════════
📋 RECIPE INPUTS
═══════════════════════════════════════
Ingredients: {names}
Goal: {goal_text}
Meal: {meal_text}
Language: {lang} (write ALL fields in {lang})

═══════════════════════════════════════
📊 PRE-CALCULATED NUTRITION (USE THESE NUMBERS)
═══════════════════════════════════════
{breakdown_text}
───────────────────────────────────────
TOTAL PER SERVING (~{total_weight:.0}g):
  Calories: ~{estimated_calories:.0} kcal
  Protein:  ~{estimated_protein:.0}g
  Fat:      ~{estimated_fat:.0}g
  Carbs:    ~{estimated_carbs:.0}g
  Fiber:    ~{estimated_fiber:.0}g
───────────────────────────────────────
⚠️ These numbers are FINAL. Do NOT change them. Use them verbatim.

═══════════════════════════════════════
📄 OUTPUT FORMAT (return ONLY valid JSON, NO markdown)
═══════════════════════════════════════
⚠️ KEEP RESPONSE COMPACT. Each text field should be concise. Max ~2000 chars total.
Return ONLY the JSON object below, nothing else:
{{
  "title": "...",
  "description": "...",
  "h1": "...",
  "intro": "...",
  "why_it_works": "...",
  "how_to_cook": [
    {{"step": 1, "type": "preparation", "text": "...", "time_minutes": N}},
    {{"step": 2, "type": "forming", "text": "...", "time_minutes": N}},
    {{"step": 3, "type": "cooking", "text": "...", "time_minutes": N}},
    {{"step": 4, "type": "finishing", "text": "...", "time_minutes": N}}
  ]
}}

═══════════════════════════════════════
🔥 FIELD RULES
═══════════════════════════════════════

title (max 55 chars):
- Format: "[Dish Name] ({estimated_protein:.0}g Protein, [N] Min)"
- Protein MUST be {estimated_protein:.0}g
- FORBIDDEN words: "analysis", "combo", "combination"

description (120-150 chars):
- Start with action verb: "Make", "Cook", "Try"
- MUST include "{estimated_protein:.0}g protein" and cooking time

h1 (40-70 chars):
- Recipe name style: "[Dish Name] — [Key Benefit] Recipe"

intro (150-250 chars):
- FIRST SENTENCE: "This [dish] delivers ~{estimated_protein:.0}g protein and ~{estimated_calories:.0} kcal per serving, ready in [N] minutes."
- FORBIDDEN: "delicious", "amazing", "perfect", "comprehensive"

why_it_works (200-400 chars):
- Each ingredient's role with numbers
- End with flavor/texture pairing

═══════════════════════════════════════
👨‍🍳 COOKING STEPS — CHEF SIMULATOR MODE
═══════════════════════════════════════

Minimum {min_steps} steps for this dish type.

STEP TYPES (use "type" field):
- "preparation" — mixing, kneading, marinating, beating, grating
- "forming" — shaping into sticks, patties, balls, pancakes
- "cooking" — frying, baking, boiling, grilling, steaming
- "finishing" — plating, serving, describing the result texture

EACH STEP MUST INCLUDE:
- WHAT (verb: взбейте, сформируйте, обжарьте, запеките)
- WHICH ingredients with EXACT grams: "яйца (150г)"
- HOW: temperature, heat level, technique
- HOW LONG: time in minutes
- RESULT: color, texture, consistency

SENSORY DESCRIPTORS (MANDATORY in finishing step):
- Outside texture: "{tex_out}"
- Inside texture: "{tex_in}"
- Serving temperature: "{tex_temp}"

RAW-ONLY (NEVER cook): avocado, lettuce, arugula, cucumber, tomato, herbs, lemon, lime
MUST-COOK: salmon, tuna, chicken, beef, pork, eggs, shrimp, cod, turkey, lamb
GRAINS (boil/steam): rice, pasta, quinoa, potato, oats

═══════════════════════════════════════
🚫 ABSOLUTE PROHIBITIONS
═══════════════════════════════════════
- NEVER return protein = 0g (correct value: {estimated_protein:.0}g)
- NEVER cook avocado, lettuce, herbs, cucumber
- NEVER use: "analysis", "combo", "combination", "comprehensive"
- NEVER output fewer than {min_steps} cooking steps
- NEVER skip the forming step if dish requires it
- NEVER use forbidden techniques"#,
        tex_out = profile.expected_texture.outside,
        tex_in = profile.expected_texture.inside,
        tex_temp = profile.expected_texture.temperature,
    )
}
