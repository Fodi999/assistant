//! Lab Combo Pages — prerendered SEO landing pages for ingredient combos.
//!
//! Converts shareable Lab URLs (/lab?q=salmon,rice&goal=high_protein&meal=dinner)
//! into clean, Google-indexable SEO pages (/chef-tools/lab/combo/high-protein-dinner-salmon-rice).
//!
//! Pipeline:
//!   1. Admin POSTs ingredient combo + optional context → service generates page
//!   2. SmartService called server-side → full response cached in DB
//!   3. SEO metadata auto-generated from SmartResponse data
//!   4. Published → appears in sitemap, prerendered by Next.js ISR
//!
//! Public endpoints:
//!   GET /public/lab-combos/:slug?locale=en          → single published page
//!   GET /public/lab-combos/sitemap                  → lightweight list for sitemap
//!
//! Admin endpoints:
//!   POST /api/admin/lab-combos/generate             → create a combo page
//!   GET  /api/admin/lab-combos                      → list all (filter by status/locale)
//!   POST /api/admin/lab-combos/:id/publish          → publish
//!   POST /api/admin/lab-combos/:id/archive          → archive
//!   DELETE /api/admin/lab-combos/:id                → delete

use crate::application::smart_service::{CulinaryContext, SmartService};
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::infrastructure::R2Client;
use crate::shared::{AppError, AppResult};
use deunicode::deunicode;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct LabComboPage {
    pub id: Uuid,
    pub slug: String,
    pub locale: String,
    pub ingredients: Vec<String>,
    pub goal: Option<String>,
    pub meal_type: Option<String>,
    pub diet: Option<String>,
    pub cooking_time: Option<String>,
    pub budget: Option<String>,
    pub cuisine: Option<String>,
    pub title: String,
    pub description: String,
    pub h1: String,
    pub intro: String,
    pub why_it_works: String,
    pub how_to_cook: serde_json::Value,
    pub optimization_tips: serde_json::Value,
    pub image_url: Option<String>,
    pub process_image_url: Option<String>,
    pub detail_image_url: Option<String>,
    pub smart_response: serde_json::Value,
    pub faq: serde_json::Value,
    pub status: String,
    pub quality_score: i16,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Lightweight version for sitemap generation
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct LabComboSitemapEntry {
    pub slug: String,
    pub locale: String,
    pub updated_at: String,
    pub ingredients: Vec<String>,
    pub goal: Option<String>,
    pub meal_type: Option<String>,
}

/// Lightweight version for related combos (internal linking)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RelatedCombo {
    pub slug: String,
    pub title: String,
    pub ingredients: Vec<String>,
    pub goal: Option<String>,
    pub meal_type: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateComboRequest {
    /// Ingredient slugs (e.g. ["salmon", "rice", "avocado"])
    pub ingredients: Vec<String>,
    /// Target locale
    pub locale: String,
    /// Optional 6D context
    pub goal: Option<String>,
    pub meal_type: Option<String>,
    pub diet: Option<String>,
    pub cooking_time: Option<String>,
    pub budget: Option<String>,
    pub cuisine: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListCombosQuery {
    pub status: Option<String>,
    pub locale: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PublicComboSlugQuery {
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RelatedCombosQuery {
    pub locale: Option<String>,
    pub limit: Option<i64>,
}

/// Partial update for admin editing.
#[derive(Debug, Deserialize)]
pub struct UpdateComboRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub h1: Option<String>,
    pub intro: Option<String>,
    pub why_it_works: Option<String>,
    pub image_url: Option<String>,
    pub process_image_url: Option<String>,
    pub detail_image_url: Option<String>,
}

// ── Slug Builder ─────────────────────────────────────────────────────────────

/// Build a deterministic, SEO-friendly slug from ingredients + context.
///
/// SHORT slug formula: `{ingredients}-{meal_type}` (max 60 chars)
///
/// Examples:
///   - `["salmon", "rice"]` → `"rice-salmon"`
///   - `["salmon", "rice", "avocado"]` + meal=breakfast → `"avocado-rice-salmon-breakfast"`
///   - `["chicken", "broccoli", "rice"]` + goal=high_protein + meal=dinner → `"broccoli-chicken-rice-dinner"`
///
/// Only meal_type goes into slug (most search-relevant dimension).
/// Other context (goal, diet, etc.) lives in DB fields but NOT in URL.
pub fn combo_slug(
    ingredients: &[String],
    _goal: Option<&str>,
    meal_type: Option<&str>,
    _diet: Option<&str>,
    _cooking_time: Option<&str>,
    _budget: Option<&str>,
    _cuisine: Option<&str>,
) -> String {
    // Ingredients sorted alphabetically
    let mut sorted_ingredients: Vec<String> = ingredients
        .iter()
        .map(|s| {
            let clean = deunicode(s.trim())
                .to_lowercase()
                .replace(' ', "-")
                .replace('_', "-");
            clean
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-")
        })
        .collect();
    sorted_ingredients.sort();
    sorted_ingredients.dedup();

    let mut parts = sorted_ingredients;

    // Only meal_type in slug (most SEO-relevant)
    if let Some(m) = meal_type {
        parts.push(m.replace('_', "-"));
    }

    let slug = parts.join("-");
    // Hard limit: 60 chars to keep URLs short
    if slug.len() > 60 {
        slug.chars().take(60).collect::<String>()
            .trim_end_matches('-')
            .to_string()
    } else {
        slug
    }
}

// ── SEO Metadata Generation ──────────────────────────────────────────────────

/// Auto-generate SEO title from ingredients + context (≤ 60 chars).
/// Recipe-oriented, always includes a number hint.
/// Format: "Salmon Rice Bowl (28g Protein, 15 Min)"
fn generate_title(ingredients: &[String], goal: Option<&str>, meal_type: Option<&str>, _locale: &str) -> String {
    let names = capitalize_words(&ingredients.join(" "));
    let meal = meal_type
        .map(|m| capitalize_words(&m.replace('_', " ")))
        .unwrap_or_default();

    // Build dish name
    let dish = if meal.is_empty() {
        names.clone()
    } else {
        format!("{} {} Bowl", names, meal)
    };

    // Add number hook in parentheses — AI enrichment will overwrite with real data,
    // but template needs a good baseline for pages that fail AI enrichment.
    let hook = match goal {
        Some(g) if g.contains("protein") => "(High Protein, 15 Min)",
        Some(g) if g.contains("loss") || g.contains("low_cal") => "(Low Cal, 20 Min)",
        Some(g) if g.contains("keto") => "(Keto, 15 Min)",
        _ => "(15 Min)",
    };

    smart_truncate(&format!("{} {}", dish, hook), 60)
}

/// Auto-generate SEO description (80–155 chars). Recipe-oriented.
fn generate_description(ingredients: &[String], goal: Option<&str>, locale: &str) -> String {
    let names = ingredients.join(", ");
    let goal_text = goal
        .map(|g| format!(" for {}", g.replace('_', " ")))
        .unwrap_or_default();

    let desc = match locale {
        "ru" => format!(
            "Быстрый рецепт из {names}{goal_text}. Пошаговая инструкция, КБЖУ на порцию и советы шефа."
        ),
        "pl" => format!(
            "Szybki przepis z {names}{goal_text}. Instrukcja krok po kroku, KBJU na porcję i wskazówki szefa."
        ),
        "uk" => format!(
            "Швидкий рецепт з {names}{goal_text}. Покрокова інструкція, КБЖУ на порцію та поради шефа."
        ),
        _ => format!(
            "Quick recipe with {names}{goal_text}. Step-by-step instructions, macros per serving, and chef tips."
        ),
    };

    smart_truncate(&desc, 155)
}

/// Auto-generate H1 heading. Recipe-style, no "analysis" words.
fn generate_h1(ingredients: &[String], goal: Option<&str>, meal_type: Option<&str>, locale: &str) -> String {
    let names = capitalize_words(&ingredients.join(" "));
    let meal = meal_type.map(|m| capitalize_words(&m.replace('_', " ")));
    let goal_hint = goal.map(|g| capitalize_words(&g.replace('_', " ")));

    // Pattern: "Salmon Rice Avocado Bowl — Healthy Breakfast Recipe"
    match locale {
        "ru" => {
            let dish = if let Some(m) = &meal { format!("{names} — рецепт на {m}") }
                       else if let Some(g) = &goal_hint { format!("{names} — рецепт ({g})") }
                       else { format!("{names} — быстрый рецепт") };
            dish
        }
        "pl" => {
            let dish = if let Some(m) = &meal { format!("{names} — przepis na {m}") }
                       else if let Some(g) = &goal_hint { format!("{names} — przepis ({g})") }
                       else { format!("{names} — szybki przepis") };
            dish
        }
        "uk" => {
            let dish = if let Some(m) = &meal { format!("{names} — рецепт на {m}") }
                       else if let Some(g) = &goal_hint { format!("{names} — рецепт ({g})") }
                       else { format!("{names} — швидкий рецепт") };
            dish
        }
        _ => {
            let dish = if let Some(m) = &meal { format!("{names} Bowl — Healthy {m} Recipe") }
                       else if let Some(g) = &goal_hint { format!("{names} Recipe — {g}") }
                       else { format!("{names} — Quick & Easy Recipe") };
            dish
        }
    }
}

/// Auto-generate intro paragraph.
/// RULE: First sentence = direct answer to user intent (numbers + benefit).
/// This targets Google featured snippets.
fn generate_intro(ingredients: &[String], goal: Option<&str>, locale: &str) -> String {
    let names = ingredients.join(", ");
    let goal_benefit = match goal {
        Some(g) if g.contains("protein") => match locale {
            "ru" => " с высоким содержанием белка",
            "pl" => " z dużą ilością białka",
            "uk" => " з високим вмістом білка",
            _    => " packed with protein",
        },
        Some(g) if g.contains("loss") || g.contains("low_cal") => match locale {
            "ru" => " для контроля калорий",
            "pl" => " o niskiej kaloryczności",
            "uk" => " для контролю калорій",
            _    => " for calorie control",
        },
        _ => "",
    };

    match locale {
        "ru" => format!(
            "Это блюдо из {names}{goal_benefit} готовится за 15–20 минут. \
             Ниже — пошаговый рецепт с точными граммовками и КБЖУ на порцию.",
        ),
        "pl" => format!(
            "To danie z {names}{goal_benefit} przygotujesz w 15–20 minut. \
             Poniżej — przepis krok po kroku z dokładnymi gramówkami i KBJU na porcję.",
        ),
        "uk" => format!(
            "Ця страва з {names}{goal_benefit} готується за 15–20 хвилин. \
             Нижче — покроковий рецепт із точними грамовками та КБЖУ на порцію.",
        ),
        _ => format!(
            "This {names} dish{goal_benefit} is ready in 15–20 minutes. \
             Below: step-by-step recipe with exact portions and macros per serving.",
        ),
    }
}

/// Auto-generate FAQ from SmartResponse data. Recipe-oriented questions.
fn generate_faq(
    ingredients: &[String],
    smart: &serde_json::Value,
    locale: &str,
) -> serde_json::Value {
    let names = ingredients.join(", ");
    let mut faq = Vec::new();

    // Q1: How many calories/protein per serving?
    let nutrition = smart.get("nutrition");
    if let Some(n) = nutrition {
        let kcal = n.get("calories").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let prot = n.get("protein").and_then(|v| v.as_f64()).unwrap_or(0.0);
        // Estimate per-serving (roughly 300g portion)
        let serving_kcal = kcal * 3.0;
        let serving_prot = prot * 3.0;
        let q = match locale {
            "ru" => format!("Сколько калорий и белка в блюде из {}?", names),
            "pl" => format!("Ile kalorii i białka ma danie z {}?", names),
            "uk" => format!("Скільки калорій і білка у страві з {}?", names),
            _    => format!("How many calories and protein in a {} dish?", names),
        };
        let a = match locale {
            "ru" => format!("Примерно {:.0} ккал и {:.0} г белка на порцию (~300 г).", serving_kcal, serving_prot),
            "pl" => format!("Około {:.0} kcal i {:.0} g białka na porcję (~300 g).", serving_kcal, serving_prot),
            "uk" => format!("Приблизно {:.0} ккал і {:.0} г білка на порцію (~300 г).", serving_kcal, serving_prot),
            _    => format!("Approximately {:.0} kcal and {:.0}g protein per serving (~300g).", serving_kcal, serving_prot),
        };
        faq.push(serde_json::json!({ "question": q, "answer": a }));
    }

    // Q2: How long does it take to cook?
    {
        let q = match locale {
            "ru" => format!("Сколько времени готовить {}?", names),
            "pl" => format!("Ile czasu zajmuje przygotowanie {}?", names),
            "uk" => format!("Скільки часу готувати {}?", names),
            _    => format!("How long does it take to cook {}?", names),
        };
        let a = match locale {
            "ru" => "Активное время — 15–20 минут. Полное время с подготовкой — около 25 минут.".to_string(),
            "pl" => "Czas aktywny — 15–20 minut. Pełny czas z przygotowaniem — około 25 minut.".to_string(),
            "uk" => "Активний час — 15–20 хвилин. Повний час із підготовкою — близько 25 хвилин.".to_string(),
            _    => "Active time: 15–20 minutes. Total time including prep: about 25 minutes.".to_string(),
        };
        faq.push(serde_json::json!({ "question": q, "answer": a }));
    }

    // Q3: What can I substitute?
    let suggestions = smart.get("suggestions").and_then(|s| s.as_array());
    if let Some(sugg) = suggestions {
        let top: Vec<String> = sugg.iter()
            .take(3)
            .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect();
        if !top.is_empty() {
            let q = match locale {
                "ru" => format!("Чем можно заменить ингредиенты в рецепте?"),
                "pl" => format!("Czym można zastąpić składniki w przepisie?"),
                "uk" => format!("Чим можна замінити інгредієнти в рецепті?"),
                _    => format!("What substitutions work in this recipe?"),
            };
            let a = match locale {
                "ru" => format!("Попробуйте добавить или заменить на: {}.", top.join(", ")),
                "pl" => format!("Spróbuj dodać lub zamienić na: {}.", top.join(", ")),
                "uk" => format!("Спробуйте додати або замінити на: {}.", top.join(", ")),
                _    => format!("Try adding or swapping with: {}.", top.join(", ")),
            };
            faq.push(serde_json::json!({ "question": q, "answer": a }));
        }
    }

    // Q4: What dish types can I make?
    let variants = smart.get("variants").and_then(|v| v.as_array());
    if let Some(vars) = variants {
        if !vars.is_empty() {
            let q = match locale {
                "ru" => format!("Какие блюда можно приготовить из {}?", names),
                "pl" => format!("Jakie dania można zrobić z {}?", names),
                "uk" => format!("Які страви можна приготувати з {}?", names),
                _    => format!("What dishes can I make with {}?", names),
            };
            let variant_names: Vec<String> = vars.iter()
                .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect();
            let a = if variant_names.is_empty() {
                match locale {
                    "ru" => format!("{} вариантов блюд — от боула до салата.", vars.len()),
                    "pl" => format!("{} wariantów dań — od bowla po sałatkę.", vars.len()),
                    "uk" => format!("{} варіантів страв — від боулу до салату.", vars.len()),
                    _    => format!("{} dish variants — from bowl to salad.", vars.len()),
                }
            } else {
                variant_names.join("; ")
            };
            faq.push(serde_json::json!({ "question": q, "answer": a }));
        }
    }

    serde_json::json!(faq)
}

// ── "Why This Combo Works" Generator ─────────────────────────────────────────

/// Generate a rich "why this combo works" paragraph from SmartResponse data.
fn generate_why_it_works(
    ingredients: &[String],
    smart: &serde_json::Value,
    goal: Option<&str>,
    locale: &str,
) -> String {
    let names = ingredients.join(", ");

    // Extract nutrition highlights
    let nutrition = smart.get("nutrition");
    let protein = nutrition.and_then(|n| n.get("protein")).and_then(|v| v.as_f64());
    let calories = nutrition.and_then(|n| n.get("calories")).and_then(|v| v.as_f64());
    let fiber = nutrition.and_then(|n| n.get("fiber")).and_then(|v| v.as_f64());

    // Extract flavor balance
    let balance_score = smart
        .get("flavor_profile")
        .and_then(|f| f.get("balance"))
        .and_then(|b| b.get("score"))
        .and_then(|v| v.as_f64());

    // Extract dominant tastes
    let dominant_tastes: Vec<String> = smart
        .get("flavor_profile")
        .and_then(|f| f.get("balance"))
        .and_then(|b| b.get("dominant_tastes"))
        .and_then(|dt| dt.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Extract variant info (what dish types are possible)
    let variant_types: Vec<String> = smart
        .get("variants")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("variant_type").and_then(|t| t.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Build explanation parts
    let mut parts: Vec<String> = Vec::new();

    // Part 1: Nutritional reason
    match locale {
        "ru" => {
            if let Some(p) = protein {
                if p > 15.0 {
                    parts.push(format!("Эта комбинация содержит {:.1} г белка на 100 г — отличный источник протеина", p));
                } else {
                    parts.push(format!("Комбинация {names} даёт {:.0} ккал на 100 г", calories.unwrap_or(0.0)));
                }
            }
        }
        "pl" => {
            if let Some(p) = protein {
                if p > 15.0 {
                    parts.push(format!("Ta kombinacja zawiera {:.1} g białka na 100 g — świetne źródło proteiny", p));
                } else {
                    parts.push(format!("Kombinacja {names} dostarcza {:.0} kcal na 100 g", calories.unwrap_or(0.0)));
                }
            }
        }
        "uk" => {
            if let Some(p) = protein {
                if p > 15.0 {
                    parts.push(format!("Ця комбінація містить {:.1} г білка на 100 г — чудове джерело протеїну", p));
                } else {
                    parts.push(format!("Комбінація {names} дає {:.0} ккал на 100 г", calories.unwrap_or(0.0)));
                }
            }
        }
        _ => {
            if let Some(p) = protein {
                if p > 15.0 {
                    parts.push(format!("This combination provides {:.1}g of protein per 100g — an excellent protein source", p));
                } else {
                    parts.push(format!("The combination of {names} delivers {:.0} kcal per 100g", calories.unwrap_or(0.0)));
                }
            }
        }
    }

    // Part 2: Fiber bonus
    if let Some(f) = fiber {
        if f > 3.0 {
            let fiber_note = match locale {
                "ru" => format!("Содержит {:.1} г клетчатки, что поддерживает пищеварение", f),
                "pl" => format!("Zawiera {:.1} g błonnika, co wspiera trawienie", f),
                "uk" => format!("Містить {:.1} г клітковини, що підтримує травлення", f),
                _    => format!("Contains {:.1}g of fiber, supporting healthy digestion", f),
            };
            parts.push(fiber_note);
        }
    }

    // Part 3: Flavor balance
    if let Some(score) = balance_score {
        let tastes_str = if !dominant_tastes.is_empty() {
            dominant_tastes.join(", ")
        } else {
            String::new()
        };
        let flavor_note = match locale {
            "ru" => {
                if tastes_str.is_empty() {
                    format!("Баланс вкуса оценивается в {:.0}/100", score)
                } else {
                    format!("Доминирующие вкусы — {tastes_str}, баланс {:.0}/100", score)
                }
            }
            "pl" => {
                if tastes_str.is_empty() {
                    format!("Balans smakowy oceniany na {:.0}/100", score)
                } else {
                    format!("Dominujące smaki — {tastes_str}, balans {:.0}/100", score)
                }
            }
            "uk" => {
                if tastes_str.is_empty() {
                    format!("Баланс смаку оцінюється в {:.0}/100", score)
                } else {
                    format!("Домінуючі смаки — {tastes_str}, баланс {:.0}/100", score)
                }
            }
            _ => {
                if tastes_str.is_empty() {
                    format!("Flavor balance scores {:.0}/100", score)
                } else {
                    format!("Dominant flavors are {tastes_str}, with a balance score of {:.0}/100", score)
                }
            }
        };
        parts.push(flavor_note);
    }

    // Part 4: Goal context
    if let Some(g) = goal {
        let goal_note = match locale {
            "ru" => format!("Оптимизировано для цели: {}", g.replace('_', " ")),
            "pl" => format!("Zoptymalizowane pod cel: {}", g.replace('_', " ")),
            "uk" => format!("Оптимізовано для мети: {}", g.replace('_', " ")),
            _    => format!("Optimized for {}", g.replace('_', " ")),
        };
        parts.push(goal_note);
    }

    // Part 5: Variant versatility
    if !variant_types.is_empty() {
        let types_str = variant_types.join(", ");
        let versatility = match locale {
            "ru" => format!("Подходит для разных стилей подачи: {types_str}"),
            "pl" => format!("Pasuje do różnych stylów podania: {types_str}"),
            "uk" => format!("Підходить для різних стилів подачі: {types_str}"),
            _    => format!("Versatile enough for multiple serving styles: {types_str}"),
        };
        parts.push(versatility);
    }

    if parts.is_empty() {
        return match locale {
            "ru" => format!("{names} — сбалансированное сочетание белка, углеводов и полезных жиров."),
            "pl" => format!("{names} — zbalansowane połączenie białka, węglowodanów i zdrowych tłuszczów."),
            "uk" => format!("{names} — збалансоване поєднання білка, вуглеводів і корисних жирів."),
            _    => format!("{names} — a balanced mix of protein, carbs, and healthy fats."),
        };
    }

    format!("{}.", parts.join(". "))
}

// ── "How to Cook" Generator ──────────────────────────────────────────────────

/// Generate cooking steps from the SmartResponse variants data.
/// CONCRETE cooking actions — "Pan-sear salmon 5 min per side", not "Cook base".
fn generate_how_to_cook(
    ingredients: &[String],
    smart: &serde_json::Value,
    locale: &str,
) -> serde_json::Value {
    let variants = smart.get("variants").and_then(|v| v.as_array());

    // If we have variants, use the "balanced" one as the reference recipe
    let reference_variant = variants.and_then(|vars| {
        vars.iter()
            .find(|v| v.get("variant_type").and_then(|t| t.as_str()) == Some("balanced"))
            .or_else(|| vars.first())
    });

    let mut steps: Vec<serde_json::Value> = Vec::new();

    if let Some(variant) = reference_variant {
        let variant_ingredients = variant.get("ingredients").and_then(|i| i.as_array());

        if let Some(vi) = variant_ingredients {
            // Group ingredients by role for cooking order
            let bases: Vec<&serde_json::Value> = vi.iter()
                .filter(|i| i.get("role").and_then(|r| r.as_str()) == Some("base"))
                .collect();
            let sides: Vec<&serde_json::Value> = vi.iter()
                .filter(|i| i.get("role").and_then(|r| r.as_str()) == Some("side"))
                .collect();
            let aromatics: Vec<&serde_json::Value> = vi.iter()
                .filter(|i| {
                    let role = i.get("role").and_then(|r| r.as_str()).unwrap_or("");
                    role == "aromatic" || role == "fat" || role == "sauce"
                })
                .collect();

            let mut step_num = 1;

            // Step 1: Cook base ingredients (protein/grains) with SPECIFIC instructions
            if !bases.is_empty() {
                let base_details: Vec<String> = bases.iter()
                    .filter_map(|b| {
                        let name = b.get("name").and_then(|n| n.as_str())?;
                        let grams = b.get("grams").and_then(|g| g.as_f64())?;
                        Some(format!("{name} ({grams:.0} g)"))
                    })
                    .collect();
                let base_names: Vec<String> = bases.iter()
                    .filter_map(|b| b.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect();

                let step_text = match locale {
                    "ru" => format!("Приготовьте {}. Обжарьте на среднем огне 5–7 минут с каждой стороны или отварите до готовности.", base_details.join(", ")),
                    "pl" => format!("Przygotuj {}. Usmaż na średnim ogniu 5–7 minut z każdej strony lub ugotuj do gotowości.", base_details.join(", ")),
                    "uk" => format!("Приготуйте {}. Обсмажте на середньому вогні 5–7 хвилин з кожного боку або зваріть до готовності.", base_details.join(", ")),
                    _    => format!("Cook {}. Pan-sear over medium heat 5–7 min per side, or boil until done.", base_details.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 10,
                    "ingredients": base_names
                }));
                step_num += 1;
            }

            // Step 2: Prepare sides (rice, vegetables, etc.)
            if !sides.is_empty() {
                let side_details: Vec<String> = sides.iter()
                    .filter_map(|s| {
                        let name = s.get("name").and_then(|n| n.as_str())?;
                        let grams = s.get("grams").and_then(|g| g.as_f64())?;
                        Some(format!("{name} ({grams:.0} g)"))
                    })
                    .collect();

                let step_text = match locale {
                    "ru" => format!("Подготовьте гарнир: {}. Нарежьте овощи и отварите крупы (10–12 мин).", side_details.join(", ")),
                    "pl" => format!("Przygotuj dodatki: {}. Pokrój warzywa i ugotuj zboża (10–12 min).", side_details.join(", ")),
                    "uk" => format!("Підготуйте гарнір: {}. Наріжте овочі та зваріть крупи (10–12 хв).", side_details.join(", ")),
                    _    => format!("Prepare sides: {}. Slice vegetables and cook grains (10–12 min).", side_details.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 12,
                    "ingredients": sides.iter()
                        .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(String::from))
                        .collect::<Vec<_>>()
                }));
                step_num += 1;
            }

            // Step 3: Add aromatics/sauces
            if !aromatics.is_empty() {
                let aro_names: Vec<String> = aromatics.iter()
                    .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect();

                let step_text = match locale {
                    "ru" => format!("Добавьте {}. Перемешайте и прогрейте 1–2 мин.", aro_names.join(", ")),
                    "pl" => format!("Dodaj {}. Wymieszaj i podgrzewaj 1–2 min.", aro_names.join(", ")),
                    "uk" => format!("Додайте {}. Перемішайте та прогрійте 1–2 хв.", aro_names.join(", ")),
                    _    => format!("Add {}. Toss together and heat through for 1–2 min.", aro_names.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 2,
                    "ingredients": aro_names
                }));
                step_num += 1;
            }

            // Final step: Assemble and serve with per-serving macros
            let total_cal = variant.get("total_calories").and_then(|c| c.as_i64()).unwrap_or(0);
            let total_prot = variant.get("total_protein").and_then(|p| p.as_f64())
                .or_else(|| {
                    smart.get("nutrition")
                        .and_then(|n| n.get("protein"))
                        .and_then(|v| v.as_f64())
                        .map(|p| p * 3.0) // estimate for ~300g serving
                })
                .unwrap_or(0.0);

            let assemble = match locale {
                "ru" => format!("Выложите на тарелку и подавайте. Порция: ~{total_cal} ккал, ~{total_prot:.0} г белка."),
                "pl" => format!("Ułóż na talerzu i podaj. Porcja: ~{total_cal} kcal, ~{total_prot:.0} g białka."),
                "uk" => format!("Викладіть на тарілку та подавайте. Порція: ~{total_cal} ккал, ~{total_prot:.0} г білка."),
                _    => format!("Plate up and serve. Per serving: ~{total_cal} kcal, ~{total_prot:.0}g protein."),
            };
            steps.push(serde_json::json!({
                "step": step_num,
                "text": assemble,
                "time_minutes": 2
            }));
        }
    }

    // Fallback: ingredient-specific steps if no variant data
    if steps.is_empty() {
        let names_list = ingredients.join(", ");
        let s1 = match locale {
            "ru" => format!("Подготовьте ингредиенты: {}. Вымойте и нарежьте.", names_list),
            "pl" => format!("Przygotuj składniki: {}. Umyj i pokrój.", names_list),
            "uk" => format!("Підготуйте інгредієнти: {}. Вимийте та наріжте.", names_list),
            _    => format!("Prep your ingredients: {}. Wash and cut.", names_list),
        };
        let s2 = match locale {
            "ru" => "Обжарьте на среднем огне 7–10 минут, помешивая.".to_string(),
            "pl" => "Usmaż na średnim ogniu 7–10 minut, mieszając.".to_string(),
            "uk" => "Обсмажте на середньому вогні 7–10 хвилин, помішуючи.".to_string(),
            _    => "Cook over medium heat for 7–10 minutes, stirring occasionally.".to_string(),
        };
        let s3 = match locale {
            "ru" => "Выложите на тарелку и подавайте сразу.".to_string(),
            "pl" => "Ułóż na talerzu i podaj od razu.".to_string(),
            "uk" => "Викладіть на тарілку та подавайте одразу.".to_string(),
            _    => "Plate up and serve immediately.".to_string(),
        };
        steps = vec![
            serde_json::json!({ "step": 1, "text": s1, "time_minutes": 5 }),
            serde_json::json!({ "step": 2, "text": s2, "time_minutes": 10 }),
            serde_json::json!({ "step": 3, "text": s3, "time_minutes": 2 }),
        ];
    }

    serde_json::json!(steps)
}

// ── Optimization Tips Generator ──────────────────────────────────────────────

/// Generate optimization/adjustment tips from next_actions and diagnostics.
fn generate_optimization_tips(
    smart: &serde_json::Value,
    locale: &str,
) -> serde_json::Value {
    let mut tips: Vec<serde_json::Value> = Vec::new();

    // From next_actions
    if let Some(actions) = smart.get("next_actions").and_then(|a| a.as_array()) {
        for action in actions.iter().take(5) {
            let action_type = action.get("type").and_then(|t| t.as_str()).unwrap_or("add");
            let name = action.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let reason = action.get("reason").and_then(|r| r.as_str()).unwrap_or("");
            if name.is_empty() { continue; }

            let icon = match action_type {
                "add" => "➕",
                "remove" => "➖",
                "swap" => "🔄",
                "adjust" => "⚙️",
                _ => "💡",
            };

            tips.push(serde_json::json!({
                "icon": icon,
                "action": action_type,
                "ingredient": name,
                "tip": reason
            }));
        }
    }

    // From diagnostics issues
    if let Some(diag) = smart.get("diagnostics") {
        if let Some(issues) = diag.get("issues").and_then(|i| i.as_array()) {
            for issue in issues.iter().take(3) {
                let severity = issue.get("severity").and_then(|s| s.as_str()).unwrap_or("info");
                let message = issue.get("message").and_then(|m| m.as_str()).unwrap_or("");
                if message.is_empty() { continue; }

                let icon = match severity {
                    "critical" => "🔴",
                    "warning" => "🟡",
                    _ => "💡",
                };

                tips.push(serde_json::json!({
                    "icon": icon,
                    "action": "tip",
                    "ingredient": "",
                    "tip": message
                }));
            }
        }
    }

    // Fallback generic tips if nothing from engine
    if tips.is_empty() {
        let generic = match locale {
            "ru" => vec![
                ("➕", "Добавьте оливковое масло для улучшения текстуры и усвоения жирорастворимых витаминов"),
                ("➕", "Добавьте лимонный сок для яркости вкуса и лучшего усвоения железа"),
                ("⚙️", "Контролируйте порцию — начните с рекомендованных граммов и корректируйте"),
            ],
            "pl" => vec![
                ("➕", "Dodaj oliwę z oliwek dla lepszej tekstury i wchłaniania witamin rozpuszczalnych w tłuszczach"),
                ("➕", "Dodaj sok z cytryny dla świeżości smaku i lepszego wchłaniania żelaza"),
                ("⚙️", "Kontroluj porcję — zacznij od zalecanych gramów i dostosuj"),
            ],
            "uk" => vec![
                ("➕", "Додайте оливкову олію для кращої текстури та засвоєння жиророзчинних вітамінів"),
                ("➕", "Додайте лимонний сік для яскравості смаку та кращого засвоєння заліза"),
                ("⚙️", "Контролюйте порцію — почніть із рекомендованих грамів та коригуйте"),
            ],
            _ => vec![
                ("➕", "Add olive oil to improve mouthfeel and fat-soluble vitamin absorption"),
                ("➕", "Add lemon juice for brightness and better iron absorption"),
                ("⚙️", "Control portions — start with recommended grams and adjust to taste"),
            ],
        };
        for (icon, tip) in generic {
            tips.push(serde_json::json!({
                "icon": icon,
                "action": "tip",
                "ingredient": "",
                "tip": tip
            }));
        }
    }

    serde_json::json!(tips)
}

// ── Quality Scoring ──────────────────────────────────────────────────────────

/// Score a lab combo page (0-5), similar to intent_pages audit.
fn quality_score(page: &LabComboPage) -> i16 {
    let mut score: i16 = 0;

    // +1: title ≤ 60 chars
    if page.title.chars().count() <= 60 && !page.title.is_empty() {
        score += 1;
    }

    // +1: description 80-155 chars
    let desc_len = page.description.chars().count();
    if desc_len >= 80 && desc_len <= 155 {
        score += 1;
    }

    // +1: intro ≥ 100 chars
    if page.intro.chars().count() >= 100 {
        score += 1;
    }

    // +1: why_it_works ≥ 50 chars
    if page.why_it_works.chars().count() >= 50 {
        score += 1;
    }

    // +1: how_to_cook ≥ 3 steps
    let cook_steps = page.how_to_cook.as_array().map(|a| a.len()).unwrap_or(0);
    if cook_steps >= 3 {
        score += 1;
    }

    // Clamp to 5 max
    score.min(5)
}

// ── Service ──────────────────────────────────────────────────────────────────

pub struct LabComboService {
    pool: PgPool,
    smart_service: Arc<SmartService>,
    r2_client: R2Client,
    llm_adapter: Arc<LlmAdapter>,
}

impl LabComboService {
    pub fn new(pool: PgPool, smart_service: Arc<SmartService>, r2_client: R2Client, llm_adapter: Arc<LlmAdapter>) -> Self {
        Self { pool, smart_service, r2_client, llm_adapter }
    }

    // ── Generate (Admin) ─────────────────────────────────────────────────

    /// Generate a lab combo page by calling SmartService and caching the response.
    pub async fn generate(&self, req: GenerateComboRequest) -> AppResult<LabComboPage> {
        if req.ingredients.is_empty() {
            return Err(AppError::validation("ingredients array must not be empty"));
        }
        if req.ingredients.len() > 10 {
            return Err(AppError::validation("max 10 ingredients per combo"));
        }

        // Sort + dedup ingredients for deterministic slug
        let mut ingredients = req.ingredients.clone();
        ingredients.sort();
        ingredients.dedup();

        let slug = combo_slug(
            &ingredients,
            req.goal.as_deref(),
            req.meal_type.as_deref(),
            req.diet.as_deref(),
            req.cooking_time.as_deref(),
            req.budget.as_deref(),
            req.cuisine.as_deref(),
        );

        // Check if already exists
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM lab_combo_pages WHERE slug = $1 AND locale = $2"
        )
            .bind(&slug)
            .bind(&req.locale)
            .fetch_optional(&self.pool)
            .await?;

        if exists.is_some() {
            return Err(AppError::validation(&format!(
                "combo page already exists: slug={}, locale={}",
                slug, req.locale
            )));
        }

        // Call SmartService to get full analysis
        let main = ingredients[0].clone();
        let additional: Vec<String> = ingredients[1..].to_vec();

        let ctx = CulinaryContext {
            ingredient: main,
            state: None,
            additional_ingredients: additional,
            goal: req.goal.clone(),
            meal_type: req.meal_type.clone(),
            diet: req.diet.clone(),
            cooking_time: req.cooking_time.clone(),
            budget: req.budget.clone(),
            cuisine: req.cuisine.clone(),
            lang: req.locale.clone(),
            session_id: None,
        };

        let smart_result = self.smart_service.get_smart_ingredient(ctx).await?;
        let smart_json = serde_json::to_value(&smart_result)
            .map_err(|e| AppError::internal(format!("failed to serialize SmartResponse: {}", e)))?;

        // Generate SEO metadata
        let title = generate_title(&ingredients, req.goal.as_deref(), req.meal_type.as_deref(), &req.locale);
        let description = generate_description(&ingredients, req.goal.as_deref(), &req.locale);
        let h1 = generate_h1(&ingredients, req.goal.as_deref(), req.meal_type.as_deref(), &req.locale);
        let intro = generate_intro(&ingredients, req.goal.as_deref(), &req.locale);
        let faq = generate_faq(&ingredients, &smart_json, &req.locale);
        let why_it_works = generate_why_it_works(&ingredients, &smart_json, req.goal.as_deref(), &req.locale);
        let how_to_cook = generate_how_to_cook(&ingredients, &smart_json, &req.locale);
        let optimization_tips = generate_optimization_tips(&smart_json, &req.locale);

        let id = Uuid::new_v4();

        let page = sqlx::query_as::<_, LabComboPage>(
            r#"
            INSERT INTO lab_combo_pages (
                id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips,
                smart_response, faq, status
            ) VALUES (
                $1, $2, $3, $4,
                $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14,
                $15, $16, $17,
                $18, $19, 'draft'
            )
            RETURNING id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
                smart_response, faq, status, quality_score,
                published_at::text, created_at::text, updated_at::text
            "#,
        )
        .bind(id)
        .bind(&slug)
        .bind(&req.locale)
        .bind(&ingredients)
        .bind(&req.goal)
        .bind(&req.meal_type)
        .bind(&req.diet)
        .bind(&req.cooking_time)
        .bind(&req.budget)
        .bind(&req.cuisine)
        .bind(&title)
        .bind(&description)
        .bind(&h1)
        .bind(&intro)
        .bind(&why_it_works)
        .bind(&how_to_cook)
        .bind(&optimization_tips)
        .bind(&smart_json)
        .bind(&faq)
        .fetch_one(&self.pool)
        .await?;

        // Update quality score
        let qs = quality_score(&page);
        sqlx::query("UPDATE lab_combo_pages SET quality_score = $1 WHERE id = $2")
            .bind(qs)
            .bind(id)
            .execute(&self.pool)
            .await?;

        // ── AI Enrichment (async) ────────────────────────────────────────
        // Rewrite template-based SEO text into unique, compelling copy via Gemini.
        // Runs in background so generate returns fast; enriched fields appear on refresh.
        let pool_bg = self.pool.clone();
        let llm_bg = self.llm_adapter.clone();
        let ingredients_bg = ingredients.clone();
        let locale_bg = req.locale.clone();
        let goal_bg = req.goal.clone();
        let meal_type_bg = req.meal_type.clone();
        tokio::spawn(async move {
            if let Err(e) = enrich_seo_with_ai(
                &pool_bg, &llm_bg, id,
                &ingredients_bg, &locale_bg,
                goal_bg.as_deref(), meal_type_bg.as_deref(),
            ).await {
                tracing::warn!("⚠️ AI enrichment failed for combo {}: {}", id, e);
            }
        });

        Ok(page)
    }

    // ── Publish (Admin) ──────────────────────────────────────────────────

    pub async fn publish(&self, id: Uuid) -> AppResult<LabComboPage> {
        let page = sqlx::query_as::<_, LabComboPage>(
            r#"
            UPDATE lab_combo_pages
            SET status = 'published',
                published_at = now(),
                updated_at = now()
            WHERE id = $1
            RETURNING id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
                smart_response, faq, status, quality_score,
                published_at::text, created_at::text, updated_at::text
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("combo page not found"))?;

        Ok(page)
    }

    // ── Archive (Admin) ──────────────────────────────────────────────────

    pub async fn archive(&self, id: Uuid) -> AppResult<LabComboPage> {
        let page = sqlx::query_as::<_, LabComboPage>(
            r#"
            UPDATE lab_combo_pages
            SET status = 'archived', updated_at = now()
            WHERE id = $1
            RETURNING id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
                smart_response, faq, status, quality_score,
                published_at::text, created_at::text, updated_at::text
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("combo page not found"))?;

        Ok(page)
    }

    // ── Delete (Admin) ───────────────────────────────────────────────────

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM lab_combo_pages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("combo page not found"));
        }

        Ok(())
    }

    // ── Update (Admin) ───────────────────────────────────────────────────

    pub async fn update(&self, id: Uuid, req: UpdateComboRequest) -> AppResult<LabComboPage> {
        let page = sqlx::query_as::<_, LabComboPage>(
            r#"
            UPDATE lab_combo_pages SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                h1 = COALESCE($4, h1),
                intro = COALESCE($5, intro),
                why_it_works = COALESCE($6, why_it_works),
                image_url = COALESCE($7, image_url),
                process_image_url = COALESCE($8, process_image_url),
                detail_image_url = COALESCE($9, detail_image_url),
                updated_at = now()
            WHERE id = $1
            RETURNING id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
                smart_response, faq, status, quality_score,
                published_at::text, created_at::text, updated_at::text
            "#,
        )
        .bind(id)
        .bind(&req.title)
        .bind(&req.description)
        .bind(&req.h1)
        .bind(&req.intro)
        .bind(&req.why_it_works)
        .bind(&req.image_url)
        .bind(&req.process_image_url)
        .bind(&req.detail_image_url)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("combo page not found"))?;

        // Recalculate quality score
        let qs = quality_score(&page);
        sqlx::query("UPDATE lab_combo_pages SET quality_score = $1 WHERE id = $2")
            .bind(qs)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(LabComboPage {
            quality_score: qs,
            ..page
        })
    }

    // ── List (Admin) ─────────────────────────────────────────────────────

    pub async fn list(&self, query: ListCombosQuery) -> AppResult<Vec<LabComboPage>> {
        let limit = query.limit.unwrap_or(50).min(200);
        let offset = query.offset.unwrap_or(0);

        let rows = sqlx::query_as::<_, LabComboPage>(
            r#"
            SELECT id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
                smart_response, faq, status, quality_score,
                published_at::text, created_at::text, updated_at::text
            FROM lab_combo_pages
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR locale = $2)
            ORDER BY updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(&query.status)
        .bind(&query.locale)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ── Public: get published page by slug ───────────────────────────────

    pub async fn get_published(&self, slug: &str, locale: &str) -> AppResult<Option<LabComboPage>> {
        let page = sqlx::query_as::<_, LabComboPage>(
            r#"
            SELECT id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
                smart_response, faq, status, quality_score,
                published_at::text, created_at::text, updated_at::text
            FROM lab_combo_pages
            WHERE slug = $1 AND locale = $2 AND status = 'published'
            "#,
        )
        .bind(slug)
        .bind(locale)
        .fetch_optional(&self.pool)
        .await?;

        Ok(page)
    }

    // ── Public: related combos (for internal linking) ────────────────────

    /// Find published combos that share at least 1 ingredient with the given slug.
    /// Returns up to 6 lightweight entries for the "Related Combos" section.
    pub async fn get_related_combos(
        &self,
        slug: &str,
        locale: &str,
        limit: i64,
    ) -> AppResult<Vec<RelatedCombo>> {
        let rows = sqlx::query_as::<_, RelatedCombo>(
            r#"
            WITH current AS (
                SELECT id, ingredients FROM lab_combo_pages
                WHERE slug = $1 AND locale = $2 AND status = 'published'
                LIMIT 1
            )
            SELECT p.slug, p.title, p.ingredients, p.goal, p.meal_type, p.image_url
            FROM lab_combo_pages p, current c
            WHERE p.status = 'published'
              AND p.locale = $2
              AND p.id != c.id
              AND p.ingredients && c.ingredients
            ORDER BY array_length(
                ARRAY(SELECT unnest(p.ingredients) INTERSECT SELECT unnest(c.ingredients)), 1
            ) DESC, p.quality_score DESC
            LIMIT $3
            "#,
        )
        .bind(slug)
        .bind(locale)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ── Public: "People also cook" — discovery by same goal/meal, different ingredients

    pub async fn get_also_cook(
        &self,
        slug: &str,
        locale: &str,
        limit: i64,
    ) -> AppResult<Vec<RelatedCombo>> {
        let rows = sqlx::query_as::<_, RelatedCombo>(
            r#"
            WITH current AS (
                SELECT id, ingredients, goal, meal_type
                FROM lab_combo_pages
                WHERE slug = $1 AND locale = $2 AND status = 'published'
                LIMIT 1
            )
            SELECT p.slug, p.title, p.ingredients, p.goal, p.meal_type, p.image_url
            FROM lab_combo_pages p, current c
            WHERE p.status = 'published'
              AND p.locale = $2
              AND p.id != c.id
              AND (
                  (p.goal IS NOT NULL AND p.goal = c.goal)
                  OR (p.meal_type IS NOT NULL AND p.meal_type = c.meal_type)
              )
              AND NOT (p.ingredients && c.ingredients)
            ORDER BY p.quality_score DESC, p.published_at DESC
            LIMIT $3
            "#,
        )
        .bind(slug)
        .bind(locale)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ── Public: sitemap data ─────────────────────────────────────────────

    pub async fn sitemap(&self) -> AppResult<Vec<LabComboSitemapEntry>> {
        let rows = sqlx::query_as::<_, LabComboSitemapEntry>(
            r#"
            SELECT slug, locale, updated_at::text, ingredients, goal, meal_type
            FROM lab_combo_pages
            WHERE status = 'published'
            ORDER BY published_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ── Bulk generate popular combos (Admin convenience) ─────────────────

    pub async fn generate_popular_combos(&self, locale: &str, limit: usize) -> AppResult<Vec<String>> {
        // Popular ingredient combos that people actually search for
        let popular_combos: Vec<(Vec<&str>, Option<&str>, Option<&str>)> = vec![
            // High-protein combos
            (vec!["chicken", "broccoli", "rice"], Some("high_protein"), Some("dinner")),
            (vec!["salmon", "avocado", "rice"], Some("high_protein"), Some("lunch")),
            (vec!["eggs", "spinach", "tomato"], Some("high_protein"), Some("breakfast")),
            (vec!["tuna", "quinoa", "cucumber"], Some("high_protein"), Some("lunch")),
            (vec!["chicken", "sweet-potato", "green-beans"], Some("high_protein"), Some("dinner")),

            // Weight loss combos
            (vec!["chicken", "salad", "cucumber"], Some("weight_loss"), Some("lunch")),
            (vec!["salmon", "asparagus", "lemon"], Some("low_carb"), Some("dinner")),
            (vec!["eggs", "avocado"], Some("keto"), Some("breakfast")),

            // Classic combos (no specific goal)
            (vec!["pasta", "tomato", "basil"], None, Some("dinner")),
            (vec!["rice", "chicken", "soy-sauce"], None, Some("dinner")),
            (vec!["potato", "onion", "mushroom"], None, Some("dinner")),
            (vec!["banana", "oats", "milk"], None, Some("breakfast")),

            // Vegan combos
            (vec!["tofu", "rice", "broccoli"], Some("high_protein"), Some("dinner")),
            (vec!["lentils", "rice", "tomato"], Some("high_protein"), Some("lunch")),
        ];

        let mut generated = Vec::new();

        for (ings, goal, meal) in popular_combos.into_iter().take(limit) {
            let slug = combo_slug(
                &ings.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                goal, meal, None, None, None, None,
            );

            // Skip if already exists
            let exists: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM lab_combo_pages WHERE slug = $1 AND locale = $2"
            )
                .bind(&slug)
                .bind(locale)
                .fetch_optional(&self.pool)
                .await?;

            if exists.is_some() {
                continue;
            }

            match self.generate(GenerateComboRequest {
                ingredients: ings.iter().map(|s| s.to_string()).collect(),
                locale: locale.to_string(),
                goal: goal.map(String::from),
                meal_type: meal.map(String::from),
                diet: None,
                cooking_time: None,
                budget: None,
                cuisine: None,
            }).await {
                Ok(page) => generated.push(format!("✅ {} ({})", page.slug, page.locale)),
                Err(e) => generated.push(format!("❌ {} — {}", slug, e)),
            }
        }

        Ok(generated)
    }

    // ── Image upload (presigned URL flow) ────────────────────────────────

    /// GET /api/admin/lab-combos/:id/image-upload-url?content_type=image/webp
    /// Returns a presigned R2 upload URL + the resulting public URL.
    pub async fn get_image_upload_url(
        &self,
        id: Uuid,
        content_type: &str,
    ) -> AppResult<crate::application::user::AvatarUploadResponse> {
        // Verify combo exists
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lab_combo_pages WHERE id = $1)")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        if !exists {
            return Err(AppError::not_found("Lab combo not found"));
        }

        let ext = if content_type.contains("jpeg") || content_type.contains("jpg") {
            "jpg"
        } else if content_type.contains("png") {
            "png"
        } else {
            "webp"
        };

        let key = format!("assets/lab-combos/{}.{}", id, ext);
        let upload_url = self.r2_client.generate_presigned_upload_url(&key, content_type).await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(crate::application::user::AvatarUploadResponse { upload_url, public_url })
    }

    /// PUT /api/admin/lab-combos/:id/image-url
    /// Saves the public URL after the frontend has uploaded the file to R2.
    pub async fn save_image_url(&self, id: Uuid, image_url: String) -> AppResult<LabComboPage> {
        self.save_typed_image_url(id, "hero", image_url).await
    }

    /// GET /api/admin/lab-combos/:id/image-upload-url/:kind?content_type=image/webp
    /// Returns presigned URL for hero/process/detail image.
    pub async fn get_typed_image_upload_url(
        &self,
        id: Uuid,
        kind: &str,
        content_type: &str,
    ) -> AppResult<crate::application::user::AvatarUploadResponse> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lab_combo_pages WHERE id = $1)")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        if !exists {
            return Err(AppError::not_found("Lab combo not found"));
        }

        let ext = if content_type.contains("jpeg") || content_type.contains("jpg") {
            "jpg"
        } else if content_type.contains("png") {
            "png"
        } else {
            "webp"
        };

        let key = format!("assets/lab-combos/{}-{}.{}", id, kind, ext);
        let upload_url = self.r2_client.generate_presigned_upload_url(&key, content_type).await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(crate::application::user::AvatarUploadResponse { upload_url, public_url })
    }

    /// PUT /api/admin/lab-combos/:id/image-url/:kind
    /// Saves URL for hero, process, or detail image.
    pub async fn save_typed_image_url(&self, id: Uuid, kind: &str, url: String) -> AppResult<LabComboPage> {
        let column = match kind {
            "process" => "process_image_url",
            "detail" => "detail_image_url",
            _ => "image_url", // "hero" or default
        };

        // Dynamic column — safe because we whitelist above
        let sql = format!(
            r#"
            UPDATE lab_combo_pages SET {} = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
                smart_response, faq, status, quality_score,
                published_at::text, created_at::text, updated_at::text
            "#,
            column,
        );

        let page = sqlx::query_as::<_, LabComboPage>(&sql)
            .bind(&url)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("Lab combo not found"))?;

        Ok(page)
    }
}

