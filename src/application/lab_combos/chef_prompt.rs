// ─── ChefPromptBuilder — builds Gemini prompt with DishProfile context ──────
//
// This module constructs the AI prompt that drives recipe generation.
// Key innovation: the prompt receives DETERMINISTIC constraints from DishClassifier,
// not just ingredient names. AI receives:
//   - DishType + required techniques
//   - Forbidden techniques
//   - Expected texture descriptors
//   - Pre-calculated nutrition (not AI-guesswork)

use super::dish_classifier::DishProfile;
use super::nutrition::NutritionTotals;

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
📄 OUTPUT FORMAT (return ONLY valid JSON)
═══════════════════════════════════════
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
