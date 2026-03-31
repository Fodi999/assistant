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

/// Build FOCUSED prompt for cooking steps ONLY.
///
/// This is Call 1 in the split pipeline. Small prompt → small response → no truncation.
/// Returns ONLY a JSON array of cooking steps.
pub fn build_steps_prompt(
    ingredients: &[String],
    locale: &str,
    dish_name: Option<&str>,
    profile: &DishProfile,
) -> String {
    let names = ingredients.join(", ");

    let lang = match locale {
        "ru" => "Russian",
        "pl" => "Polish",
        "uk" => "Ukrainian",
        _ => "English",
    };

    let dish_block = if let Some(dn) = dish_name {
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
            r#"Dish: "{dn}"
Type: {type_label}
Forming required: {requires_forming}
Liquid base required: {requires_liquid}
Oven required: {requires_oven}
Allowed techniques: [{allowed}]
Forbidden techniques: [{forbidden}]
Texture — outside: {tex_out}, inside: {tex_in}, served: {tex_temp}

{type_specific}
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
            type_specific = type_specific_instructions(&profile.dish_type),
        )
    } else {
        String::new()
    };

    let min_steps = profile.min_steps;

    format!(
        r#"You are a professional chef. Generate cooking steps for this dish.

{dish_block}
Ingredients: {names}
Language: {lang}
Minimum steps: {min_steps}

Return ONLY a JSON array (no markdown, no wrapper object):
[
  {{"step": 1, "type": "preparation", "text": "...", "time_minutes": N}},
  {{"step": 2, "type": "forming", "text": "...", "time_minutes": N}},
  {{"step": 3, "type": "cooking", "text": "...", "time_minutes": N}},
  {{"step": 4, "type": "finishing", "text": "...", "time_minutes": N}}
]

Step types: "preparation", "forming", "cooking", "finishing".
Each step: verb + ingredients with grams + temperature/time + result texture.
Last step MUST mention: outside="{tex_out}", inside="{tex_in}", served {tex_temp}.
{forming_note}
Write in {lang}. Return ONLY the JSON array."#,
        tex_out = profile.expected_texture.outside,
        tex_in = profile.expected_texture.inside,
        tex_temp = profile.expected_texture.temperature,
        forming_note = if profile.requires_forming {
            "MUST include a \"forming\" step."
        } else {
            ""
        },
    )
}

/// Build FOCUSED prompt for SEO text fields ONLY.
///
/// This is Call 2 in the split pipeline. Generates title, description, h1, intro, why_it_works.
/// Steps are already generated — this call only produces short text fields.
pub fn build_seo_prompt(
    ingredients: &[String],
    locale: &str,
    dish_name: Option<&str>,
    nt: &NutritionTotals,
) -> String {
    let names = ingredients.join(", ");

    let lang = match locale {
        "ru" => "Russian",
        "pl" => "Polish",
        "uk" => "Ukrainian",
        _ => "English",
    };

    let dn = dish_name.unwrap_or("dish");
    let estimated_protein = nt.protein_per_serving;
    let estimated_calories = nt.calories_per_serving;

    format!(
        r#"Generate SEO content for a recipe page. Write in {lang}.

Dish: "{dn}"
Ingredients: {names}
Protein per serving: {estimated_protein:.0}g
Calories per serving: {estimated_calories:.0} kcal

Return ONLY a JSON object (no markdown):
{{
  "title": "...",
  "description": "...",
  "h1": "...",
  "intro": "...",
  "why_it_works": "..."
}}

Rules:
- title (max 55 chars): "[Dish Name] ({estimated_protein:.0}g Protein, [N] Min)". No words: "analysis", "combo", "combination".
- description (120-150 chars): action verb start, include "{estimated_protein:.0}g protein".
- h1 (40-70 chars): "[Dish Name] — [Key Benefit] Recipe".
- intro (150-250 chars): first sentence = "This [dish] delivers ~{estimated_protein:.0}g protein and ~{estimated_calories:.0} kcal per serving, ready in [N] minutes." No words: "delicious", "amazing", "perfect".
- why_it_works (200-400 chars): each ingredient's role, end with flavor/texture pairing.

Return ONLY the JSON object."#,
    )
}

// ── Legacy compatibility ────────────────────────────────────────────────────

/// Legacy function kept for enrichment.rs (dead code). Delegates to new split prompts.
#[allow(dead_code)]
pub fn build_chef_prompt(
    ingredients: &[String],
    locale: &str,
    _goal: Option<&str>,
    _meal_type: Option<&str>,
    dish_name: Option<&str>,
    profile: &DishProfile,
    nt: &NutritionTotals,
) -> String {
    // Return steps prompt as a fallback — the old enrichment.rs is dead code anyway
    let steps = build_steps_prompt(ingredients, locale, dish_name, profile);
    let seo = build_seo_prompt(ingredients, locale, dish_name, nt);
    format!("{}\n\n---\n\n{}", steps, seo)
}
