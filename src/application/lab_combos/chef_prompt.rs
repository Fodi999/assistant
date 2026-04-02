// ─── ChefPromptBuilder — builds Gemini prompt with DishProfile context ──────
//
// This module constructs the AI prompt that drives recipe generation.
// Key innovation: the prompt receives DETERMINISTIC constraints from DishClassifier,
// not just ingredient names. AI receives:
//   - DishType + required techniques
//   - Forbidden techniques
//   - Expected texture descriptors
//   - Pre-calculated nutrition (not AI-guesswork)
//
// SKELETON APPROACH:
//   AI does NOT invent step structure. We provide a rigid skeleton (step count,
//   types, order). AI only fills in the culinary content of each slot.
//   This eliminates "fry eggs + sauté everything + assemble" garbage.

use super::dish_classifier::{DishProfile, DishType};
use super::nutrition::NutritionTotals;

// ── Ingredient name localizer ───────────────────────────────────────────────
//
// Converts slug-style ingredient IDs ("chicken-eggs", "hard-cheese") into
// human-readable names in the target language. Used for SEO texts so users
// see "яйца куриные, твёрдый сыр" instead of "chicken-eggs, hard-cheese".

pub fn localize_ingredients(ingredients: &[String], locale: &str) -> Vec<String> {
    ingredients.iter().map(|slug| localize_one(slug, locale)).collect()
}

