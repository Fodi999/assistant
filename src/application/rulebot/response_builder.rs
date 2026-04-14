//! Response Builder — "как собрать ответ"
//!
//! Takes structured data from handlers and assembles `ChatResponse`
//! using templates for text generation.
//!
//! ```text
//! chat_engine  →  what to answer  (intent routing)
//! response_builder →  how to build  (card assembly + suggestions)
//! response_templates →  how it sounds  (localized text)
//! ```

use super::intent_router::{ChatLang, Intent};
use super::chat_response::{Card, ChatResponse, ConversionCard, NutritionCard, ProductCard, RecipeCard, RecipeIngredientRow, Suggestion};
use super::response_templates as tpl;
use super::meal_builder::MealCombo;
use crate::infrastructure::ingredient_cache::IngredientData;

// ── Health goal (re-exported for templates) ──────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthGoal {
    HighProtein,
    LowCalorie,
    Balanced,
}

use super::goal_modifier::HealthModifier;

impl HealthGoal {
    /// Map expanded HealthModifier → core HealthGoal (3 variants).
    ///
    /// ```text
    /// HighProtein  → HighProtein
    /// LowCalorie   → LowCalorie
    /// LowCarb      → LowCalorie  (keto/low-carb is calorie-aware)
    /// HighFiber    → LowCalorie  (fiber = diet-adjacent)
    /// ComfortFood  → Balanced    (hearty = no restriction)
    /// Quick        → Balanced    (speed ≠ nutrition goal)
    /// Budget       → Balanced    (price ≠ nutrition goal)
    /// None         → fallback on input text
    /// ```
    pub fn from_modifier(modifier: HealthModifier, input: &str) -> Self {
        match modifier {
            HealthModifier::HighProtein => Self::HighProtein,
            HealthModifier::LowCalorie  => Self::LowCalorie,
            HealthModifier::LowCarb     => Self::LowCalorie,
            HealthModifier::HighFiber   => Self::LowCalorie,
            HealthModifier::ComfortFood => Self::Balanced,
            HealthModifier::Quick       => Self::Balanced,
            HealthModifier::Budget      => Self::Balanced,
            HealthModifier::None => {
                let t = input.to_lowercase();
                if t.contains("белок") || t.contains("протеин") || t.contains("мышц")
                    || t.contains("protein") || t.contains("muscle") || t.contains("białk")
                {
                    Self::HighProtein
                } else if t.contains("похуд") || t.contains("диет") || t.contains("diet")
                    || t.contains("lose weight") || t.contains("slim") || t.contains("сушк")
                    || t.contains("лёгк") || t.contains("легк") || t.contains("light")
                    || t.contains("кето") || t.contains("keto") || t.contains("low carb")
                    || t.contains("клетчатк") || t.contains("fiber")
                {
                    Self::LowCalorie
                } else {
                    Self::Balanced
                }
            }
        }
    }
}

// ── Builders ─────────────────────────────────────────────────────────────────

/// Realistic portion size (grams) based on product type.
/// Used in meal plan to avoid absurd "200g bacon" or "200g cheese" calculations.
pub fn portion_grams(p: &IngredientData) -> f32 {
    match p.product_type.as_str() {
        // Proteins: standard serving 150g
        "meat" | "poultry" | "fish" | "seafood" => 150.0,
        // Eggs: ~2 eggs = 120g
        "eggs" => 120.0,
        // Dairy: cheese 50g, cottage cheese 150g, milk 200ml
        "dairy" => {
            if p.calories_per_100g > 250.0 { 50.0 }   // hard cheese, butter
            else if p.calories_per_100g > 150.0 { 100.0 } // cottage cheese, yogurt
            else { 200.0 }                              // milk, kefir
        }
        // Grains & legumes: dry weight ~80g (cooked ~200g)
        "grain" | "legume" => 80.0,
        // Vegetables: generous 200g
        "vegetable" | "leafy_green" => 200.0,
        // Fruits: 150g
        "fruit" | "berry" => 150.0,
        // Nuts & seeds: small 30g handful
        "nut" | "seed" => 30.0,
        // Default
        _ => 150.0,
    }
}

pub fn build_greeting(lang: ChatLang) -> ChatResponse {
    ChatResponse::text_only(tpl::greeting(lang), Intent::Greeting, lang, 0)
}

pub fn build_unknown(lang: ChatLang) -> ChatResponse {
    ChatResponse::text_only(tpl::unknown(lang), Intent::Unknown, lang, 0)
}

/// Build a nutrition follow-up response (used when context resolves a product).
pub fn build_followup_nutrition(p: &IngredientData, lang: ChatLang) -> ChatResponse {
    let name = p.name(lang.code()).to_string();
    let text = tpl::nutrition_text(&name, p, lang);
    ChatResponse::with_card(
        text,
        Card::Nutrition(NutritionCard {
            name,
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
        }),
        Intent::NutritionInfo,
        lang,
        0,
    )
}

