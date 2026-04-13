//! Tool executors — execute the tool chosen by LLM and build ChatResponse.
//!
//! Each executor:
//!   1. Reads data from IngredientCache (or calls unit_converter)
//!   2. Optionally asks LLM to format a human-friendly response
//!   3. Returns ChatResponse with cards + suggestions

use std::sync::Arc;

use crate::infrastructure::IngredientCache;
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::domain::tools::unit_converter as uc;
use crate::application::rulebot::intent_router::ChatLang;
use crate::application::rulebot::chat_response::{
    Card, ChatResponse, ConversionCard, NutritionCard, ProductCard,
};

use super::response_helpers::{
    format_nutrition_response, no_products_text, fallback_text,
    build_followup_suggestions, build_meal_plan_suggestions,
    product_matches_query,
};

// ── Search Products ──────────────────────────────────────────────────────────

pub(crate) async fn execute_search(
    ingredient_cache: &Arc<IngredientCache>,
    llm_adapter: &Arc<LlmAdapter>,
    query: &str,
    goal: &str,
    limit: usize,
    lang: ChatLang,
) -> ChatResponse {
    let all = ingredient_cache.all().await;
    if all.is_empty() {
        return ChatResponse::text_only(
            no_products_text(lang),
            crate::application::rulebot::intent_router::Intent::HealthyProduct,
            lang, 0,
        );
    }

    // Score products by goal
    let mut scored: Vec<(f64, &crate::infrastructure::ingredient_cache::IngredientData)> = all
        .iter()
        .filter(|p| p.calories_per_100g > 0.0 || p.protein_per_100g > 0.0)
        .map(|p| {
            let score = match goal {
                "high_protein" => p.protein_per_100g as f64 * 2.0 - p.calories_per_100g as f64 * 0.3,
                "low_calorie"  => 500.0 - p.calories_per_100g as f64 + p.protein_per_100g as f64 * 0.5,
                _              => p.protein_per_100g as f64 + (300.0 - p.calories_per_100g as f64) * 0.3,
            };
            // Boost if query words appear in product name
            let query_lower = query.to_lowercase();
            let name_bonus = if product_matches_query(p, &query_lower) { 100.0 } else { 0.0 };
            (score + name_bonus, p)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let top: Vec<_> = scored.into_iter().take(limit).collect();
    if top.is_empty() {
        return ChatResponse::text_only(
            no_products_text(lang),
            crate::application::rulebot::intent_router::Intent::HealthyProduct,
            lang, 0,
        );
    }

    let cards: Vec<Card> = top.iter().map(|(_, p)| {
        Card::Product(ProductCard {
            slug: p.slug.clone(),
            name: p.name(lang.code()).to_string(),
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
            highlight: None,
            reason_tag: Some(match goal {
                "high_protein" => "high_protein",
                "low_calorie"  => "low_calorie",
                _              => "balanced",
            }),
        })
    }).collect();

    // Ask LLM to format a nice response text using the found products
    let product_info: Vec<String> = top.iter().map(|(_, p)| {
        format!("{}: {}kcal, {}g protein, {}g fat, {}g carbs per 100g",
            p.name(lang.code()), p.calories_per_100g as i32,
            p.protein_per_100g, p.fat_per_100g, p.carbs_per_100g)
    }).collect();

    let text = format_llm_response(
        llm_adapter,
        query,
        &format!("Found products:\n{}", product_info.join("\n")),
        lang,
    ).await;

    let mut resp = ChatResponse::with_cards(
        text,
        cards,
        crate::application::rulebot::intent_router::Intent::HealthyProduct,
        vec![],
        format!("AI Brain → search_products(goal={})", goal),
        lang,
        0,
    );
    resp.suggestions = build_followup_suggestions(lang, goal);
    resp
}

// ── Get Nutrition ────────────────────────────────────────────────────────────

pub(crate) async fn execute_nutrition(
    ingredient_cache: &Arc<IngredientCache>,
    llm_adapter: &Arc<LlmAdapter>,
    product: &str,
    lang: ChatLang,
) -> ChatResponse {
    let product_lower = product.to_lowercase();

    // Try exact slug match first, then fuzzy name match
    let found = if let Some(p) = ingredient_cache.get(&product_lower).await {
        Some(p)
    } else {
        let all = ingredient_cache.all().await;
        all.into_iter().find(|p| {
            p.name_en.to_lowercase().contains(&product_lower)
                || p.name_ru.to_lowercase().contains(&product_lower)
                || p.name_pl.to_lowercase().contains(&product_lower)
                || p.name_uk.to_lowercase().contains(&product_lower)
                || p.slug.contains(&product_lower)
        })
    };

    if let Some(p) = found {
        let name = p.name(lang.code()).to_string();
        let text = format_nutrition_response(&name, &p, lang);
        return ChatResponse::with_card(
            text,
            Card::Nutrition(NutritionCard {
                name,
                calories_per_100g: p.calories_per_100g,
                protein_per_100g: p.protein_per_100g,
                fat_per_100g: p.fat_per_100g,
                carbs_per_100g: p.carbs_per_100g,
                image_url: p.image_url.clone(),
            }),
            crate::application::rulebot::intent_router::Intent::NutritionInfo,
            lang,
            0,
        );
    }

    // Product not in DB — answer from LLM knowledge
    let text = format_llm_response(
        llm_adapter,
        &format!("nutrition info about {}", product),
        "Product not in our database. Answer from your culinary knowledge.",
        lang,
    ).await;

    ChatResponse::text_only(text, crate::application::rulebot::intent_router::Intent::NutritionInfo, lang, 0)
}

// ── Convert Units ────────────────────────────────────────────────────────────

pub(crate) fn execute_conversion(value: f64, from: &str, to: &str, lang: ChatLang) -> ChatResponse {
    let result_raw = uc::convert_units(value, from, to);
    let supported = result_raw.is_some();
    let result = uc::display_round(result_raw.unwrap_or(0.0));

    let text = if supported {
        format!("{} {} = **{} {}**", value, from, result, to)
    } else {
        match lang {
            ChatLang::Ru => format!("Не могу конвертировать {} → {}. Попробуй: г, мл, ложки, стаканы, унции.", from, to),
            ChatLang::En => format!("Can't convert {} → {}. Try: g, ml, tbsp, cups, oz.", from, to),
            ChatLang::Pl => format!("Nie mogę przeliczyć {} → {}. Spróbuj: g, ml, łyżki, szklanki, uncje.", from, to),
            ChatLang::Uk => format!("Не можу конвертувати {} → {}. Спробуй: г, мл, ложки, склянки, унції.", from, to),
        }
    };

    ChatResponse::with_card(
        text,
        Card::Conversion(ConversionCard { value, from: from.to_string(), to: to.to_string(), result, supported }),
        crate::application::rulebot::intent_router::Intent::Conversion,
        lang,
        0,
    )
}

// ── Meal Plan ────────────────────────────────────────────────────────────────

pub(crate) async fn execute_meal_plan(
    ingredient_cache: &Arc<IngredientCache>,
    llm_adapter: &Arc<LlmAdapter>,
    original_input: &str,
    goal: &str,
    meals: usize,
    lang: ChatLang,
) -> ChatResponse {
    let all = ingredient_cache.all().await;
    let mut scored: Vec<(f64, &crate::infrastructure::ingredient_cache::IngredientData)> = all
        .iter()
        .filter(|p| p.calories_per_100g > 0.0 && p.protein_per_100g > 0.0)
        .map(|p| {
            let score = match goal {
                g if g.contains("protein") || g.contains("muscle") || g.contains("масс") || g.contains("białk") => {
                    p.protein_per_100g as f64 * 2.0 - p.calories_per_100g as f64 * 0.2
                }
                g if g.contains("lose") || g.contains("худ") || g.contains("diet") || g.contains("schud") || g.contains("калор") => {
                    500.0 - p.calories_per_100g as f64 + p.protein_per_100g as f64 * 1.5
                }
                _ => {
                    p.protein_per_100g as f64 + (300.0 - p.calories_per_100g as f64) * 0.3
                }
            };
            (score, p)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let top_products: Vec<_> = scored.into_iter().take(meals * 2).collect();

    // Build product list for LLM
    let products_text: String = top_products.iter().map(|(_, p)| {
        format!("- {} ({}kcal, {}g protein per 100g)",
            p.name(lang.code()), p.calories_per_100g as i32, p.protein_per_100g)
    }).collect::<Vec<_>>().join("\n");

    // Build cards from top picks
    let cards: Vec<Card> = top_products.iter().take(meals).map(|(_, p)| {
        Card::Product(ProductCard {
            slug: p.slug.clone(),
            name: p.name(lang.code()).to_string(),
            calories_per_100g: p.calories_per_100g,
            protein_per_100g: p.protein_per_100g,
            fat_per_100g: p.fat_per_100g,
            carbs_per_100g: p.carbs_per_100g,
            image_url: p.image_url.clone(),
            highlight: None,
            reason_tag: None,
        })
    }).collect();

    // Ask LLM to compose a meal plan using these real products
    let plan_prompt = format!(
        r#"You are ChefOS — an expert culinary assistant.

The user asked: "{input}"
Their goal: {goal}

Create a meal plan using ONLY these products from our database:
{products}

RULES:
- Answer in {lang}
- Be specific: suggest actual dishes with portions (grams)
- Include total calories and protein per meal
- 2-4 sentences per meal, practical and actionable
- Use **bold** for meal names and key numbers
- No generic advice — only specific, data-driven recommendations
- Keep it concise: max 6-8 sentences total"#,
        input = original_input,
        goal = goal,
        products = products_text,
        lang = lang.code(),
    );

    let text = match llm_adapter.groq_raw_request_with_model(&plan_prompt, 600, "gemini-3-flash-preview").await {
        Ok(t) => t,
        Err(_) => match lang {
            ChatLang::Ru => "Не удалось создать план. Попробуй ещё раз.".to_string(),
            ChatLang::En => "Could not create the plan. Please try again.".to_string(),
            ChatLang::Pl => "Nie udało się stworzyć planu. Spróbuj ponownie.".to_string(),
            ChatLang::Uk => "Не вдалося створити план. Спробуй ще раз.".to_string(),
        },
    };

    let mut resp = ChatResponse::with_cards(
        text,
        cards,
        crate::application::rulebot::intent_router::Intent::MealIdea,
        vec![],
        format!("AI Brain → meal_plan(goal={})", goal),
        lang,
        0,
    );
    resp.suggestions = build_meal_plan_suggestions(lang);
    resp
}

// ── LLM Response Formatting (shared) ─────────────────────────────────────────

/// Ask LLM to format a human-friendly response using tool results.
pub(crate) async fn format_llm_response(
    llm_adapter: &Arc<LlmAdapter>,
    user_query: &str,
    tool_result: &str,
    lang: ChatLang,
) -> String {
    let prompt = format!(
        r#"You are ChefOS — an expert culinary assistant.
The user asked: "{}"
Tool result: {}

Write a helpful response in {} language. 2-4 sentences max.
Use **bold** for key data. Be specific and practical. No markdown headers.
If the tool found nothing, answer from your culinary knowledge."#,
        user_query, tool_result, lang.code()
    );

    match llm_adapter.groq_raw_request_with_model(&prompt, 300, "gemini-3-flash-preview").await {
        Ok(text) => text,
        Err(_) => fallback_text(lang).to_string(),
    }
}

/// Fallback when AI Brain decision fails entirely.
pub(crate) async fn fallback_response(
    llm_adapter: &Arc<LlmAdapter>,
    input: &str,
    lang: ChatLang,
) -> ChatResponse {
    let lang_name = match lang {
        ChatLang::Ru => "Russian",
        ChatLang::En => "English",
        ChatLang::Pl => "Polish",
        ChatLang::Uk => "Ukrainian",
    };
    let prompt = format!(
        "You are ChefOS, a culinary assistant. The user asked: \"{}\". \
        Reply in {} in 2-4 sentences. \
        ONLY answer if about food/cooking/nutrition. \
        If unrelated, politely say you only help with culinary topics. \
        Use **bold** for key data. No markdown headers.",
        input, lang_name
    );

    let text = match llm_adapter.groq_raw_request_with_model(&prompt, 300, "gemini-3-flash-preview").await {
        Ok(t) => t,
        Err(_) => fallback_text(lang).to_string(),
    };

    ChatResponse::text_only(text, crate::application::rulebot::intent_router::Intent::Unknown, lang, 0)
}