fn localize_one(slug: &str, locale: &str) -> String {
    // Normalise: lower-case, treat both "-" and "_" as separators
    let key = slug.to_lowercase().replace('_', "-");

    // Map: (slug_key, locale) → human name
    // Add more rows as your catalog grows.
    let name = match (key.as_str(), locale) {
        // ── Eggs ──────────────────────────────────────────────────────
        ("chicken-eggs" | "eggs" | "egg", "ru") => "яйца куриные",
        ("chicken-eggs" | "eggs" | "egg", "uk") => "курячі яйця",
        ("chicken-eggs" | "eggs" | "egg", "pl") => "jajka kurze",
        ("chicken-eggs" | "eggs" | "egg", _)    => "chicken eggs",
        // ── Cheese ────────────────────────────────────────────────────
        ("hard-cheese" | "cheese", "ru") => "твёрдый сыр",
        ("hard-cheese" | "cheese", "uk") => "твердий сир",
        ("hard-cheese" | "cheese", "pl") => "twardy ser",
        ("hard-cheese" | "cheese", _)   => "hard cheese",
        ("mozzarella", "ru") => "моцарелла",
        ("mozzarella", "uk") => "моцарела",
        ("mozzarella", _)    => "mozzarella",
        // ── Flour / Breadcrumbs ────────────────────────────────────────
        ("wheat-flour" | "flour", "ru") => "пшеничная мука",
        ("wheat-flour" | "flour", "uk") => "пшенична борошно",
        ("wheat-flour" | "flour", "pl") => "mąka pszenna",
        ("wheat-flour" | "flour", _)    => "wheat flour",
        ("breadcrumbs", "ru") => "панировочные сухари",
        ("breadcrumbs", "uk") => "панірувальні сухарі",
        ("breadcrumbs", "pl") => "bułka tarta",
        ("breadcrumbs", _)    => "breadcrumbs",
        // ── Oils / Fats ────────────────────────────────────────────────
        ("olive-oil" | "olive_oil", "ru") => "оливковое масло",
        ("olive-oil" | "olive_oil", "uk") => "оливкова олія",
        ("olive-oil" | "olive_oil", "pl") => "oliwa z oliwek",
        ("olive-oil" | "olive_oil", _)   => "olive oil",
        ("sunflower-oil" | "vegetable-oil", "ru") => "подсолнечное масло",
        ("sunflower-oil" | "vegetable-oil", "uk") => "соняшникова олія",
        ("sunflower-oil" | "vegetable-oil", "pl") => "olej słonecznikowy",
        ("sunflower-oil" | "vegetable-oil", _)   => "sunflower oil",
        ("butter", "ru") => "сливочное масло",
        ("butter", "uk") => "вершкове масло",
        ("butter", "pl") => "masło",
        ("butter", _)    => "butter",
        // ── Proteins ──────────────────────────────────────────────────
        ("chicken-breast" | "chicken", "ru") => "куриная грудка",
        ("chicken-breast" | "chicken", "uk") => "куряча грудка",
        ("chicken-breast" | "chicken", "pl") => "pierś z kurczaka",
        ("chicken-breast" | "chicken", _)   => "chicken breast",
        ("ground-beef" | "minced-beef", "ru") => "говяжий фарш",
        ("ground-beef" | "minced-beef", "uk") => "яловичий фарш",
        ("ground-beef" | "minced-beef", "pl") => "mięso mielone wołowe",
        ("ground-beef" | "minced-beef", _)   => "ground beef",
        ("bacon", "ru") => "бекон",
        ("bacon", "uk") => "бекон",
        ("bacon", "pl") => "boczek",
        ("bacon", _)    => "bacon",
        ("tuna", "ru") => "тунец",
        ("tuna", "uk") => "тунець",
        ("tuna", "pl") => "tuńczyk",
        ("tuna", _)    => "tuna",
        // ── Dairy ─────────────────────────────────────────────────────
        ("milk", "ru") => "молоко",
        ("milk", "uk") => "молоко",
        ("milk", "pl") => "mleko",
        ("milk", _)    => "milk",
        ("cream" | "heavy-cream", "ru") => "жирные сливки",
        ("cream" | "heavy-cream", "uk") => "жирні вершки",
        ("cream" | "heavy-cream", "pl") => "śmietana",
        ("cream" | "heavy-cream", _)   => "heavy cream",
        ("sour-cream", "ru") => "сметана",
        ("sour-cream", "uk") => "сметана",
        ("sour-cream", "pl") => "śmietana kwaśna",
        ("sour-cream", _)   => "sour cream",
        ("cottage-cheese" | "ricotta", "ru") => "творог",
        ("cottage-cheese" | "ricotta", "uk") => "сир кисломолочний",
        ("cottage-cheese" | "ricotta", "pl") => "twaróg",
        ("cottage-cheese" | "ricotta", _)   => "cottage cheese",
        // ── Vegetables ────────────────────────────────────────────────
        ("potato" | "potatoes", "ru") => "картофель",
        ("potato" | "potatoes", "uk") => "картопля",
        ("potato" | "potatoes", "pl") => "ziemniaki",
        ("potato" | "potatoes", _)   => "potatoes",
        ("onion", "ru") => "лук репчатый",
        ("onion", "uk") => "цибуля",
        ("onion", "pl") => "cebula",
        ("onion", _)   => "onion",
        ("garlic", "ru") => "чеснок",
        ("garlic", "uk") => "часник",
        ("garlic", "pl") => "czosnek",
        ("garlic", _)   => "garlic",
        ("carrot" | "carrots", "ru") => "морковь",
        ("carrot" | "carrots", "uk") => "морква",
        ("carrot" | "carrots", "pl") => "marchewka",
        ("carrot" | "carrots", _)   => "carrots",
        ("tomato" | "tomatoes", "ru") => "помидоры",
        ("tomato" | "tomatoes", "uk") => "помідори",
        ("tomato" | "tomatoes", "pl") => "pomidory",
        ("tomato" | "tomatoes", _)   => "tomatoes",
        ("bell-pepper" | "pepper", "ru") => "болгарский перец",
        ("bell-pepper" | "pepper", "uk") => "болгарський перець",
        ("bell-pepper" | "pepper", "pl") => "papryka",
        ("bell-pepper" | "pepper", _)   => "bell pepper",
        ("spinach", "ru") => "шпинат",
        ("spinach", "uk") => "шпинат",
        ("spinach", "pl") => "szpinak",
        ("spinach", _)   => "spinach",
        ("broccoli", "ru") => "брокколи",
        ("broccoli", "uk") => "броколі",
        ("broccoli", "pl") => "brokuły",
        ("broccoli", _)   => "broccoli",
        ("avocado", "ru") => "авокадо",
        ("avocado", "uk") => "авокадо",
        ("avocado", "pl") => "awokado",
        ("avocado", _)   => "avocado",
        ("cucumber" | "cucumbers", "ru") => "огурцы",
        ("cucumber" | "cucumbers", "uk") => "огірки",
        ("cucumber" | "cucumbers", "pl") => "ogórki",
        ("cucumber" | "cucumbers", _)   => "cucumbers",
        // ── Grains / Legumes ──────────────────────────────────────────
        ("rice", "ru") => "рис",
        ("rice", "uk") => "рис",
        ("rice", "pl") => "ryż",
        ("rice", _)   => "rice",
        ("oats" | "oatmeal", "ru") => "овсяные хлопья",
        ("oats" | "oatmeal", "uk") => "вівсяні пластівці",
        ("oats" | "oatmeal", "pl") => "płatki owsiane",
        ("oats" | "oatmeal", _)   => "oatmeal",
        ("chickpeas", "ru") => "нут",
        ("chickpeas", "uk") => "нут",
        ("chickpeas", "pl") => "ciecierzyca",
        ("chickpeas", _)   => "chickpeas",
        ("lentils", "ru") => "чечевица",
        ("lentils", "uk") => "сочевиця",
        ("lentils", "pl") => "soczewica",
        ("lentils", _)   => "lentils",
        // ── Spices / Condiments ────────────────────────────────────────
        ("salt", "ru") => "соль",
        ("salt", "uk") => "сіль",
        ("salt", "pl") => "sól",
        ("salt", _)   => "salt",
        ("black-pepper" | "pepper-black", "ru") => "чёрный перец",
        ("black-pepper" | "pepper-black", "uk") => "чорний перець",
        ("black-pepper" | "pepper-black", "pl") => "czarny pieprz",
        ("black-pepper" | "pepper-black", _)   => "black pepper",
        ("paprika", "ru") => "паприка",
        ("paprika", "uk") => "паприка",
        ("paprika", "pl") => "papryka słodka",
        ("paprika", _)   => "paprika",
        ("soy-sauce", "ru") => "соевый соус",
        ("soy-sauce", "uk") => "соєвий соус",
        ("soy-sauce", "pl") => "sos sojowy",
        ("soy-sauce", _)   => "soy sauce",
        // ── Fallback: prettify the slug ────────────────────────────────
        _ => return slug.replace('-', " "),
    };
    name.to_string()
}