/// Build a healthy-product response with multiple cards, suggestions, chef tip.
pub fn build_healthy_response(
    products: &[(IngredientData, &'static str, String)],
    lang: ChatLang,
    goal: HealthGoal,
) -> ChatResponse {
    if products.is_empty() {
        return ChatResponse::text_only(tpl::healthy_fallback(lang), Intent::HealthyProduct, lang, 0);
    }

    // Build cards
    let cards: Vec<Card> = products.iter().map(|(p, reason_tag, _)| {
        let name = p.name(lang.code()).to_string();
        let hl = tpl::highlight(p, lang, goal);
        Card::Product(ProductCard {
            slug: p.slug.clone(),
            name,
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
            highlight: Some(hl),
            reason_tag: Some(reason_tag),
        })
    }).collect();

    let (top_p, _, _) = &products[0];
    let top_name = top_p.name(lang.code()).to_string();
    let text = tpl::healthy_text(&top_name, top_p, lang, goal);
    let reason = tpl::macro_summary(top_p, lang, goal, products.len());

    let mut resp = ChatResponse::with_cards(
        text, cards, Intent::HealthyProduct, vec![], reason, lang, 0,
    );

    resp.suggestions = build_healthy_suggestions(lang, goal, &top_name);
    resp.chef_tip = Some(tpl::chef_tip(top_p, lang, goal));
    resp
}

/// Build a response for a SPECIFIC product mentioned in a healthy-intent query.
/// "курица полезная" → 1 card for chicken with health analysis.
pub fn build_specific_healthy_product(
    p: &IngredientData,
    lang: ChatLang,
    goal: HealthGoal,
) -> ChatResponse {
    let name = p.name(lang.code()).to_string();
    let hl = tpl::highlight(p, lang, goal);
    let text = tpl::healthy_text(&name, p, lang, goal);
    let reason = tpl::macro_summary(p, lang, goal, 1);

    let card = Card::Product(ProductCard {
        slug: p.slug.clone(),
        name: name.clone(),
        calories_per_100g: p.calories_per_100g,
        protein_per_100g: p.protein_per_100g,
        fat_per_100g: p.fat_per_100g,
        carbs_per_100g: p.carbs_per_100g,
        image_url: p.image_url.clone(),
        highlight: Some(hl),
        reason_tag: Some("specific"),
    });

    let mut resp = ChatResponse::with_cards(
        text, vec![card], Intent::HealthyProduct, vec![], reason, lang, 0,
    );

    resp.suggestions = build_specific_suggestions(lang, goal, &name, &p.slug);
    resp.chef_tip = Some(tpl::chef_tip(p, lang, goal));
    resp
}

/// Build a response when the user asks about a product they've ALREADY seen.
/// Instead of repeating the same card, show comparison with alternatives.
/// "если есть курицу можно похудеть" (after already seeing chicken) →
///   "Курица — хороший выбор ✅ А вот что ещё поможет: [alt1] [alt2]"
pub fn build_already_seen_product(
    p: &IngredientData,
    alternatives: &[(IngredientData, &'static str, String)],
    lang: ChatLang,
    goal: HealthGoal,
) -> ChatResponse {
    let name = p.name(lang.code()).to_string();
    let text = tpl::already_seen_text(&name, p, alternatives, lang, goal);

    // Build cards: ALTERNATIVES only (not the product itself — already shown)
    let mut cards: Vec<Card> = alternatives.iter().map(|(alt, reason_tag, _)| {
        let alt_name = alt.name(lang.code()).to_string();
        let hl = tpl::highlight(alt, lang, goal);
        Card::Product(ProductCard {
            slug: alt.slug.clone(),
            name: alt_name,
            calories_per_100g: alt.calories_per_100g,
            protein_per_100g: alt.protein_per_100g,
            fat_per_100g: alt.fat_per_100g,
            carbs_per_100g: alt.carbs_per_100g,
            image_url: alt.image_url.clone(),
            highlight: Some(hl),
            reason_tag: Some(reason_tag),
        })
    }).collect();

    // If no alternatives found, still show the original product card
    if cards.is_empty() {
        let hl = tpl::highlight(p, lang, goal);
        cards.push(Card::Product(ProductCard {
            slug: p.slug.clone(),
            name: name.clone(),
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
            highlight: Some(hl),
            reason_tag: Some("specific"),
        }));
    }

    let reason = tpl::already_seen_reason(&name, alternatives, lang, goal);

    let mut resp = ChatResponse::with_cards(
        text, cards, Intent::HealthyProduct, vec![], reason, lang, 0,
    );

    resp.suggestions = build_already_seen_suggestions(lang, goal, &name, &p.slug);
    resp.chef_tip = Some(tpl::chef_tip_alternative(p, lang, goal));
    resp
}

/// Build a conversion response.
pub fn build_conversion(value: f64, from: String, to: String, result: f64, supported: bool, lang: ChatLang) -> ChatResponse {
    let text = tpl::conversion_text(value, &from, result, &to, supported, lang);
    ChatResponse::with_card(
        text,
        Card::Conversion(ConversionCard { value, from, to, result, supported }),
        Intent::Conversion,
        lang,
        0,
    )
}

pub fn build_conversion_hint(lang: ChatLang) -> ChatResponse {
    ChatResponse::text_only(tpl::conversion_hint(lang), Intent::Conversion, lang, 0)
}

/// Build a density-based conversion response that mentions the product.
pub fn build_conversion_with_product(
    value: f64,
    from: String,
    to: String,
    result: f64,
    product_name: &str,
    lang: ChatLang,
) -> ChatResponse {
    let text = tpl::conversion_with_product_text(value, &from, result, &to, product_name, lang);
    ChatResponse::with_card(
        text,
        Card::Conversion(ConversionCard { value, from, to, result, supported: true }),
        Intent::Conversion,
        lang,
        0,
    )
}

/// Build a "for which product?" response when g↔ml conversion needs density.
pub fn build_conversion_ask_product(
    value: f64,
    from: String,
    to: String,
    lang: ChatLang,
) -> ChatResponse {
    let text = tpl::conversion_ask_product_text(value, &from, &to, lang);
    let mut resp = ChatResponse::text_only(text, Intent::Conversion, lang, 0);
    // Add quick-pick suggestions
    resp.suggestions = match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "💧 Вода".into(), query: format!("{} {} воды в {}", value, from, to), emoji: Some("💧") },
            Suggestion { label: "🌾 Мука".into(), query: format!("{} {} муки в {}", value, from, to), emoji: Some("🌾") },
            Suggestion { label: "🍚 Рис".into(), query: format!("{} {} риса в {}", value, from, to), emoji: Some("🍚") },
            Suggestion { label: "🍯 Мёд".into(), query: format!("{} {} мёда в {}", value, from, to), emoji: Some("🍯") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "💧 Water".into(), query: format!("{} {} water in {}", value, from, to), emoji: Some("💧") },
            Suggestion { label: "🌾 Flour".into(), query: format!("{} {} flour in {}", value, from, to), emoji: Some("🌾") },
            Suggestion { label: "🍚 Rice".into(), query: format!("{} {} rice in {}", value, from, to), emoji: Some("🍚") },
            Suggestion { label: "🍯 Honey".into(), query: format!("{} {} honey in {}", value, from, to), emoji: Some("🍯") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "💧 Woda".into(), query: format!("{} {} wody na {}", value, from, to), emoji: Some("💧") },
            Suggestion { label: "🌾 Mąka".into(), query: format!("{} {} mąki na {}", value, from, to), emoji: Some("🌾") },
            Suggestion { label: "🍚 Ryż".into(), query: format!("{} {} ryżu na {}", value, from, to), emoji: Some("🍚") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "💧 Вода".into(), query: format!("{} {} води в {}", value, from, to), emoji: Some("💧") },
            Suggestion { label: "🌾 Борошно".into(), query: format!("{} {} борошна в {}", value, from, to), emoji: Some("🌾") },
            Suggestion { label: "🍚 Рис".into(), query: format!("{} {} рису в {}", value, from, to), emoji: Some("🍚") },
        ],
    };
    resp
}

/// Build a nutrition info response.
pub fn build_nutrition(p: &IngredientData, lang: ChatLang) -> ChatResponse {
    let name = p.name(lang.code()).to_string();
    let text = tpl::nutrition_text(&name, p, lang);
    ChatResponse::with_card(
        text,
        Card::Nutrition(NutritionCard {
            name,
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
        }),
        Intent::NutritionInfo,
        lang,
        0,
    )
}

pub fn build_nutrition_hint(lang: ChatLang) -> ChatResponse {
    ChatResponse::text_only(tpl::nutrition_hint(lang), Intent::NutritionInfo, lang, 0)
}

/// Build a seasonality response.
pub fn build_seasonality(product: Option<&str>, lang: ChatLang) -> ChatResponse {
    let text = match product {
        Some(p) => tpl::season_text(p, lang),
        None => tpl::season_hint(lang).to_string(),
    };
    ChatResponse::text_only(text, Intent::Seasonality, lang, 0)
}

/// Build a recipe hint response (fallback when Gemini fails).
pub fn build_recipe(dish: Option<&str>, lang: ChatLang) -> ChatResponse {
    let text = match dish {
        Some(name) => tpl::recipe_hint(name, lang),
        None => tpl::recipe_generic(lang).to_string(),
    };
    ChatResponse::text_only(text, Intent::RecipeHelp, lang, 0)
}

/// Build a full recipe/tech-card response from a resolved TechCard.
pub fn build_recipe_card(
    card: &super::recipe_engine::TechCard,
    text: String,
    lang: ChatLang,
) -> ChatResponse {
    let recipe_card = RecipeCard {
        dish_name: card.dish_name.clone(),
        dish_name_local: card.dish_name_local.clone(),
        display_name: card.display_name.clone(),
        dish_type: Some(card.dish_type.clone()),
        servings: card.servings,
        ingredients: card.ingredients.iter().map(|ing| {
            RecipeIngredientRow {
                name: ing.product.as_ref()
                    .map(|p| p.name(lang.code()).to_string())
                    .unwrap_or_else(|| ing.slug_hint.clone()),
                slug: ing.resolved_slug.clone(),
                state: ing.state.clone(),
                role: ing.role.clone(),
                gross_g: ing.gross_g,
                net_g: ing.cooked_net_g,
                kcal: ing.kcal,
                protein_g: ing.protein_g,
                fat_g: ing.fat_g,
                carbs_g: ing.carbs_g,
            }
        }).collect(),
        steps: card.steps.iter().map(|s| {
            super::chat_response::RecipeStepRow {
                step: s.step,
                text: s.text.clone(),
                time_min: s.time_min,
            }
        }).collect(),
        total_output_g: card.total_output_g,
        total_kcal: card.total_kcal,
        total_protein: card.total_protein,
        total_fat: card.total_fat,
        total_carbs: card.total_carbs,
        per_serving_kcal: card.per_serving_kcal,
        per_serving_protein: card.per_serving_protein,
        per_serving_fat: card.per_serving_fat,
        per_serving_carbs: card.per_serving_carbs,
        unresolved: card.unresolved.clone(),
    };

    let mut resp = ChatResponse::with_card(
        text,
        Card::Recipe(recipe_card),
        Intent::RecipeHelp,
        lang,
        0,
    );

    // Suggest follow-ups
    let suggestions = match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "🔄 Другое блюдо".into(), query: "предложи рецепт".into(), emoji: None },
            Suggestion { label: "🥗 План питания".into(), query: "составь план питания".into(), emoji: None },
        ],
        ChatLang::En => vec![
            Suggestion { label: "🔄 Another dish".into(), query: "suggest a recipe".into(), emoji: None },
            Suggestion { label: "🥗 Meal plan".into(), query: "build my meal plan".into(), emoji: None },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "🔄 Inne danie".into(), query: "zaproponuj przepis".into(), emoji: None },
            Suggestion { label: "🥗 Plan posiłków".into(), query: "ułóż plan posiłków".into(), emoji: None },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "🔄 Інша страва".into(), query: "запропонуй рецепт".into(), emoji: None },
            Suggestion { label: "🥗 План харчування".into(), query: "склади план харчування".into(), emoji: None },
        ],
    };
    resp.suggestions = suggestions;
    resp
}

