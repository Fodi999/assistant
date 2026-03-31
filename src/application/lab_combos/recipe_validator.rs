// ─── RecipeValidator — semantic validation using DishProfile ────────────────
//
// Validates AI-generated recipe steps against the deterministic DishClassifier.
// NOT just formal checks (">=3 steps"), but SEMANTIC validation:
//   - technique matches dish type (no boiling sticks)
//   - forming step present when dish requires it
//   - all ingredients accounted for with grams
//   - last step describes expected texture

use super::dish_classifier::{CookingTechnique, DishProfile, DishType};

/// Result of validating a set of cooking steps against a DishProfile.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub problems: Vec<String>,
}

/// Validate AI-generated steps against the dish profile.
///
/// Returns `ValidationResult` with `is_valid=false` if any semantic check fails.
pub fn validate_recipe(
    profile: &DishProfile,
    steps_json: &serde_json::Value,
    _dish_name: Option<&str>,
    ingredients: &[String],
) -> ValidationResult {
    let mut problems = Vec::new();

    let steps = match steps_json.as_array() {
        Some(arr) => arr,
        None => {
            return ValidationResult {
                is_valid: false,
                problems: vec!["how_to_cook is not an array".to_string()],
            };
        }
    };

    // ── Check 1: minimum step count ─────────────────────────────────────
    if steps.len() < profile.min_steps {
        problems.push(format!(
            "NOT ENOUGH STEPS: got {}, need at least {} for {}",
            steps.len(),
            profile.min_steps,
            profile.type_label,
        ));
    }

    // Collect all step text for analysis
    let all_text: String = steps
        .iter()
        .filter_map(|s| s.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    let all_lower = all_text.to_lowercase();

    // ── Check 2: steps mention grams ────────────────────────────────────
    let has_grams = all_text.contains('г')
        || all_text.contains("g)")
        || all_text.contains("g ")
        || all_text.contains("гр");
    if !has_grams {
        problems.push("STEPS DON'T MENTION INGREDIENT GRAMS".to_string());
    }

    // ── Check 3: allowed cooking technique present ──────────────────────
    let has_allowed_technique = profile.allowed_techniques.iter().any(|t| {
        technique_keywords_present(t, &all_lower)
    });
    if !has_allowed_technique && profile.dish_type != DishType::Generic {
        let allowed_str: Vec<String> = profile
            .allowed_techniques
            .iter()
            .map(|t| t.to_string())
            .collect();
        problems.push(format!(
            "NO ALLOWED TECHNIQUE FOUND for {} — expected one of: {}",
            profile.type_label,
            allowed_str.join(", "),
        ));
    }

    // ── Check 4: forbidden technique detected ───────────────────────────
    for forbidden in &profile.forbidden_techniques {
        if technique_keywords_present(forbidden, &all_lower) {
            problems.push(format!(
                "FORBIDDEN TECHNIQUE '{}' detected for dish type '{}'",
                forbidden, profile.type_label,
            ));
        }
    }

    // ── Check 5: forming step present when required ─────────────────────
    if profile.requires_forming {
        let has_forming = all_lower.contains("сформируйте")
            || all_lower.contains("формиру")
            || all_lower.contains("form ")
            || all_lower.contains("shape ")
            || all_lower.contains("скатайте")
            || all_lower.contains("вылеп")
            || all_lower.contains("раскатайте")
            || all_lower.contains("нарежьте на палочки")
            || all_lower.contains("палочки длиной")
            || all_lower.contains("котлет")
            || all_lower.contains("шарик")
            || all_lower.contains("лепёшк");
        if !has_forming {
            problems.push(format!(
                "MISSING FORMING STEP — dish type '{}' requires forming (палочки/котлеты/шарики)",
                profile.type_label,
            ));
        }
    }

    // ── Check 6: liquid base when required (soup) ───────────────────────
    if profile.requires_liquid {
        let has_liquid = all_lower.contains("бульон")
            || all_lower.contains("вод")
            || all_lower.contains("broth")
            || all_lower.contains("water")
            || all_lower.contains("stock")
            || all_lower.contains("молок")
            || all_lower.contains("milk")
            || all_lower.contains("жидкост")
            || all_lower.contains("liquid");
        if !has_liquid {
            problems.push(format!(
                "MISSING LIQUID BASE — dish type '{}' requires liquid (broth/water/milk)",
                profile.type_label,
            ));
        }
    }

    // ── Check 7: oven when required (casserole/baked) ───────────────────
    if profile.requires_oven {
        let has_oven = all_lower.contains("духовк")
            || all_lower.contains("oven")
            || all_lower.contains("°c")
            || all_lower.contains("°f")
            || all_lower.contains("градус")
            || all_lower.contains("degree")
            || all_lower.contains("запек")
            || all_lower.contains("bake");
        if !has_oven {
            problems.push(format!(
                "MISSING OVEN STEP — dish type '{}' requires oven with temperature",
                profile.type_label,
            ));
        }
    }

    // ── Check 8: last step describes texture/result ─────────────────────
    if let Some(last_step) = steps.last() {
        let last_text = last_step
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_lowercase();
        let has_texture_desc = last_text.contains("хрустящ")
            || last_text.contains("мягк")
            || last_text.contains("золотист")
            || last_text.contains("горяч")
            || last_text.contains("нежн")
            || last_text.contains("сочн")
            || last_text.contains("crispy")
            || last_text.contains("golden")
            || last_text.contains("tender")
            || last_text.contains("hot")
            || last_text.contains("creamy")
            || last_text.contains("подавайте")
            || last_text.contains("serve");
        if !has_texture_desc {
            problems.push("LAST STEP doesn't describe expected texture/result".to_string());
        }
    }

    // ── Check 9: all ingredients mentioned ──────────────────────────────
    let mut missing_ingredients = Vec::new();
    for ing in ingredients {
        let ing_lower = ing.to_lowercase();
        // Check if ingredient (or a close synonym) appears in any step
        let found = all_lower.contains(&ing_lower)
            || all_lower.contains(&ing_lower.replace('-', " "))
            || all_lower.contains(&ing_lower.replace('-', ""));
        if !found {
            missing_ingredients.push(ing.clone());
        }
    }
    if !missing_ingredients.is_empty() && missing_ingredients.len() > ingredients.len() / 2 {
        problems.push(format!(
            "INGREDIENTS NOT MENTIONED in steps: {}",
            missing_ingredients.join(", "),
        ));
    }

    ValidationResult {
        is_valid: problems.is_empty(),
        problems,
    }
}

/// Build a fix prompt describing what's wrong with the recipe.
pub fn build_fix_prompt(
    problems: &[String],
    all_step_text: &str,
    dish_name: Option<&str>,
    ingredients: &str,
    lang: &str,
    profile: &DishProfile,
) -> String {
    let problems_text = problems
        .iter()
        .map(|p| format!("- {}", p))
        .collect::<Vec<_>>()
        .join("\n");

    let technique_hint = profile
        .allowed_techniques
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"The following recipe has FAILED quality validation. Fix it.

PROBLEMS FOUND:
{problems_text}

DISH TYPE: {type_label}
ALLOWED TECHNIQUES: {technique_hint}
REQUIRES FORMING: {requires_forming}
REQUIRES LIQUID BASE: {requires_liquid}
REQUIRES OVEN: {requires_oven}
EXPECTED TEXTURE — Outside: {tex_out}, Inside: {tex_in}, Temperature: {tex_temp}

ORIGINAL RECIPE STEPS:
{all_step_text}

DISH NAME: "{dish_name}"
INGREDIENTS: {ingredients}
LANGUAGE: {lang}

RULES:
- Minimum {min_steps} steps for this dish type
- Each step MUST mention specific ingredients with grams in parentheses
- MUST use one of these techniques: {technique_hint}
- {forming_rule}
- Last step MUST describe result texture: "{tex_out}" outside, "{tex_in}" inside
- Write in {lang}

Return ONLY a valid JSON array of cooking steps:
[
  {{"step": 1, "text": "...", "time_minutes": N}},
  {{"step": 2, "text": "...", "time_minutes": N}},
  ...
]"#,
        type_label = profile.type_label,
        requires_forming = profile.requires_forming,
        requires_liquid = profile.requires_liquid,
        requires_oven = profile.requires_oven,
        tex_out = profile.expected_texture.outside,
        tex_in = profile.expected_texture.inside,
        tex_temp = profile.expected_texture.temperature,
        dish_name = dish_name.unwrap_or("(not provided)"),
        min_steps = profile.min_steps,
        forming_rule = if profile.requires_forming {
            "MUST have a FORMING step (shape the dough/mixture into the required form)"
        } else {
            "No forming step needed"
        },
    )
}