// ── Skeleton builder ────────────────────────────────────────────────────────
//
// CORE IDEA: Instead of asking AI to "generate steps", we provide a rigid skeleton
// with fixed step count, types, and culinary intent for each slot. AI only fills
// the text and time_minutes. This eliminates "fry eggs + sauté everything" garbage.

struct SkeletonStep {
    step_type: &'static str,
    /// Instruction to AI for this specific slot (in English, AI translates output)
    intent: &'static str,
    time_hint: u8,
}

/// Returns the mandatory step skeleton for a dish type.
/// AI MUST fill each slot exactly — no adding/removing steps.
fn build_skeleton(dish_type: &DishType) -> Vec<SkeletonStep> {
    match dish_type {
        DishType::Sticks => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Cut the main solid ingredient (cheese/chicken/etc.) into finger-sized sticks (~8×2 cm). \
                         Set up a 3-bowl batter station: Bowl 1 = flour, Bowl 2 = eggs beaten smooth with a whisk, \
                         Bowl 3 = breadcrumbs mixed with salt and spices.",
                time_hint: 5,
            },
            SkeletonStep {
                step_type: "preparation",
                intent: "Season the cut sticks lightly with salt and pepper. \
                         Mix the egg bowl until fully homogeneous — no streaks.",
                time_hint: 3,
            },
            SkeletonStep {
                step_type: "forming",
                intent: "Coat each stick in sequence: dredge in flour (shake off excess) → \
                         dip fully in beaten egg → roll in breadcrumbs pressing firmly so coating adheres. \
                         Place coated sticks on a plate. Eggs here are BATTER — not a separate dish.",
                time_hint: 5,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Pour 2 cm of oil into a heavy pan and heat to 170–180°C (test with a breadcrumb — \
                         it should sizzle immediately). Fry coated sticks in batches 3–4 min per side \
                         until deep golden-brown crust forms. Do NOT overcrowd the pan.",
                time_hint: 8,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Drain on paper towels 1–2 min. The result: golden crispy crust outside, \
                         soft tender inside, served hot. Plate immediately.",
                time_hint: 2,
            },
        ],
        DishType::Cutlets => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Mince or grind the main protein finely. Add beaten egg as binder, \
                         breadcrumbs for texture, and seasoning. Mix until homogeneous.",
                time_hint: 7,
            },
            SkeletonStep {
                step_type: "forming",
                intent: "Shape mixture into flat round patties ~1.5 cm thick and ~8 cm wide. \
                         Dust each patty in flour and press lightly to seal the crust.",
                time_hint: 5,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Heat 2 tbsp oil in pan over medium heat. Fry patties 4–5 min per side \
                         until golden-brown seared crust forms. Do not press down.",
                time_hint: 10,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Rest 2 min. The result: golden-brown seared crust outside, \
                         juicy tender inside, served hot.",
                time_hint: 2,
            },
        ],
        DishType::Pancakes => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Whisk dry ingredients (flour, salt, sugar/sweetener) in a bowl. \
                         In a separate bowl, beat eggs, then add milk and any wet additions. \
                         Combine wet into dry and whisk until smooth — no lumps.",
                time_hint: 5,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Heat non-stick pan over medium heat with butter or oil. \
                         Pour ~60 ml batter per pancake. Cook 2 min until bubbles appear \
                         and edges set, then flip and cook 1 min on the other side.",
                time_hint: 12,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Stack pancakes on warm plate. The result: golden lightly crispy edges, \
                         soft fluffy inside, served hot. Add toppings if desired.",
                time_hint: 2,
            },
        ],
        DishType::Soup => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Dice all vegetables uniformly for even cooking. \
                         Prepare protein — cut into equal pieces. Measure liquids.",
                time_hint: 8,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Sauté aromatic base (onion, garlic, carrot) in oil 3–5 min until soft. \
                         Add protein and brown 3 min.",
                time_hint: 8,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Add broth or water. Bring to boil over high heat, then reduce to simmer. \
                         Add remaining vegetables and cook 15–20 min until tender.",
                time_hint: 20,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Season to taste. The result: rich aromatic broth, \
                         tender pieces in liquid, served hot in deep bowls.",
                time_hint: 3,
            },
        ],
        DishType::Casserole => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Preheat oven to 180–200°C. Prepare all ingredients — \
                         slice, dice, or cook as needed. Grease baking dish.",
                time_hint: 10,
            },
            SkeletonStep {
                step_type: "preparation",
                intent: "Make the binding mixture: whisk eggs with dairy (cream/milk/sour cream), \
                         season with salt, pepper, and spices.",
                time_hint: 5,
            },
            SkeletonStep {
                step_type: "forming",
                intent: "Layer ingredients evenly in the baking dish. \
                         Pour binding mixture over the top. Cover with grated cheese or breadcrumbs.",
                time_hint: 5,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Bake at 180–200°C for 30–40 min until top is golden and \
                         centre is set (test with a toothpick — should come out clean).",
                time_hint: 35,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Rest 5 min before slicing — casserole holds its shape. \
                         Serve hot, portioned into squares.",
                time_hint: 5,
            },
        ],
        DishType::Omelette => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Crack eggs into bowl. Beat with fork until yolks and whites are fully combined \
                         — about 30 strokes. Season with salt and pepper. \
                         Prepare fillings (chop, grate) as needed.",
                time_hint: 4,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Melt butter in a non-stick pan over medium-low heat until foaming subsides. \
                         Pour in eggs. Cook 2–3 min, gently pulling edges inward with spatula \
                         so uncooked egg flows to the edges.",
                time_hint: 4,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "When surface is barely set (still slightly glossy), \
                         add fillings to one half of the omelette.",
                time_hint: 1,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Fold omelette in half and slide onto plate. \
                         Serve immediately — soft fluffy inside, set outside.",
                time_hint: 1,
            },
        ],
        DishType::StirFry => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Cut ALL ingredients into thin strips or small pieces (max 1 cm) \
                         for rapid high-heat cooking. Mix sauce (soy sauce, oil, spices) in a small bowl. \
                         Have everything at the pan before you start — stir-fry is fast.",
                time_hint: 8,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Heat wok or heavy pan over high heat until a drop of water evaporates instantly. \
                         Add oil, then protein. Stir-fry 2–3 min without resting until \
                         lightly browned. Remove to a bowl.",
                time_hint: 4,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "In the same pan, stir-fry vegetables 1–2 min — they should stay crisp. \
                         Return protein. Pour sauce over everything and toss 30 sec on maximum heat.",
                time_hint: 3,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Serve immediately. The result: colorful vibrant presentation, \
                         crisp varied textures, warm.",
                time_hint: 1,
            },
        ],
        DishType::Salad => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Wash and thoroughly dry all greens and vegetables. \
                         Cut into uniform bite-size pieces. Do NOT cook raw vegetables.",
                time_hint: 5,
            },
            SkeletonStep {
                step_type: "preparation",
                intent: "Make dressing: whisk oil, acid (lemon/vinegar), salt, pepper \
                         and any other seasonings until emulsified.",
                time_hint: 3,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Toss all ingredients with dressing — coat evenly. \
                         Plate and garnish. The result: fresh vibrant colors, \
                         crisp varied textures, cold or room temperature.",
                time_hint: 2,
            },
        ],
        // Generic, Bowl, Pasta, Wrap, Baked, Porridge, Smoothie, etc.
        _ => vec![
            SkeletonStep {
                step_type: "preparation",
                intent: "Prepare all ingredients: clean, cut, measure as needed. \
                         Set up your workspace.",
                time_hint: 7,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Cook main components using the appropriate technique \
                         (specified by allowed techniques). Monitor heat and timing closely.",
                time_hint: 10,
            },
            SkeletonStep {
                step_type: "cooking",
                intent: "Combine all components together. Adjust seasoning.",
                time_hint: 5,
            },
            SkeletonStep {
                step_type: "finishing",
                intent: "Plate and serve at the correct temperature. \
                         Add garnish or sauce if applicable.",
                time_hint: 3,
            },
        ],
    }
}