/// Build a meal idea response (with product card).
pub fn build_meal_idea(
    meal_name: &str,
    description: &str,
    slug: &str,
    p: &IngredientData,
    lang: ChatLang,
    goal: HealthGoal,
) -> ChatResponse {
    let ingredient_name = p.name(lang.code()).to_string();
    let text = tpl::meal_idea_with_product(meal_name, description, &ingredient_name, p.calories_per_100g as i32, lang);
    let mut resp = ChatResponse::with_card(
        text,
        Card::Product(ProductCard {
            slug: p.slug.clone(),
            name: ingredient_name.clone(),
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
            highlight: None,
            reason_tag: None,
        }),
        Intent::MealIdea,
        lang,
        0,
    );
    resp.suggestions = build_meal_suggestions(lang, slug);
    resp.chef_tip = Some(tpl::chef_tip(p, lang, goal));
    resp
}

pub fn build_meal_idea_text_only(meal_name: &str, description: &str, lang: ChatLang) -> ChatResponse {
    let text = tpl::meal_idea_text_only(meal_name, description, lang);
    ChatResponse::text_only(text, Intent::MealIdea, lang, 0)
}

/// Build a dynamic meal combo response with multiple product cards.
pub fn build_meal_combo(combo: &MealCombo, lang: ChatLang, goal: HealthGoal) -> ChatResponse {
    let text = tpl::meal_combo_text(combo, lang, goal);

    // Build product cards for each ingredient in the combo
    let mut cards: Vec<Card> = Vec::new();
    let role_labels = match lang {
        ChatLang::Ru => ["🥩 Белок", "🥦 Гарнир", "🍚 База"],
        ChatLang::En => ["🥩 Protein", "🥦 Side", "🍚 Base"],
        ChatLang::Pl => ["🥩 Białko", "🥦 Dodatek", "🍚 Baza"],
        ChatLang::Uk => ["🥩 Білок", "🥦 Гарнір", "🍚 База"],
    };

    // Protein card
    let pname = combo.protein.name(lang.code()).to_string();
    cards.push(Card::Product(ProductCard {
        slug: combo.protein.slug.clone(),
        name: pname,
        calories_per_100g: combo.protein.calories_per_100g,
        protein_per_100g: combo.protein.protein_per_100g,
        fat_per_100g: combo.protein.fat_per_100g,
        carbs_per_100g: combo.protein.carbs_per_100g,
        image_url: combo.protein.image_url.clone(),
        highlight: Some(format!("{} · {}g", role_labels[0], combo.protein_g as u32)),
        reason_tag: Some("meal-protein"),
    }));

    // Side card
    let sname = combo.side.name(lang.code()).to_string();
    cards.push(Card::Product(ProductCard {
        slug: combo.side.slug.clone(),
        name: sname,
        calories_per_100g: combo.side.calories_per_100g,
        protein_per_100g: combo.side.protein_per_100g,
        fat_per_100g: combo.side.fat_per_100g,
        carbs_per_100g: combo.side.carbs_per_100g,
        image_url: combo.side.image_url.clone(),
        highlight: Some(format!("{} · {}g", role_labels[1], combo.side_g as u32)),
        reason_tag: Some("meal-side"),
    }));

    // Base card (optional)
    if let Some(ref base) = combo.base {
        let bname = base.name(lang.code()).to_string();
        cards.push(Card::Product(ProductCard {
            slug: base.slug.clone(),
            name: bname,
            calories_per_100g: base.calories_per_100g,
            protein_per_100g: base.protein_per_100g,
            fat_per_100g: base.fat_per_100g,
            carbs_per_100g: base.carbs_per_100g,
            image_url: base.image_url.clone(),
            highlight: Some(format!("{} · {}g", role_labels[2], combo.base_g as u32)),
            reason_tag: Some("meal-base"),
        }));
    }

    let reason = match lang {
        ChatLang::Ru => format!("{}ккал · {:.0}г белка", combo.total_kcal, combo.total_protein),
        ChatLang::En => format!("{}kcal · {:.0}g protein", combo.total_kcal, combo.total_protein),
        ChatLang::Pl => format!("{}kcal · {:.0}g białka", combo.total_kcal, combo.total_protein),
        ChatLang::Uk => format!("{}ккал · {:.0}г білка", combo.total_kcal, combo.total_protein),
    };

    let mut resp = ChatResponse::with_cards(
        text, cards, Intent::MealIdea, vec![], reason, lang, 0,
    );

    // Suggestions: try another combo, recipe with protein, details
    resp.suggestions = match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "🔄 Другой вариант".into(), query: "другое блюдо".into(), emoji: Some("🔄") },
            Suggestion { label: format!("🍳 Рецепт с {}", combo.protein.name_ru), query: format!("рецепт с {}", combo.protein.slug), emoji: Some("🍳") },
            Suggestion { label: "📋 План на день".into(), query: "план питания на день".into(), emoji: Some("📋") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "🔄 Another combo".into(), query: "another meal idea".into(), emoji: Some("🔄") },
            Suggestion { label: format!("🍳 Recipe with {}", combo.protein.name_en), query: format!("recipe with {}", combo.protein.slug), emoji: Some("🍳") },
            Suggestion { label: "📋 Full day plan".into(), query: "meal plan for the day".into(), emoji: Some("📋") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "🔄 Inny wariant".into(), query: "inny pomysł na posiłek".into(), emoji: Some("🔄") },
            Suggestion { label: format!("🍳 Przepis z {}", combo.protein.name_pl), query: format!("przepis z {}", combo.protein.slug), emoji: Some("🍳") },
            Suggestion { label: "📋 Plan na dzień".into(), query: "plan posiłków na dzień".into(), emoji: Some("📋") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "🔄 Інший варіант".into(), query: "інша страва".into(), emoji: Some("🔄") },
            Suggestion { label: format!("🍳 Рецепт з {}", combo.protein.name_uk), query: format!("рецепт з {}", combo.protein.slug), emoji: Some("🍳") },
            Suggestion { label: "📋 План на день".into(), query: "план харчування на день".into(), emoji: Some("📋") },
        ],
    };

    resp
}