// ── AI SEO Enrichment ────────────────────────────────────────────────────────

/// Rewrite template-based SEO text into unique, Gemini-generated copy.
/// Called asynchronously after combo creation. Updates DB in place.
///
/// KEY RULES: Write as a RECIPE, not an analysis. Include real numbers.
async fn enrich_seo_with_ai(
    pool: &PgPool,
    llm: &LlmAdapter,
    combo_id: Uuid,
    ingredients: &[String],
    locale: &str,
    goal: Option<&str>,
    meal_type: Option<&str>,
) -> AppResult<()> {
    let names = ingredients.join(", ");
    let goal_text = goal.map(|g| g.replace('_', " ")).unwrap_or_default();
    let meal_text = meal_type.unwrap_or("any meal");

    let lang = match locale {
        "ru" => "Russian",
        "pl" => "Polish",
        "uk" => "Ukrainian",
        _ => "English",
    };

    let prompt = format!(
        r#"You are a professional chef writing a recipe page for SEO.

Ingredients: {names}
Goal: {goal_text}
Meal: {meal_text}
Language: {lang} (write ALL fields in {lang})

Return ONLY valid JSON (no markdown fences, no extra text):
{{
  "title": "...",
  "description": "...",
  "h1": "...",
  "intro": "...",
  "why_it_works": "...",
  "how_to_cook": [
    {{"step": 1, "text": "...", "time_minutes": N}},
    {{"step": 2, "text": "...", "time_minutes": N}},
    {{"step": 3, "text": "...", "time_minutes": N}},
    {{"step": 4, "text": "...", "time_minutes": N}}
  ]
}}

STRICT RULES for each field:

title (max 55 chars):
- Format: "[Dish Name] ([Number]g Protein, [Number] Min)"
- MUST end with parentheses containing 2 numbers: protein + time
- Examples: "Salmon Rice Bowl (28g Protein, 15 Min)", "Chicken Broccoli Dinner (32g Protein, 20 Min)"
- NEVER use "analysis", "combo", "combination", "recipe" in title
- NEVER use " | " or " — " as separator, use parentheses

description (120-150 chars):
- Start with action verb: "Make", "Try", "Cook"
- Include one specific number
- End with benefit for the reader
- Example: "Make this salmon rice bowl in 15 minutes. 28g protein per serving — perfect for a post-workout meal."

h1 (40-70 chars):
- Must be different from title, LONGER than title
- Write as recipe name: "[Ingredients] [Dish Type] — [Key Benefit] Recipe"
- Example: "Salmon Avocado Rice Bowl — Healthy Breakfast Recipe"
- NEVER start with "Analysis:" or "Combo:"

intro (150-250 chars):
- CRITICAL: First sentence = direct answer with numbers. Format: "This [dish] delivers ~[N]g protein and is ready in [N] minutes."
- Second sentence adds ONE specific culinary fact (nutrient, technique, or flavor science)
- This targets Google featured snippets — the first sentence MUST standalone as a complete answer
- NO filler words: "delicious", "amazing", "perfect", "comprehensive"
- Write as a real chef, not as AI

why_it_works (200-400 chars):
- Explain CULINARY SCIENCE: which nutrient each ingredient provides
- Name specific compounds: omega-3, leucine, beta-carotene, resistant starch
- Explain flavor pairing logic: umami + acid, fat + crunch, sweet + salt
- Professional chef tone, NOT marketing copy

how_to_cook (array of 3-5 steps):
- Each step MUST be a concrete cooking action with time
- Include exact grams from ingredients
- BAD: "Prepare ingredients", "Cook base"
- GOOD: "Pan-sear salmon fillet (150g) for 4-5 min per side over medium-high heat"
- GOOD: "Cook rice (100g) in 200ml water for 12 min, then rest 5 min covered"
- GOOD: "Slice avocado (80g) and fan out on plate"

ABSOLUTE PROHIBITIONS:
- Do NOT use words: "analysis", "analyze", "combo", "combination", "comprehensive", "detailed"
- Do NOT write generic phrases: "prepare ingredients", "cook to preference", "season to taste"
- Every cooking step must specify: ingredient name, weight in grams, time in minutes, cooking method
- Every number must be realistic (not 0.0g protein)
- Title MUST have parentheses with (Xg Protein, Y Min) format"#
    );

    let raw_response = llm.groq_raw_request_with_model(&prompt, 3000, "gemini-3-flash-preview").await?;

    // Parse JSON from response (strip markdown fences if present)
    let cleaned = raw_response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let enriched: serde_json::Value = serde_json::from_str(cleaned)
        .or_else(|_| {
            // Try to find JSON in the response
            if let Some(start) = raw_response.find('{') {
                if let Some(end) = raw_response.rfind('}') {
                    return serde_json::from_str(&raw_response[start..=end]);
                }
            }
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No JSON found in AI response",
            )))
        })
        .map_err(|e| {
            tracing::warn!("Failed to parse AI enrichment response: {} — raw: {}", e, &raw_response[..raw_response.len().min(500)]);
            AppError::internal("AI enrichment parse error")
        })?;

    let title = enriched.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
    let description = enriched.get("description").and_then(|v| v.as_str()).unwrap_or("").trim();
    let h1 = enriched.get("h1").and_then(|v| v.as_str()).unwrap_or("").trim();
    let intro = enriched.get("intro").and_then(|v| v.as_str()).unwrap_or("").trim();
    let why_it_works = enriched.get("why_it_works").and_then(|v| v.as_str()).unwrap_or("").trim();

    // AI-generated cooking steps (overwrite template steps)
    let ai_steps = enriched.get("how_to_cook").cloned();

    // Only update non-empty fields
    if title.is_empty() && description.is_empty() {
        tracing::warn!("AI enrichment returned empty fields for combo {}", combo_id);
        return Ok(());
    }

    // Update text fields + cooking steps if AI provided them
    if let Some(steps) = ai_steps {
        if steps.is_array() && !steps.as_array().unwrap().is_empty() {
            sqlx::query(
                r#"UPDATE lab_combo_pages SET
                    title = CASE WHEN $1 != '' THEN $1 ELSE title END,
                    description = CASE WHEN $2 != '' THEN $2 ELSE description END,
                    h1 = CASE WHEN $3 != '' THEN $3 ELSE h1 END,
                    intro = CASE WHEN $4 != '' THEN $4 ELSE intro END,
                    why_it_works = CASE WHEN $5 != '' THEN $5 ELSE why_it_works END,
                    how_to_cook = $7,
                    updated_at = NOW()
                WHERE id = $6"#,
            )
            .bind(title)
            .bind(description)
            .bind(h1)
            .bind(intro)
            .bind(why_it_works)
            .bind(combo_id)
            .bind(&steps)
            .execute(pool)
            .await?;

            tracing::info!("✅ AI-enriched SEO + cooking steps for combo {}", combo_id);
            return Ok(());
        }
    }

    // Fallback: update only text fields
    sqlx::query(
        r#"UPDATE lab_combo_pages SET
            title = CASE WHEN $1 != '' THEN $1 ELSE title END,
            description = CASE WHEN $2 != '' THEN $2 ELSE description END,
            h1 = CASE WHEN $3 != '' THEN $3 ELSE h1 END,
            intro = CASE WHEN $4 != '' THEN $4 ELSE intro END,
            why_it_works = CASE WHEN $5 != '' THEN $5 ELSE why_it_works END,
            updated_at = NOW()
        WHERE id = $6"#,
    )
    .bind(title)
    .bind(description)
    .bind(h1)
    .bind(intro)
    .bind(why_it_works)
    .bind(combo_id)
    .execute(pool)
    .await?;

    tracing::info!("✅ AI-enriched SEO content for combo {}", combo_id);
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn capitalize_words(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn smart_truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_len - 1).collect();
    if let Some(pos) = truncated.rfind(' ') {
        format!("{}…", &truncated[..pos])
    } else {
        format!("{}…", truncated)
    }
}
