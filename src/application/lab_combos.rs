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
    // ── Pre-calculated nutrition (single source of truth) ────────────
    pub total_weight_g: f32,
    pub servings_count: i16,
    pub calories_total: f32,
    pub protein_total: f32,
    pub fat_total: f32,
    pub carbs_total: f32,
    pub fiber_total: f32,
    pub calories_per_serving: f32,
    pub protein_per_serving: f32,
    pub fat_per_serving: f32,
    pub carbs_per_serving: f32,
    pub fiber_per_serving: f32,
    // ── Structured ingredients (DB data, not AI) ─────────────────────
    pub structured_ingredients: serde_json::Value,
    // ─────────────────────────────────────────────────────────────────
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
    /// AI model override: "flash" (fast, default) or "pro" (smart, better quality).
    /// "pro" = gemini-3.1-pro-preview — recommended for final SEO pages.
    /// "flash" = gemini-3-flash-preview — good for drafts/testing.
    #[serde(default)]
    pub model: Option<String>,
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
/// Recipe-oriented, always includes REAL protein + time.
/// Format: "Salmon Rice Bowl (34g Protein, 15 Min)"
fn generate_title(ingredients: &[String], goal: Option<&str>, meal_type: Option<&str>, _locale: &str, nt: &NutritionTotals) -> String {
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

    // Use pre-calculated protein from NutritionTotals (single source of truth)
    let est_protein = nt.protein_per_serving.round() as i64;

    let hook = if est_protein > 5 {
        match goal {
            Some(g) if g.contains("loss") || g.contains("low_cal") => format!("({}g Protein, Low Cal)", est_protein),
            Some(g) if g.contains("keto") => format!("({}g Protein, Keto)", est_protein),
            _ => format!("({}g Protein, 15 Min)", est_protein),
        }
    } else {
        "(15 Min)".to_string()
    };

    smart_truncate(&format!("{} {}", dish, hook), 60)
}