/// Build a full-day meal plan (breakfast + lunch + dinner) with product cards.
pub fn build_meal_plan(
    products: &[(IngredientData, &'static str, String)],
    lang: ChatLang,
    goal: HealthGoal,
) -> ChatResponse {
    if products.is_empty() {
        return ChatResponse::text_only(tpl::healthy_fallback(lang), Intent::MealIdea, lang, 0);
    }

    let meal_labels: &[&str] = match lang {
        ChatLang::Ru => &["🌅 Завтрак", "☀️ Обед", "🌙 Ужин"],
        ChatLang::En => &["🌅 Breakfast", "☀️ Lunch", "🌙 Dinner"],
        ChatLang::Pl => &["🌅 Śniadanie", "☀️ Obiad", "🌙 Kolacja"],
        ChatLang::Uk => &["🌅 Сніданок", "☀️ Обід", "🌙 Вечеря"],
    };

    let cards: Vec<Card> = products.iter().map(|(p, reason_tag, _)| {
        let name = p.name(lang.code()).to_string();
        let hl = tpl::highlight(p, lang, goal);
        Card::Product(ProductCard {
            slug: p.slug.clone(),
            name,
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
            highlight: Some(hl),
            reason_tag: Some(reason_tag),
        })
    }).collect();

    let text = tpl::meal_plan_text(products, meal_labels, lang, goal);

    // Use realistic portions per product type (not blanket 200g)
    let total_cal: i32 = products.iter().map(|(p, _, _)| {
        let g = portion_grams(p);
        (p.calories_per_100g * g / 100.0) as i32
    }).sum();
    let total_pro: f32 = products.iter().map(|(p, _, _)| {
        let g = portion_grams(p);
        p.protein_per_100g * g / 100.0
    }).sum();
    let reason = match lang {
        ChatLang::Ru => format!("~{} ккал · {:.0}г белка за день", total_cal, total_pro),
        ChatLang::En => format!("~{} kcal · {:.0}g protein per day", total_cal, total_pro),
        ChatLang::Pl => format!("~{} kcal · {:.0}g białka na dzień", total_cal, total_pro),
        ChatLang::Uk => format!("~{} ккал · {:.0}г білка за день", total_cal, total_pro),
    };

    let mut resp = ChatResponse::with_cards(
        text, cards, Intent::MealIdea, vec![], reason, lang, 0,
    );

    resp.suggestions = match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "Другой вариант плана".into(), query: "другой план питания".into(), emoji: Some("🔄") },
            Suggestion { label: "Подробнее о завтраке".into(), query: format!("что такое {}", products.first().map(|(p,_,_)| p.slug.as_str()).unwrap_or("eggs")), emoji: Some("🔍") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "Another plan".into(), query: "another meal plan".into(), emoji: Some("🔄") },
            Suggestion { label: "Breakfast details".into(), query: format!("what is {}", products.first().map(|(p,_,_)| p.slug.as_str()).unwrap_or("eggs")), emoji: Some("🔍") },
        ],
        _ => vec![
            Suggestion { label: "Inny plan".into(), query: "inny plan posiłków".into(), emoji: Some("🔄") },
        ],
    };

    resp
}