// ── Internal: keyword detection per technique ───────────────────────────────

fn technique_keywords_present(technique: &CookingTechnique, text: &str) -> bool {
    match technique {
        CookingTechnique::Fry => {
            text.contains("обжар") || text.contains("жар") || text.contains("сковород")
                || text.contains("fry") || text.contains("sear") || text.contains("sauté")
                || text.contains("pan ")
        }
        CookingTechnique::DeepFry => {
            text.contains("фритюр") || text.contains("deep fry") || text.contains("deep-fry")
                || text.contains("во фритюре") || text.contains("масле") && text.contains("погру")
        }
        CookingTechnique::Bake => {
            text.contains("духовк") || text.contains("запек") || text.contains("bake")
                || text.contains("oven") || text.contains("°c") || text.contains("°f")
                || text.contains("градус")
        }
        CookingTechnique::Boil => {
            text.contains("свар") || text.contains("кипя") || text.contains("варит")
                || text.contains("boil") || text.contains("кастрюл") || text.contains("кипящ")
        }
        CookingTechnique::Steam => {
            text.contains("пар") || text.contains("steam") || text.contains("на пару")
                || text.contains("пароварк")
        }
        CookingTechnique::Grill => {
            text.contains("гриль") || text.contains("grill") || text.contains("на углях")
                || text.contains("решётк") || text.contains("barbecue")
        }
        CookingTechnique::StirFry => {
            text.contains("вок") || text.contains("wok") || text.contains("стир-фрай")
                || text.contains("stir-fry") || text.contains("stir fry")
                || text.contains("быстро обжар") || text.contains("помешивая")
        }
        CookingTechnique::Braise => {
            text.contains("тушит") || text.contains("тушен") || text.contains("braise")
                || text.contains("на медленном") || text.contains("simmer")
        }
        CookingTechnique::RawAssembly => {
            text.contains("собер") || text.contains("выложите") || text.contains("нарежьте")
                || text.contains("assemble") || text.contains("arrange") || text.contains("toss")
                || text.contains("mix") || text.contains("смешайте")
        }
        CookingTechnique::Blend => {
            text.contains("блендер") || text.contains("blend") || text.contains("взбейте")
                || text.contains("пюрир") || text.contains("измельч")
        }
        CookingTechnique::Simmer => {
            text.contains("томит") || text.contains("на медленном") || text.contains("simmer")
                || text.contains("тушит") || text.contains("помешива")
        }
    }
}