/// Auto-generate SEO description (80–155 chars). Recipe-oriented.
fn generate_description(ingredients: &[String], goal: Option<&str>, locale: &str, nt: &NutritionTotals) -> String {
    let names = ingredients.join(", ");
    let est_protein = nt.protein_per_serving.round() as i64;
    let goal_text = goal
        .map(|g| format!(" for {}", g.replace('_', " ")))
        .unwrap_or_default();

    let protein_hint = if est_protein > 5 {
        format!(" {}g protein per serving.", est_protein)
    } else {
        String::new()
    };

    let desc = match locale {
        "ru" => format!(
            "Рецепт из {names}{goal_text} за 15 мин.{} Пошаговая инструкция и КБЖУ.",
            if est_protein > 5 { format!(" {}г белка на порцию.", est_protein) } else { String::new() }
        ),
        "pl" => format!(
            "Przepis z {names}{goal_text} w 15 min.{} Instrukcja krok po kroku i KBJU.",
            if est_protein > 5 { format!(" {}g białka na porcję.", est_protein) } else { String::new() }
        ),
        "uk" => format!(
            "Рецепт з {names}{goal_text} за 15 хв.{} Покрокова інструкція та КБЖУ.",
            if est_protein > 5 { format!(" {}г білка на порцію.", est_protein) } else { String::new() }
        ),
        _ => format!(
            "Quick recipe with {names}{goal_text}.{protein_hint} Step-by-step instructions and macros."
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
/// RULE: First sentence = direct answer with REAL protein + calorie numbers.
/// This targets Google featured snippets.
fn generate_intro(ingredients: &[String], goal: Option<&str>, locale: &str, nt: &NutritionTotals) -> String {
    let names = ingredients.join(", ");
    let est_protein = nt.protein_per_serving.round() as i64;
    let est_calories = nt.calories_per_serving.round() as i64;

    let protein_text = if est_protein > 5 {
        match locale {
            "ru" => format!(" ~{}г белка,", est_protein),
            "pl" => format!(" ~{}g białka,", est_protein),
            "uk" => format!(" ~{}г білка,", est_protein),
            _    => format!(" ~{}g protein,", est_protein),
        }
    } else {
        String::new()
    };

    match locale {
        "ru" => format!(
            "Это блюдо из {names} содержит{protein_text} ~{est_calories} ккал на порцию и готовится за 15–20 минут. \
             Ниже — пошаговый рецепт с точными граммовками и КБЖУ на порцию.",
        ),
        "pl" => format!(
            "To danie z {names} zawiera{protein_text} ~{est_calories} kcal na porcję i przygotujesz je w 15–20 minut. \
             Poniżej — przepis krok po kroku z dokładnymi gramówkami i KBJU na porcję.",
        ),
        "uk" => format!(
            "Ця страва з {names} містить{protein_text} ~{est_calories} ккал на порцію і готується за 15–20 хвилин. \
             Нижче — покроковий рецепт із точними грамовками та КБЖУ на порцію.",
        ),
        _ => format!(
            "This {names} dish delivers{protein_text} ~{est_calories} kcal per serving and is ready in 15–20 minutes. \
             Below: step-by-step recipe with exact portions and macros per serving.",
        ),
    }
}

/// Auto-generate FAQ from SmartResponse data. Recipe-oriented questions.
fn generate_faq(
    ingredients: &[String],
    smart: &serde_json::Value,
    locale: &str,
    nt: &NutritionTotals,
) -> serde_json::Value {
    let names = ingredients.join(", ");
    let mut faq = Vec::new();

    // Q1: How many calories/protein per serving? — uses NutritionTotals (single source of truth)
    {
        let serving_kcal = nt.calories_per_serving.round() as i64;
        let serving_prot = nt.protein_per_serving.round() as i64;
        let serving_weight = nt.total_weight_g.round() as i64;
        let q = match locale {
            "ru" => format!("Сколько калорий и белка в блюде из {}?", names),
            "pl" => format!("Ile kalorii i białka ma danie z {}?", names),
            "uk" => format!("Скільки калорій і білка у страві з {}?", names),
            _    => format!("How many calories and protein in a {} dish?", names),
        };
        let a = match locale {
            "ru" => format!("Примерно {} ккал и {} г белка на порцию (~{} г).", serving_kcal, serving_prot, serving_weight),
            "pl" => format!("Około {} kcal i {} g białka na porcję (~{} g).", serving_kcal, serving_prot, serving_weight),
            "uk" => format!("Приблизно {} ккал і {} г білка на порцію (~{} г).", serving_kcal, serving_prot, serving_weight),
            _    => format!("Approximately {} kcal and {}g protein per serving (~{}g).", serving_kcal, serving_prot, serving_weight),
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
    nt: &NutritionTotals,
) -> String {
    let names = ingredients.join(", ");

    // Use pre-calculated nutrition (single source of truth)
    let protein = nt.protein_per_serving;
    let calories = nt.calories_per_serving;
    let fiber = nt.fiber_per_serving;

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

    // Part 1: Nutritional reason (uses pre-calculated per-serving values)
    match locale {
        "ru" => {
            if protein > 15.0 {
                parts.push(format!("Эта комбинация содержит {:.0} г белка на порцию — отличный источник протеина", protein));
            } else {
                parts.push(format!("Комбинация {names} даёт {:.0} ккал на порцию", calories));
            }
        }
        "pl" => {
            if protein > 15.0 {
                parts.push(format!("Ta kombinacja zawiera {:.0} g białka na porcję — świetne źródło proteiny", protein));
            } else {
                parts.push(format!("Kombinacja {names} dostarcza {:.0} kcal na porcję", calories));
            }
        }
        "uk" => {
            if protein > 15.0 {
                parts.push(format!("Ця комбінація містить {:.0} г білка на порцію — чудове джерело протеїну", protein));
            } else {
                parts.push(format!("Комбінація {names} дає {:.0} ккал на порцію", calories));
            }
        }
        _ => {
            if protein > 15.0 {
                parts.push(format!("This combination provides {:.0}g of protein per serving — an excellent protein source", protein));
            } else {
                parts.push(format!("The combination of {names} delivers {:.0} kcal per serving", calories));
            }
        }
    }

    // Part 2: Fiber bonus
    if fiber > 3.0 {
        let fiber_note = match locale {
            "ru" => format!("Содержит {:.1} г клетчатки, что поддерживает пищеварение", fiber),
            "pl" => format!("Zawiera {:.1} g błonnika, co wspiera trawienie", fiber),
            "uk" => format!("Містить {:.1} г клітковини, що підтримує травлення", fiber),
            _    => format!("Contains {:.1}g of fiber, supporting healthy digestion", fiber),
        };
        parts.push(fiber_note);
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

    // Raw-only ingredients that should NEVER be cooked
    let raw_only = ["avocado", "lettuce", "arugula", "cucumber", "basil", "cilantro",
                     "parsley", "dill", "mint", "lemon", "lime"];
    // Grains/starches that need boiling
    let grains = ["rice", "pasta", "quinoa", "bulgur", "couscous", "oats", "noodle", "noodles"];
    // Proteins that must be cooked
    let proteins = ["salmon", "chicken", "beef", "pork", "tuna", "cod", "shrimp", "prawn",
                     "turkey", "lamb", "duck", "egg", "eggs", "tofu"];

    if let Some(variant) = reference_variant {
        let variant_ingredients = variant.get("ingredients").and_then(|i| i.as_array());

        if let Some(vi) = variant_ingredients {
            // Classify ingredients by cooking method, NOT by variant role
            let mut grain_items: Vec<(&str, f64)> = Vec::new();
            let mut protein_items: Vec<(&str, f64)> = Vec::new();
            let mut raw_items: Vec<(&str, f64)> = Vec::new();
            let mut other_cook_items: Vec<(&str, f64)> = Vec::new();

            for ing in vi.iter() {
                let name = ing.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let grams = ing.get("grams").and_then(|g| g.as_f64()).unwrap_or(100.0);
                let name_lower = name.to_lowercase();

                if raw_only.iter().any(|r| name_lower.contains(r)) {
                    raw_items.push((name, grams));
                } else if grains.iter().any(|g| name_lower.contains(g)) {
                    grain_items.push((name, grams));
                } else if proteins.iter().any(|p| name_lower.contains(p)) {
                    protein_items.push((name, grams));
                } else {
                    // Vegetables like broccoli, spinach (to cook), peppers, etc.
                    other_cook_items.push((name, grams));
                }
            }

            let mut step_num = 1;

            // Step 1: Cook grains (they take longest — start first)
            if !grain_items.is_empty() {
                let details: Vec<String> = grain_items.iter()
                    .map(|(name, grams)| format!("{name} ({grams:.0}g)"))
                    .collect();
                let step_text = match locale {
                    "ru" => format!("Сварите {} в подсоленной воде (соотношение 2:1) 12–15 мин. Снимите с огня, накройте и оставьте на 5 мин.", details.join(", ")),
                    "pl" => format!("Ugotuj {} w osolonej wodzie (proporcja 2:1) 12–15 min. Zdejmij z ognia, przykryj i zostaw na 5 min.", details.join(", ")),
                    "uk" => format!("Зваріть {} у підсоленій воді (співвідношення 2:1) 12–15 хв. Зніміть з вогню, накрийте і залиште на 5 хв.", details.join(", ")),
                    _    => format!("Boil {} in salted water (2:1 ratio) for 12–15 min. Remove from heat, cover, and let rest 5 min.", details.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 15
                }));
                step_num += 1;
            }

            // Step 2: Cook protein
            if !protein_items.is_empty() {
                let details: Vec<String> = protein_items.iter()
                    .map(|(name, grams)| format!("{name} ({grams:.0}g)"))
                    .collect();
                let name_lower = protein_items[0].0.to_lowercase();
                let (method_en, method_ru, method_pl, method_uk, time) =
                    if name_lower.contains("egg") {
                        ("Fry eggs in a non-stick pan over medium heat", "Обжарьте яйца на сковороде с антипригарным покрытием на среднем огне", "Usmaż jajka na patelni z powłoką nieprzywierającą na średnim ogniu", "Обсмажте яйця на сковороді з антипригарним покриттям на середньому вогні", 4)
                    } else if name_lower.contains("shrimp") || name_lower.contains("prawn") {
                        ("Sauté shrimp in olive oil over high heat until pink", "Обжарьте креветки в оливковом масле на сильном огне до розового цвета", "Usmaż krewetki na oliwie z oliwek na dużym ogniu do różowego koloru", "Обсмажте креветки в оливковій олії на сильному вогні до рожевого кольору", 4)
                    } else {
                        ("Pan-sear over medium-high heat, 4–5 min per side until golden", "Обжарьте на среднем-сильном огне 4–5 мин с каждой стороны до золотистой корочки", "Usmaż na średnio-dużym ogniu 4–5 min z każdej strony do złotego koloru", "Обсмажте на середньо-сильному вогні 4–5 хв з кожного боку до золотистої скоринки", 10)
                    };
                let step_text = match locale {
                    "ru" => format!("{} {}.", method_ru, details.join(", ")),
                    "pl" => format!("{} {}.", method_pl, details.join(", ")),
                    "uk" => format!("{} {}.", method_uk, details.join(", ")),
                    _    => format!("{} {}.", method_en, details.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": time
                }));
                step_num += 1;
            }

            // Step 3: Cook other vegetables (if any)
            if !other_cook_items.is_empty() {
                let details: Vec<String> = other_cook_items.iter()
                    .map(|(name, grams)| format!("{name} ({grams:.0}g)"))
                    .collect();
                let step_text = match locale {
                    "ru" => format!("Обжарьте {} на среднем огне 3–4 мин, помешивая.", details.join(", ")),
                    "pl" => format!("Usmaż {} na średnim ogniu 3–4 min, mieszając.", details.join(", ")),
                    "uk" => format!("Обсмажте {} на середньому вогні 3–4 хв, помішуючи.", details.join(", ")),
                    _    => format!("Sauté {} over medium heat for 3–4 min, stirring.", details.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 4
                }));
                step_num += 1;
            }

            // Step 4: Prepare raw ingredients (slice, arrange — NO cooking)
            if !raw_items.is_empty() {
                let details: Vec<String> = raw_items.iter()
                    .map(|(name, grams)| format!("{name} ({grams:.0}g)"))
                    .collect();
                let step_text = match locale {
                    "ru" => format!("Нарежьте {} и выложите в тарелку.", details.join(", ")),
                    "pl" => format!("Pokrój {} i ułóż na talerzu.", details.join(", ")),
                    "uk" => format!("Наріжте {} та викладіть на тарілку.", details.join(", ")),
                    _    => format!("Slice {} and arrange on the plate.", details.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 2
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
                "ru" => format!("Соберите блюдо: выложите все компоненты на тарелку и подавайте. Порция: ~{total_cal} ккал, ~{total_prot:.0} г белка."),
                "pl" => format!("Złóż danie: ułóż wszystkie składniki na talerzu i podaj. Porcja: ~{total_cal} kcal, ~{total_prot:.0} g białka."),
                "uk" => format!("Зберіть страву: викладіть усі компоненти на тарілку та подавайте. Порція: ~{total_cal} ккал, ~{total_prot:.0} г білка."),
                _    => format!("Assemble: arrange all components on the plate and serve. Per serving: ~{total_cal} kcal, ~{total_prot:.0}g protein."),
            };
            steps.push(serde_json::json!({
                "step": step_num,
                "text": assemble,
                "time_minutes": 2
            }));
        }
    }

    // Fallback: ingredient-aware steps if no variant data
    if steps.is_empty() {
        let mut step_num = 1;
        let mut fallback_steps: Vec<serde_json::Value> = Vec::new();

        // Separate ingredients by type
        let mut has_grain = false;
        let mut has_protein = false;

        for ing in ingredients {
            let ing_lower = ing.to_lowercase();
            if grains.iter().any(|g| ing_lower.contains(g)) && !has_grain {
                has_grain = true;
                let step_text = match locale {
                    "ru" => format!("Сварите {} (100 г) в подсоленной воде 12–15 мин.", ing),
                    "pl" => format!("Ugotuj {} (100 g) w osolonej wodzie 12–15 min.", ing),
                    "uk" => format!("Зваріть {} (100 г) у підсоленій воді 12–15 хв.", ing),
                    _    => format!("Boil {} (100g) in salted water for 12–15 min.", ing),
                };
                fallback_steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 15 }));
                step_num += 1;
            } else if proteins.iter().any(|p| ing_lower.contains(p)) && !has_protein {
                has_protein = true;
                let step_text = match locale {
                    "ru" => format!("Обжарьте {} (150 г) на среднем-сильном огне 4–5 мин с каждой стороны.", ing),
                    "pl" => format!("Usmaż {} (150 g) na średnio-dużym ogniu 4–5 min z każdej strony.", ing),
                    "uk" => format!("Обсмажте {} (150 г) на середньо-сильному вогні 4–5 хв з кожного боку.", ing),
                    _    => format!("Pan-sear {} (150g) over medium-high heat, 4–5 min per side.", ing),
                };
                fallback_steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 10 }));
                step_num += 1;
            } else if raw_only.iter().any(|r| ing_lower.contains(r)) {
                let step_text = match locale {
                    "ru" => format!("Нарежьте {} (80 г) и отложите.", ing),
                    "pl" => format!("Pokrój {} (80 g) i odłóż.", ing),
                    "uk" => format!("Наріжте {} (80 г) та відкладіть.", ing),
                    _    => format!("Slice {} (80g) and set aside.", ing),
                };
                fallback_steps.push(serde_json::json!({ "step": step_num, "text": step_text, "time_minutes": 2 }));
                step_num += 1;
            }
        }

        // Final assemble step
        let assemble = match locale {
            "ru" => "Соберите блюдо: выложите все компоненты на тарелку и подавайте.".to_string(),
            "pl" => "Złóż danie: ułóż wszystkie składniki na talerzu i podaj.".to_string(),
            "uk" => "Зберіть страву: викладіть усі компоненти на тарілку та подавайте.".to_string(),
            _    => "Assemble: arrange all components on the plate and serve.".to_string(),
        };
        fallback_steps.push(serde_json::json!({ "step": step_num, "text": assemble, "time_minutes": 2 }));

        steps = fallback_steps;
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

    // ── Resolve ingredient names (any language) → English slugs ──────────

    /// Accept ingredient names in any language (ru, pl, uk, en) or slugs,
    /// and resolve them to canonical English slugs from catalog_ingredients.
    /// Falls back to the input text (lowercased, spaces→dashes) if not found.
    pub async fn resolve_ingredient_slugs(&self, inputs: &[String]) -> AppResult<Vec<String>> {
        let mut slugs = Vec::with_capacity(inputs.len());

        for input in inputs {
            let normalized = input.trim().to_lowercase();

            // Try exact slug match first
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT slug FROM catalog_ingredients WHERE slug = $1 AND is_active = true LIMIT 1"
            )
                .bind(&normalized)
                .fetch_optional(&self.pool)
                .await?;

            if let Some((slug,)) = row {
                slugs.push(slug);
                continue;
            }

            // Try match by name in any language (case-insensitive)
            let row: Option<(String,)> = sqlx::query_as(
                r#"SELECT slug FROM catalog_ingredients
                   WHERE is_active = true
                     AND (
                       LOWER(name_en) = $1
                       OR LOWER(name_ru) = $1
                       OR LOWER(name_pl) = $1
                       OR LOWER(name_uk) = $1
                     )
                   LIMIT 1"#
            )
                .bind(&normalized)
                .fetch_optional(&self.pool)
                .await?;

            if let Some((slug,)) = row {
                slugs.push(slug);
                continue;
            }

            // Fallback: use input as slug (spaces → dashes)
            let fallback = normalized.replace(' ', "-");
            tracing::warn!("⚠️ Ingredient '{}' not found in catalog, using as slug: {}", input, fallback);
            slugs.push(fallback);
        }

        Ok(slugs)
    }

    // ── Build structured ingredients from catalog DB ─────────────────────

    /// Query catalog_ingredients for each slug, return a JSONB array:
    /// [{ slug, name, grams, kcal, protein, fat, carbs, image_url, product_type }]
    /// Name is localized. Grams come from default_portion_grams().
    /// Falls back to hardcoded nutrition if slug not found in catalog.
    async fn build_structured_ingredients(
        &self,
        slugs: &[String],
        locale: &str,
    ) -> AppResult<serde_json::Value> {
        let mut items = Vec::new();

        for slug in slugs {
            // Query catalog_ingredients for this slug
            let row: Option<(
                String,                // slug
                Option<String>,        // name_en
                Option<String>,        // name_ru
                Option<String>,        // name_pl
                Option<String>,        // name_uk
                Option<String>,        // image_url
                Option<String>,        // product_type
                Option<f32>,           // calories_per_100g
                Option<f32>,           // protein_per_100g
                Option<f32>,           // fat_per_100g
                Option<f32>,           // carbs_per_100g
            )> = sqlx::query_as(
                r#"SELECT slug, name_en, name_ru, name_pl, name_uk,
                          image_url, product_type,
                          calories_per_100g::REAL, protein_per_100g::REAL,
                          fat_per_100g::REAL, carbs_per_100g::REAL
                   FROM catalog_ingredients
                   WHERE slug = $1 AND is_active = true
                   LIMIT 1"#,
            )
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;

            let portion = default_portion_grams(slug);

            if let Some((s, name_en, name_ru, name_pl, name_uk, img, pt, cal100, prot100, fat100, carb100)) = row {
                // Pick localized name
                let name = match locale {
                    "ru" => name_ru.as_deref().or(name_en.as_deref()).unwrap_or(&s),
                    "pl" => name_pl.as_deref().or(name_en.as_deref()).unwrap_or(&s),
                    "uk" => name_uk.as_deref().or(name_en.as_deref()).unwrap_or(&s),
                    _    => name_en.as_deref().unwrap_or(&s),
                };

                let cal = cal100.unwrap_or(0.0) as f64;
                let prot = prot100.unwrap_or(0.0) as f64;
                let fat = fat100.unwrap_or(0.0) as f64;
                let carb = carb100.unwrap_or(0.0) as f64;

                items.push(serde_json::json!({
                    "slug": s,
                    "name": name,
                    "grams": portion,
                    "kcal": (cal * portion / 100.0).round(),
                    "protein": ((prot * portion / 100.0) * 10.0).round() / 10.0,
                    "fat": ((fat * portion / 100.0) * 10.0).round() / 10.0,
                    "carbs": ((carb * portion / 100.0) * 10.0).round() / 10.0,
                    "image_url": img,
                    "product_type": pt,
                }));
            } else {
                // Fallback: use hardcoded nutrition data
                let (cal100, prot100, fat100, carb100, _fiber) = nutrition_per_100g(slug);
                let name = capitalize_words(&slug.replace('-', " "));

                items.push(serde_json::json!({
                    "slug": slug,
                    "name": name,
                    "grams": portion,
                    "kcal": (cal100 * portion / 100.0).round(),
                    "protein": ((prot100 * portion / 100.0) * 10.0).round() / 10.0,
                    "fat": ((fat100 * portion / 100.0) * 10.0).round() / 10.0,
                    "carbs": ((carb100 * portion / 100.0) * 10.0).round() / 10.0,
                    "image_url": null,
                    "product_type": null,
                }));
            }
        }

        Ok(serde_json::json!(items))
    }

    // ── Backfill structured_ingredients for existing records ────────────

    /// Populate structured_ingredients for all lab_combo_pages that have an empty array.
    /// This is needed because the field was added after initial records were created.
    /// Returns the number of records updated.
    pub async fn backfill_structured_ingredients(&self) -> AppResult<usize> {
        // Fetch all pages with empty structured_ingredients
        let rows: Vec<(uuid::Uuid, Vec<String>, String)> = sqlx::query_as(
            r#"SELECT id, ingredients, locale
               FROM lab_combo_pages
               WHERE structured_ingredients = '[]'::jsonb
               ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = rows.len();
        tracing::info!("🔄 Backfilling structured_ingredients for {} records", total);

        let mut updated = 0;
        for (id, ingredients, locale) in rows {
            let structured = self.build_structured_ingredients(&ingredients, &locale).await?;

            sqlx::query(
                "UPDATE lab_combo_pages SET structured_ingredients = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(&structured)
            .bind(id)
            .execute(&self.pool)
            .await?;

            updated += 1;
            tracing::info!("  ✅ Backfilled {}/{} — id={}", updated, total, id);
        }

        tracing::info!("✅ Backfill complete: {}/{} records updated", updated, total);
        Ok(updated)
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
        let mut smart_json = serde_json::to_value(&smart_result)
            .map_err(|e| AppError::internal(format!("failed to serialize SmartResponse: {}", e)))?;

        // ── Calculate nutrition (Single Source of Truth) ──────────────────
        let nt = calculate_nutrition(&ingredients);

        // ── NUTRITION SAFETY NET ─────────────────────────────────────────
        // If SmartService returned protein = 0 (AI hallucination), override with calculated estimate.
        // This is critical: "Salmon Rice Bowl — 0g protein" destroys SEO trust.
        {
            if let Some(nutrition) = smart_json.get_mut("nutrition") {
                let current_protein = nutrition.get("protein").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let current_calories = nutrition.get("calories").and_then(|v| v.as_f64()).unwrap_or(0.0);

                // Fix protein if 0 or unrealistically low (< 2g when we know there's protein)
                if current_protein < 2.0 && nt.protein_per_serving > 5.0 {
                    // Store per-100g (smart_response.nutrition is per 100g)
                    let protein_per_100g = nt.protein_per_serving / 3.0; // serving is ~300g
                    nutrition.as_object_mut().map(|n| {
                        n.insert("protein".to_string(), serde_json::json!(protein_per_100g));
                    });
                    tracing::warn!("⚠️ SmartService returned protein={:.1}g — overriding with {:.1}g/100g (est serving: {:.0}g)",
                        current_protein, protein_per_100g, nt.protein_per_serving);
                }

                // Fix calories if 0
                if current_calories < 10.0 && nt.calories_per_serving > 50.0 {
                    let calories_per_100g = nt.calories_per_serving / 3.0;
                    nutrition.as_object_mut().map(|n| {
                        n.insert("calories".to_string(), serde_json::json!(calories_per_100g));
                    });
                    tracing::warn!("⚠️ SmartService returned calories={:.0} — overriding with {:.0}/100g",
                        current_calories, calories_per_100g);
                }
            }
        }

        // Generate SEO metadata
        let title = generate_title(&ingredients, req.goal.as_deref(), req.meal_type.as_deref(), &req.locale, &nt);
        let description = generate_description(&ingredients, req.goal.as_deref(), &req.locale, &nt);
        let h1 = generate_h1(&ingredients, req.goal.as_deref(), req.meal_type.as_deref(), &req.locale);
        let intro = generate_intro(&ingredients, req.goal.as_deref(), &req.locale, &nt);
        let faq = generate_faq(&ingredients, &smart_json, &req.locale, &nt);
        let why_it_works = generate_why_it_works(&ingredients, &smart_json, req.goal.as_deref(), &req.locale, &nt);
        let how_to_cook = generate_how_to_cook(&ingredients, &smart_json, &req.locale);
        let optimization_tips = generate_optimization_tips(&smart_json, &req.locale);

        // ── Build structured ingredients from catalog_ingredients DB ──────
        let structured_ingredients = self.build_structured_ingredients(&ingredients, &req.locale).await?;

        let id = Uuid::new_v4();

        let page = sqlx::query_as::<_, LabComboPage>(
            r#"
            INSERT INTO lab_combo_pages (
                id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips,
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status
            ) VALUES (
                $1, $2, $3, $4,
                $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14,
                $15, $16, $17,
                $18, $19,
                $20, $21,
                $22, $23, $24, $25, $26,
                $27, $28, $29, $30, $31,
                $32,
                'draft'
            )
            RETURNING id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status, quality_score,
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
        .bind(nt.total_weight_g as f32)
        .bind(nt.servings_count)
        .bind(nt.calories_total as f32)
        .bind(nt.protein_total as f32)
        .bind(nt.fat_total as f32)
        .bind(nt.carbs_total as f32)
        .bind(nt.fiber_total as f32)
        .bind(nt.calories_per_serving as f32)
        .bind(nt.protein_per_serving as f32)
        .bind(nt.fat_per_serving as f32)
        .bind(nt.carbs_per_serving as f32)
        .bind(nt.fiber_per_serving as f32)
        .bind(&structured_ingredients)
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
        // Model selection: "pro" → gemini-3.1-pro-preview (better quality), "flash" → gemini-3-flash-preview (faster)
        let ai_model = match req.model.as_deref() {
            Some("pro") | Some("gemini-3.1-pro-preview") => "gemini-3.1-pro-preview",
            _ => "gemini-3-flash-preview",
        };
        let pool_bg = self.pool.clone();
        let llm_bg = self.llm_adapter.clone();
        let ingredients_bg = ingredients.clone();
        let locale_bg = req.locale.clone();
        let goal_bg = req.goal.clone();
        let meal_type_bg = req.meal_type.clone();
        let model_bg = ai_model.to_string();
        let nt_bg = nt.clone();
        tokio::spawn(async move {
            if let Err(e) = enrich_seo_with_ai(
                &pool_bg, &llm_bg, id,
                &ingredients_bg, &locale_bg,
                goal_bg.as_deref(), meal_type_bg.as_deref(),
                &model_bg, &nt_bg,
            ).await {
                tracing::warn!("⚠️ AI enrichment failed for combo {}: {}", id, e);
            }
        });

        Ok(page)
    }

    // ── Generate for ALL locales (Admin — like ingredients) ──────────────

    /// Generate a lab combo page for all 4 locales (en, pl, ru, uk) in one call.
    /// Accepts ingredient names in ANY language — auto-resolves to English slugs.
    /// Returns a vec of generated pages. Skips locales where the slug already exists.
    pub async fn generate_all_locales(&self, req: GenerateComboRequest) -> AppResult<Vec<LabComboPage>> {
        const LOCALES: [&str; 4] = ["en", "pl", "ru", "uk"];

        // ── Step 1: Resolve ingredient names (any language) → English slugs
        let resolved_slugs = self.resolve_ingredient_slugs(&req.ingredients).await?;
        tracing::info!(
            "🔄 Resolved ingredients: {:?} → {:?}",
            req.ingredients, resolved_slugs
        );

        let mut pages = Vec::new();

        for locale in LOCALES {
            let locale_req = GenerateComboRequest {
                ingredients: resolved_slugs.clone(),
                locale: locale.to_string(),
                goal: req.goal.clone(),
                meal_type: req.meal_type.clone(),
                diet: req.diet.clone(),
                cooking_time: req.cooking_time.clone(),
                budget: req.budget.clone(),
                cuisine: req.cuisine.clone(),
                model: req.model.clone(),
            };

            match self.generate(locale_req).await {
                Ok(page) => {
                    tracing::info!("✅ Generated lab combo [{}] locale={}", page.slug, locale);
                    pages.push(page);
                }
                Err(e) => {
                    // Skip if already exists, propagate other errors
                    let msg = format!("{}", e);
                    if msg.contains("already exists") {
                        tracing::info!("⏭️ Lab combo already exists for locale={}, skipping", locale);
                    } else {
                        tracing::error!("❌ Failed to generate lab combo locale={}: {}", locale, e);
                        return Err(e);
                    }
                }
            }
        }

        Ok(pages)
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
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status, quality_score,
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
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status, quality_score,
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
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status, quality_score,
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
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status, quality_score,
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
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status, quality_score,
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

            // Skip if already exists in ALL 4 locales
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM lab_combo_pages WHERE slug = $1 AND locale IN ('en','pl','ru','uk')"
            )
                .bind(&slug)
                .fetch_one(&self.pool)
                .await?;

            if count.0 >= 4 {
                continue; // All 4 locales already exist
            }

            match self.generate_all_locales(GenerateComboRequest {
                ingredients: ings.iter().map(|s| s.to_string()).collect(),
                locale: "en".to_string(), // ignored — generate_all_locales iterates all 4
                goal: goal.map(String::from),
                meal_type: meal.map(String::from),
                diet: None,
                cooking_time: None,
                budget: None,
                cuisine: None,
                model: Some("pro".to_string()),
            }).await {
                Ok(pages) => {
                    let locales: Vec<String> = pages.iter().map(|p| p.locale.clone()).collect();
                    generated.push(format!("✅ {} [{}]", slug, locales.join(",")));
                }
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
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status, quality_score,
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

// ── Nutrition Calculator (Single Source of Truth) ────────────────────────────

/// Per-100g USDA-based nutrition data for common ingredients.
/// (kcal, protein, fat, carbs, fiber)
fn nutrition_per_100g(ingredient: &str) -> (f64, f64, f64, f64, f64) {
    let name = ingredient.to_lowercase();
    // (kcal, protein, fat, carbs, fiber)
    // Fish & seafood
    if name.contains("salmon")   { return (208.0, 20.0, 13.0,  0.0, 0.0); }
    if name.contains("tuna")     { return (132.0, 23.0,  5.0,  0.0, 0.0); }
    if name.contains("cod")      { return ( 82.0, 17.0,  0.7,  0.0, 0.0); }
    if name.contains("shrimp") || name.contains("prawn") { return ( 99.0, 24.0,  0.3,  0.2, 0.0); }
    if name.contains("mackerel") { return (205.0, 19.0, 14.0,  0.0, 0.0); }
    if name.contains("trout")    { return (148.0, 20.0,  7.0,  0.0, 0.0); }
    if name.contains("sardine")  { return (208.0, 21.0, 11.0,  0.0, 0.0); }
    // Poultry & meat
    if name.contains("chicken")  { return (165.0, 31.0,  3.6,  0.0, 0.0); }
    if name.contains("turkey")   { return (157.0, 29.0,  3.2,  0.0, 0.0); }
    if name.contains("beef")     { return (250.0, 26.0, 15.0,  0.0, 0.0); }
    if name.contains("pork")     { return (242.0, 25.0, 14.0,  0.0, 0.0); }
    if name.contains("lamb")     { return (258.0, 25.0, 17.0,  0.0, 0.0); }
    if name.contains("duck")     { return (201.0, 19.0, 14.0,  0.0, 0.0); }
    // Eggs & dairy
    if name.contains("egg")      { return (155.0, 13.0, 11.0,  1.1, 0.0); }
    if name.contains("cheese")   { return (350.0, 22.0, 28.0,  1.3, 0.0); }
    if name.contains("yogurt") || name.contains("yoghurt") { return ( 59.0,  5.0,  0.4,  3.6, 0.0); }
    // Legumes & plant protein
    if name.contains("tofu")     { return ( 76.0,  8.0,  4.8,  1.9, 0.3); }
    if name.contains("tempeh")   { return (193.0, 19.0, 11.0,  9.4, 0.0); }
    if name.contains("lentil")   { return (116.0,  9.0,  0.4, 20.0, 7.9); }
    if name.contains("chickpea") { return (164.0,  8.5,  2.6, 27.0, 7.6); }
    if name.contains("bean")     { return (127.0,  8.0,  0.5, 22.0, 7.4); }
    // Grains (cooked values)
    if name.contains("quinoa")   { return (120.0,  4.4,  1.9, 21.0, 2.8); }
    if name.contains("rice")     { return (130.0,  2.7,  0.3, 28.0, 0.4); }
    if name.contains("pasta") || name.contains("noodle") { return (131.0,  5.0,  1.1, 25.0, 1.8); }
    if name.contains("oat")      { return ( 68.0,  2.4,  1.4, 12.0, 1.7); }
    if name.contains("bread")    { return (265.0,  9.0,  3.2, 49.0, 2.7); }
    if name.contains("potato")   { return ( 77.0,  2.0,  0.1, 17.0, 2.2); }
    // Vegetables
    if name.contains("broccoli") { return ( 34.0,  2.8,  0.4,  7.0, 2.6); }
    if name.contains("spinach")  { return ( 23.0,  2.9,  0.4,  3.6, 2.2); }
    if name.contains("tomato")   { return ( 18.0,  0.9,  0.2,  3.9, 1.2); }
    if name.contains("cucumber") { return ( 15.0,  0.7,  0.1,  3.6, 0.5); }
    if name.contains("pepper") || name.contains("paprika") { return ( 31.0,  1.0,  0.3,  6.0, 2.1); }
    if name.contains("onion")    { return ( 40.0,  1.1,  0.1,  9.3, 1.7); }
    if name.contains("carrot")   { return ( 41.0,  0.9,  0.2,  9.6, 2.8); }
    if name.contains("zucchini") || name.contains("courgette") { return ( 17.0,  1.2,  0.3,  3.1, 1.0); }
    if name.contains("sweet-potato") || name.contains("sweet_potato") { return ( 86.0, 1.6, 0.1, 20.0, 3.0); }
    if name.contains("mushroom") { return ( 22.0,  3.1,  0.3,  3.3, 1.0); }
    if name.contains("asparagus") { return ( 20.0, 2.2, 0.1, 3.9, 2.1); }
    if name.contains("cauliflower") { return ( 25.0, 1.9, 0.3, 5.0, 2.0); }
    if name.contains("eggplant") || name.contains("aubergine") { return ( 25.0, 1.0, 0.2, 6.0, 3.0); }
    if name.contains("kale")     { return ( 49.0,  4.3,  0.9,  8.8, 3.6); }
    if name.contains("pea")      { return ( 81.0,  5.4,  0.4, 14.0, 5.1); }
    if name.contains("corn")     { return ( 86.0,  3.3,  1.2, 19.0, 2.7); }
    if name.contains("cabbage")  { return ( 25.0,  1.3,  0.1,  5.8, 2.5); }
    if name.contains("lettuce") || name.contains("arugula") { return ( 15.0, 1.4, 0.2, 2.9, 1.3); }
    // Fruits & fats
    if name.contains("avocado")  { return (160.0,  2.0, 15.0,  9.0, 7.0); }
    if name.contains("banana")   { return ( 89.0,  1.1,  0.3, 23.0, 2.6); }
    if name.contains("apple")    { return ( 52.0,  0.3,  0.2, 14.0, 2.4); }
    if name.contains("olive")    { return (115.0,  0.8, 11.0,  6.0, 3.2); }
    if name.contains("coconut")  { return (354.0,  3.3, 33.0, 15.0, 9.0); }
    // Nuts & seeds
    if name.contains("almond")   { return (579.0, 21.0, 50.0, 22.0, 12.0); }
    if name.contains("walnut")   { return (654.0, 15.0, 65.0, 14.0, 6.7); }
    if name.contains("sesame") || name.contains("tahini") { return (573.0, 17.0, 50.0, 23.0, 12.0); }
    if name.contains("peanut")   { return (567.0, 26.0, 49.0, 16.0, 8.5); }
    // Default: unknown ingredient (moderate vegetable)
    (35.0, 1.5, 0.3, 7.0, 2.0)
}

/// Default portion size (grams) for each ingredient type.
fn default_portion_grams(ingredient: &str) -> f64 {
    let name = ingredient.to_lowercase();
    // Proteins: larger portions
    if name.contains("salmon") || name.contains("tuna") || name.contains("chicken")
        || name.contains("beef") || name.contains("pork") || name.contains("turkey")
        || name.contains("lamb") || name.contains("cod") || name.contains("shrimp")
        || name.contains("duck") || name.contains("trout") || name.contains("mackerel")
    {
        return 150.0;
    }
    // Eggs
    if name.contains("egg") { return 100.0; }
    // Grains/starches (cooked)
    if name.contains("rice") || name.contains("pasta") || name.contains("quinoa")
        || name.contains("noodle") || name.contains("oat")
    {
        return 100.0;
    }
    if name.contains("potato") || name.contains("sweet-potato") { return 150.0; }
    if name.contains("bread") { return 60.0; } // ~2 slices
    // Avocado, cheese — smaller
    if name.contains("avocado") { return 80.0; }
    if name.contains("cheese")  { return 30.0; }
    // Nuts/seeds — small portion
    if name.contains("almond") || name.contains("walnut") || name.contains("peanut")
        || name.contains("sesame") || name.contains("tahini")
    {
        return 20.0;
    }
    // Legumes
    if name.contains("lentil") || name.contains("chickpea") || name.contains("bean")
        || name.contains("tofu") || name.contains("tempeh")
    {
        return 120.0;
    }
    // Default vegetable/fruit
    80.0
}

/// Pre-calculated nutrition totals for a combo.
/// This is the SINGLE SOURCE OF TRUTH — all display reads from these values.
#[derive(Debug, Clone)]
struct NutritionTotals {
    total_weight_g: f64,
    servings_count: i16,
    calories_total: f64,
    protein_total: f64,
    fat_total: f64,
    carbs_total: f64,
    fiber_total: f64,
    calories_per_serving: f64,
    protein_per_serving: f64,
    fat_per_serving: f64,
    carbs_per_serving: f64,
    fiber_per_serving: f64,
    /// Per-ingredient breakdown for the AI prompt
    breakdown: Vec<String>,
}

/// Calculate nutrition totals from ingredient list.
/// Uses USDA-based lookup table × default portion sizes.
/// Returns one authoritative NutritionTotals — used everywhere.
fn calculate_nutrition(ingredients: &[String]) -> NutritionTotals {
    let mut total_weight = 0.0_f64;
    let mut total_kcal = 0.0_f64;
    let mut total_protein = 0.0_f64;
    let mut total_fat = 0.0_f64;
    let mut total_carbs = 0.0_f64;
    let mut total_fiber = 0.0_f64;
    let mut breakdown = Vec::new();

    for ing in ingredients {
        let (kcal100, prot100, fat100, carbs100, fiber100) = nutrition_per_100g(ing);
        let portion = default_portion_grams(ing);

        let kcal = kcal100 * portion / 100.0;
        let prot = prot100 * portion / 100.0;
        let fat  = fat100  * portion / 100.0;
        let carbs = carbs100 * portion / 100.0;
        let fiber = fiber100 * portion / 100.0;

        total_weight += portion;
        total_kcal += kcal;
        total_protein += prot;
        total_fat += fat;
        total_carbs += carbs;
        total_fiber += fiber;

        breakdown.push(format!(
            "- {} ({}g): {:.0} kcal, {:.1}g protein, {:.1}g fat, {:.1}g carbs",
            ing, portion, kcal, prot, fat, carbs
        ));
    }

    // 1 serving = whole recipe (single portion)
    let servings: i16 = 1;
    NutritionTotals {
        total_weight_g: total_weight,
        servings_count: servings,
        calories_total: total_kcal,
        protein_total: total_protein,
        fat_total: total_fat,
        carbs_total: total_carbs,
        fiber_total: total_fiber,
        calories_per_serving: total_kcal,
        protein_per_serving: total_protein,
        fat_per_serving: total_fat,
        carbs_per_serving: total_carbs,
        fiber_per_serving: total_fiber,
        breakdown,
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
    model: &str,
    nt: &NutritionTotals,
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

    // Nutrition numbers are PRE-CALCULATED and stored in DB — inject as constants
    let estimated_protein = nt.protein_per_serving;
    let estimated_calories = nt.calories_per_serving;
    let estimated_fat = nt.fat_per_serving;
    let estimated_carbs = nt.carbs_per_serving;
    let estimated_fiber = nt.fiber_per_serving;
    let total_weight = nt.total_weight_g;

    // Per-ingredient breakdown from NutritionTotals
    let breakdown_text = nt.breakdown.join("\n");

    let prompt = format!(
        r#"You are a professional chef and nutritionist writing a recipe page for SEO.

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
The following values are calculated from USDA food composition data.
You MUST use these numbers in title, intro, and why_it_works fields.
DO NOT recalculate — use the values below:

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
� OUTPUT FORMAT (return ONLY valid JSON)
═══════════════════════════════════════
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

═══════════════════════════════════════
🔥 FIELD RULES (follow exactly)
═══════════════════════════════════════

title (max 55 chars):
- Format: "[Dish Name] ({estimated_protein:.0}g Protein, [N] Min)"
- The protein number MUST be {estimated_protein:.0}g (pre-calculated above)
- Example: "Salmon Rice Bowl ({estimated_protein:.0}g Protein, 15 Min)"
- FORBIDDEN words: "analysis", "combo", "combination"

description (120-150 chars):
- Start with action verb: "Make", "Cook", "Try"
- MUST include "{estimated_protein:.0}g protein" and cooking time
- Example: "Cook this {meal_text} in 15 min. {estimated_protein:.0}g protein, {estimated_calories:.0} kcal per serving."

h1 (40-70 chars):
- Recipe name style: "[Ingredients] [Dish Type] — [Benefit] Recipe"
- Example: "Salmon Avocado Rice Bowl — High Protein {meal_text} Recipe"

intro (150-250 chars):
- FIRST SENTENCE must contain exact numbers: "This [dish] delivers ~{estimated_protein:.0}g protein and ~{estimated_calories:.0} kcal per serving, ready in [N] minutes."
- Second sentence: one specific nutrition or culinary fact (omega-3, complex carbs, etc.)
- FORBIDDEN fillers: "delicious", "amazing", "perfect", "comprehensive"

why_it_works (200-400 chars):
- Explain EACH ingredient's specific role with numbers:
  * Name the protein source: "[ingredient] provides [N]g protein + [nutrient: omega-3, leucine, iron, etc.]"
  * Name the carb source: "[ingredient] delivers complex carbs for sustained energy"
  * Name the fat/veggie source: "[ingredient] adds [N]g monounsaturated fats / fiber / vitamins"
- End with flavor pairing logic: "umami from [X] meets [Y] for..."
- NEVER write: "Optimized for balanced...", "This combination is great", "comprehensive"

═══════════════════════════════════════
🔥 COOKING STEPS RULES
═══════════════════════════════════════

STEP ORDER (must follow):
1. Grains/starches first (they take longest)
2. Protein second (fish, meat, eggs)
3. Vegetables third (if any need cooking)
4. Raw ingredients (slice avocado, chop herbs — NEVER cook)
5. Assemble and serve

RAW-ONLY (NEVER cook): avocado, lettuce, arugula, cucumber, tomato, herbs (basil, cilantro, parsley, dill, mint), lemon, lime
MUST-COOK: salmon, tuna, chicken, beef, pork, eggs, shrimp, cod, turkey, lamb
GRAINS (boil/steam): rice, pasta, quinoa, potato, oats

Each step MUST include: ingredient name + weight (g) + method + time (min) + heat level

BAD: "Prepare sides: Rice, Salmon" / "Cook Avocado" / "Season to taste"
GOOD: "Boil rice (100g) in 200ml water for 12 min, rest 5 min covered."

═══════════════════════════════════════
🚫 ABSOLUTE PROHIBITIONS
═══════════════════════════════════════
- NEVER return protein = 0g (the correct value is {estimated_protein:.0}g)
- NEVER cook avocado, lettuce, herbs, cucumber
- NEVER use: "analysis", "combo", "combination", "comprehensive", "detailed"
- NEVER write generic steps without specific grams and minutes
- NEVER list fish/meat as a "side" — it is the MAIN protein"#
    );

    tracing::info!("🤖 AI enrichment for combo {} using model: {} (estimated protein: {:.0}g, calories: {:.0} kcal)",
        combo_id, model, estimated_protein, estimated_calories);

    let raw_response = llm.groq_raw_request_with_model(&prompt, 3000, model).await?;

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

    let mut title = enriched.get("title").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let mut description = enriched.get("description").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let h1 = enriched.get("h1").and_then(|v| v.as_str()).unwrap_or("").trim();
    let mut intro = enriched.get("intro").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let why_it_works = enriched.get("why_it_works").and_then(|v| v.as_str()).unwrap_or("").trim();

    // ── PROTEIN SAFETY NET ──────────────────────────────────────────────
    // If AI returned title/intro without protein numbers, or with 0g — fix it.
    // This is the HARD fallback that ensures we NEVER show "0g protein".
    let est_protein = estimated_protein.round() as i64;
    let est_calories = estimated_calories.round() as i64;

    // Fix title: ensure it contains "(Xg Protein, N Min)"
    if !title.contains("Protein") && !title.contains("protein") && !title.contains("белка") && !title.contains("białka") && !title.contains("білка") {
        // Title is missing protein — append it
        let time_hint = "15 Min";
        let protein_hook = format!("({}g Protein, {})", est_protein, time_hint);
        // Remove existing parenthetical if any
        if let Some(paren_start) = title.rfind('(') {
            title = title[..paren_start].trim().to_string();
        }
        title = smart_truncate(&format!("{} {}", title, protein_hook), 60);
        tracing::warn!("⚠️ AI title missing protein for combo {} — injected {}g", combo_id, est_protein);
    }

    // Fix title: if it says "0g Protein" — replace with real value
    if title.contains("0g Protein") || title.contains("0 g Protein") {
        title = title.replace("0g Protein", &format!("{}g Protein", est_protein));
        title = title.replace("0 g Protein", &format!("{}g Protein", est_protein));
        tracing::warn!("⚠️ AI returned 0g protein in title for combo {} — fixed to {}g", combo_id, est_protein);
    }

    // Fix intro: if it mentions 0g or has no protein number
    if intro.contains("~0g") || intro.contains("0g protein") || intro.contains("0 g protein") {
        intro = intro.replace("~0g", &format!("~{}g", est_protein));
        intro = intro.replace("0g protein", &format!("{}g protein", est_protein));
        intro = intro.replace("0 g protein", &format!("{}g protein", est_protein));
        tracing::warn!("⚠️ AI returned 0g protein in intro for combo {} — fixed to {}g", combo_id, est_protein);
    }

    // Fix description: ensure protein is mentioned
    if !description.contains("protein") && !description.contains("белка") && !description.contains("białka") && !description.contains("білка") && est_protein > 5 {
        description = format!("{}. {}g protein per serving.", description.trim_end_matches('.'), est_protein);
        if description.len() > 155 {
            description = description.chars().take(152).collect::<String>() + "...";
        }
    }

    tracing::info!("✅ AI enrichment result — title: '{}', protein: {}g, calories: {} kcal, model: {}",
        title, est_protein, est_calories, model);

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