/// Build a product info response (from cache hit).
pub fn build_product_info(p: &IngredientData, lang: ChatLang) -> ChatResponse {
    let name = p.name(lang.code()).to_string();
    let text = tpl::product_info_text(&name, p, lang);
    ChatResponse::with_card(
        text,
        Card::Product(ProductCard {
            slug: p.slug.clone(),
            name,
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
            highlight: None,
            reason_tag: None,
        }),
        Intent::ProductInfo,
        lang,
        0,
    )
}

pub fn build_product_info_llm(text: String, lang: ChatLang) -> ChatResponse {
    ChatResponse::text_only(text, Intent::ProductInfo, lang, 0)
}

pub fn build_product_not_found(lang: ChatLang) -> ChatResponse {
    ChatResponse::text_only(tpl::product_not_found(lang), Intent::ProductInfo, lang, 0)
}

// ── Suggestion builders ──────────────────────────────────────────────────────

fn build_healthy_suggestions(lang: ChatLang, goal: HealthGoal, top_name: &str) -> Vec<Suggestion> {
    let plan_label: String = match (lang, goal) {
        (ChatLang::Ru, HealthGoal::LowCalorie)  => "~1600 ккал · 100г белка → Собрать день".into(),
        (ChatLang::Ru, HealthGoal::HighProtein)  => "~2200 ккал · 160г белка → Собрать день".into(),
        (ChatLang::Ru, HealthGoal::Balanced)     => "~1800 ккал · 120г белка → Собрать день".into(),
        (ChatLang::En, HealthGoal::LowCalorie)   => "~1600 kcal · 100g protein → Build my day".into(),
        (ChatLang::En, HealthGoal::HighProtein)   => "~2200 kcal · 160g protein → Build my day".into(),
        (ChatLang::En, HealthGoal::Balanced)      => "~1800 kcal · 120g protein → Build my day".into(),
        (ChatLang::Pl, HealthGoal::LowCalorie)   => "~1600 kcal · 100g białka → Ułóż dzień".into(),
        (ChatLang::Pl, HealthGoal::HighProtein)   => "~2200 kcal · 160g białka → Ułóż dzień".into(),
        (ChatLang::Pl, HealthGoal::Balanced)      => "~1800 kcal · 120g białka → Ułóż dzień".into(),
        (ChatLang::Uk, HealthGoal::LowCalorie)   => "~1600 ккал · 100г білка → Скласти день".into(),
        (ChatLang::Uk, HealthGoal::HighProtein)   => "~2200 ккал · 160г білка → Скласти день".into(),
        (ChatLang::Uk, HealthGoal::Balanced)      => "~1800 ккал · 120г білка → Скласти день".into(),
    };

    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: format!("🍲 Лёгкий рецепт с {}", top_name), query: format!("приготовь лёгкое блюдо с {}", top_name), emoji: Some("🍲") },
            Suggestion { label: plan_label, query: "план питания на день".into(), emoji: Some("📋") },
            Suggestion { label: "Ещё варианты".into(), query: match goal {
                HealthGoal::HighProtein => "ещё высокобелковые продукты".into(),
                HealthGoal::LowCalorie  => "ещё низкокалорийные продукты".into(),
                HealthGoal::Balanced    => "ещё полезные продукты".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::En => vec![
            Suggestion { label: format!("🍲 Light recipe with {}", top_name), query: format!("cook a light dish with {}", top_name), emoji: Some("🍲") },
            Suggestion { label: plan_label, query: "meal plan for the day".into(), emoji: Some("📋") },
            Suggestion { label: "More options".into(), query: match goal {
                HealthGoal::HighProtein => "more high protein foods".into(),
                HealthGoal::LowCalorie  => "more low calorie foods".into(),
                HealthGoal::Balanced    => "more healthy food ideas".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: format!("🍲 Lekki przepis z {}", top_name), query: format!("ugotuj lekkie danie z {}", top_name), emoji: Some("🍲") },
            Suggestion { label: plan_label, query: "plan posiłków na dzień".into(), emoji: Some("📋") },
            Suggestion { label: "Więcej opcji".into(), query: match goal {
                HealthGoal::HighProtein => "więcej produktów wysokobiałkowych".into(),
                HealthGoal::LowCalorie  => "więcej niskokalorycznych produktów".into(),
                HealthGoal::Balanced    => "więcej zdrowych produktów".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: format!("🍲 Легкий рецепт з {}", top_name), query: format!("приготуй легку страву з {}", top_name), emoji: Some("🍲") },
            Suggestion { label: plan_label, query: "план харчування на день".into(), emoji: Some("📋") },
            Suggestion { label: "Ще варіанти".into(), query: match goal {
                HealthGoal::HighProtein => "ще високобілкові продукти".into(),
                HealthGoal::LowCalorie  => "ще низькокалорійні продукти".into(),
                HealthGoal::Balanced    => "ще корисні продукти".into(),
            }, emoji: Some("🔄") },
        ],
    }
}

fn build_meal_suggestions(lang: ChatLang, slug: &str) -> Vec<Suggestion> {
    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: "Покажи рецепт".into(), query: format!("рецепт с {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Другая идея".into(), query: "что ещё приготовить".into(), emoji: Some("🔄") },
            Suggestion { label: "Калории продукта".into(), query: format!("калории {}", slug), emoji: Some("📊") },
        ],
        ChatLang::En => vec![
            Suggestion { label: "Show recipe".into(), query: format!("recipe with {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Another idea".into(), query: "another meal idea".into(), emoji: Some("🔄") },
            Suggestion { label: "Product calories".into(), query: format!("calories in {}", slug), emoji: Some("📊") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: "Pokaż przepis".into(), query: format!("przepis z {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Inny pomysł".into(), query: "inny pomysł na posiłek".into(), emoji: Some("🔄") },
            Suggestion { label: "Kalorie produktu".into(), query: format!("kalorie {}", slug), emoji: Some("📊") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: "Покажи рецепт".into(), query: format!("рецепт з {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Інша ідея".into(), query: "що ще приготувати".into(), emoji: Some("🔄") },
            Suggestion { label: "Калорії продукту".into(), query: format!("калорії {}", slug), emoji: Some("📊") },
        ],
    }
}

/// Suggestions for a specific product mentioned in a healthy query.
/// E.g. "курица полезная" → recipes, nutrition deep-dive, alternatives.
fn build_specific_suggestions(lang: ChatLang, goal: HealthGoal, name: &str, slug: &str) -> Vec<Suggestion> {
    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: format!("Рецепты с {}", name), query: format!("рецепт с {}", slug), emoji: Some("📖") },
            Suggestion { label: format!("Подробнее о {}", name), query: format!("что такое {}", slug), emoji: Some("🔍") },
            Suggestion { label: "Похожие продукты".into(), query: match goal {
                HealthGoal::HighProtein => "ещё высокобелковые продукты".into(),
                HealthGoal::LowCalorie  => "ещё низкокалорийные продукты".into(),
                HealthGoal::Balanced    => "ещё полезные продукты".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::En => vec![
            Suggestion { label: format!("Recipes with {}", name), query: format!("recipe with {}", slug), emoji: Some("📖") },
            Suggestion { label: format!("More about {}", name), query: format!("what is {}", slug), emoji: Some("🔍") },
            Suggestion { label: "Similar products".into(), query: match goal {
                HealthGoal::HighProtein => "more high protein foods".into(),
                HealthGoal::LowCalorie  => "more low calorie foods".into(),
                HealthGoal::Balanced    => "more healthy food ideas".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: format!("Przepisy z {}", name), query: format!("przepis z {}", slug), emoji: Some("📖") },
            Suggestion { label: format!("Więcej o {}", name), query: format!("co to jest {}", slug), emoji: Some("🔍") },
            Suggestion { label: "Podobne produkty".into(), query: match goal {
                HealthGoal::HighProtein => "więcej produktów wysokobiałkowych".into(),
                HealthGoal::LowCalorie  => "więcej niskokalorycznych produktów".into(),
                HealthGoal::Balanced    => "więcej zdrowych produktów".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: format!("Рецепти з {}", name), query: format!("рецепт з {}", slug), emoji: Some("📖") },
            Suggestion { label: format!("Детальніше про {}", name), query: format!("що таке {}", slug), emoji: Some("🔍") },
            Suggestion { label: "Схожі продукти".into(), query: match goal {
                HealthGoal::HighProtein => "ще високобілкові продукти".into(),
                HealthGoal::LowCalorie  => "ще низькокалорійні продукти".into(),
                HealthGoal::Balanced    => "ще корисні продукти".into(),
            }, emoji: Some("🔄") },
        ],
    }
}

/// Suggestions for "already seen" responses — focus on action + exploration.
fn build_already_seen_suggestions(lang: ChatLang, goal: HealthGoal, name: &str, slug: &str) -> Vec<Suggestion> {
    match lang {
        ChatLang::Ru => vec![
            Suggestion { label: format!("Рецепт с {}", name), query: format!("рецепт с {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Собрать план на день".into(), query: "план питания на день".into(), emoji: Some("📋") },
            Suggestion { label: "Другие продукты".into(), query: match goal {
                HealthGoal::HighProtein => "что ещё высокобелкового".into(),
                HealthGoal::LowCalorie  => "что ещё низкокалорийного".into(),
                HealthGoal::Balanced    => "ещё полезные продукты".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::En => vec![
            Suggestion { label: format!("Recipe with {}", name), query: format!("recipe with {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Build meal plan".into(), query: "meal plan for the day".into(), emoji: Some("📋") },
            Suggestion { label: "Other products".into(), query: match goal {
                HealthGoal::HighProtein => "more high protein foods".into(),
                HealthGoal::LowCalorie  => "more low calorie foods".into(),
                HealthGoal::Balanced    => "more healthy foods".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::Pl => vec![
            Suggestion { label: format!("Przepis z {}", name), query: format!("przepis z {}", slug), emoji: Some("🍳") },
            Suggestion { label: "Plan na dzień".into(), query: "plan posiłków na dzień".into(), emoji: Some("📋") },
            Suggestion { label: "Inne produkty".into(), query: match goal {
                HealthGoal::HighProtein => "więcej produktów wysokobiałkowych".into(),
                HealthGoal::LowCalorie  => "więcej niskokalorycznych produktów".into(),
                HealthGoal::Balanced    => "więcej zdrowych produktów".into(),
            }, emoji: Some("🔄") },
        ],
        ChatLang::Uk => vec![
            Suggestion { label: format!("Рецепт з {}", name), query: format!("рецепт з {}", slug), emoji: Some("🍳") },
            Suggestion { label: "План на день".into(), query: "план харчування на день".into(), emoji: Some("📋") },
            Suggestion { label: "Інші продукти".into(), query: match goal {
                HealthGoal::HighProtein => "ще високобілкові продукти".into(),
                HealthGoal::LowCalorie  => "ще низькокалорійні продукти".into(),
                HealthGoal::Balanced    => "ще корисні продукти".into(),
            }, emoji: Some("🔄") },
        ],
    }
}