/// Render the skeleton into the prompt body showing AI exactly what each slot must contain.
fn render_skeleton_prompt(skeleton: &[SkeletonStep], lang: &str, names: &str) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "You are a professional chef. Your task is to FILL IN the cooking steps below.\n\
         DO NOT change the structure, step count, or step types — they are fixed.\n\
         Write all text in {lang}.\n\
         Ingredients available: {names}\n\
         Use realistic grams/quantities in each step text.\n"
    ));

    lines.push(
        "MANDATORY STRUCTURE — fill each slot with realistic culinary text:\n".to_string()
    );

    for (i, slot) in skeleton.iter().enumerate() {
        lines.push(format!(
            "SLOT {} — type=\"{}\" — time_minutes≈{}\n  Intent: {}\n",
            i + 1,
            slot.step_type,
            slot.time_hint,
            slot.intent
        ));
    }

    // Build the exact JSON template AI must fill
    lines.push("\nReturn ONLY this JSON array with all slots filled (no markdown, no wrapper object):".to_string());
    lines.push("[".to_string());
    for (i, slot) in skeleton.iter().enumerate() {
        let comma = if i + 1 < skeleton.len() { "," } else { "" };
        lines.push(format!(
            "  {{\"step\": {}, \"type\": \"{}\", \"text\": \"[FILL IN {lang} text for slot {} here — verb + ingredients + temp/time + result]\", \"time_minutes\": {}}}{comma}",
            i + 1,
            slot.step_type,
            i + 1,
            slot.time_hint,
        ));
    }
    lines.push("]".to_string());
    lines.push(format!("\nRules: Write in {lang}. Keep step types EXACTLY as shown. Return ONLY the JSON array."));

    lines.join("\n")
}

/// Build FOCUSED prompt for cooking steps ONLY.
///
/// SKELETON APPROACH: We provide the fixed step structure (count, types, culinary intent).
/// AI only fills in the text and time_minutes. No invention of structure.
/// This is Call 1 in the split pipeline.
pub fn build_steps_prompt(
    ingredients: &[String],
    locale: &str,
    dish_name: Option<&str>,
    profile: &DishProfile,
) -> String {
    let localized = localize_ingredients(ingredients, locale);
    let names = localized.join(", ");

    let lang = match locale {
        "ru" => "Russian",
        "pl" => "Polish",
        "uk" => "Ukrainian",
        _ => "English",
    };

    let skeleton = build_skeleton(&profile.dish_type);

    let dish_header = if let Some(dn) = dish_name {
        format!(
            "Dish name: \"{dn}\" | Type: {} | Forming required: {} | Texture: {} outside, {} inside, served {}\n\n",
            profile.type_label,
            profile.requires_forming,
            profile.expected_texture.outside,
            profile.expected_texture.inside,
            profile.expected_texture.temperature,
        )
    } else {
        String::new()
    };

    format!("{dish_header}{}", render_skeleton_prompt(&skeleton, lang, &names))
}

/// Build FOCUSED prompt for SEO text fields ONLY.
///
/// This is Call 2 in the split pipeline. Generates title, description, h1, intro, why_it_works.
/// Uses localized ingredient names ("яйца куриные" not "chicken-eggs") for better SEO.
pub fn build_seo_prompt(
    ingredients: &[String],
    locale: &str,
    dish_name: Option<&str>,
    nt: &NutritionTotals,
) -> String {
    // Use localized names for SEO — much better than slug IDs
    let localized = localize_ingredients(ingredients, locale);
    let names = localized.join(", ");

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
