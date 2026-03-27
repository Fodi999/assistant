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

/// Partial update for admin editing.
#[derive(Debug, Deserialize)]
pub struct UpdateComboRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub h1: Option<String>,
    pub intro: Option<String>,
    pub why_it_works: Option<String>,
    pub image_url: Option<String>,
}

// ── Slug Builder ─────────────────────────────────────────────────────────────

/// Build a deterministic, SEO-friendly slug from ingredients + context.
///
/// Examples:
///   - `["salmon", "rice"]` → `"salmon-rice"`
///   - `["salmon", "rice"]` + goal=high_protein + meal=dinner → `"high-protein-dinner-salmon-rice"`
///   - `["avocado", "chicken", "broccoli"]` → `"avocado-broccoli-chicken"` (sorted)
///
/// Ingredients are sorted alphabetically for deduplication.
pub fn combo_slug(
    ingredients: &[String],
    goal: Option<&str>,
    meal_type: Option<&str>,
    diet: Option<&str>,
    cooking_time: Option<&str>,
    budget: Option<&str>,
    cuisine: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Context dimensions first (most specific → broadest)
    if let Some(g) = goal {
        parts.push(g.replace('_', "-"));
    }
    if let Some(m) = meal_type {
        parts.push(m.replace('_', "-"));
    }
    if let Some(d) = diet {
        parts.push(d.replace('_', "-"));
    }
    if let Some(t) = cooking_time {
        parts.push(t.replace('_', "-"));
    }
    if let Some(b) = budget {
        parts.push(b.replace('_', "-"));
    }
    if let Some(c) = cuisine {
        parts.push(c.replace('_', "-"));
    }

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

    parts.extend(sorted_ingredients);
    parts.join("-")
}

// ── SEO Metadata Generation ──────────────────────────────────────────────────

/// Auto-generate SEO title from ingredients + context (≤ 60 chars).
fn generate_title(ingredients: &[String], goal: Option<&str>, meal_type: Option<&str>, locale: &str) -> String {
    let names = ingredients.join(", ");
    let ctx_parts: Vec<&str> = [goal, meal_type].iter().filter_map(|o| *o).collect();
    let ctx = if ctx_parts.is_empty() {
        String::new()
    } else {
        format!(" — {}", ctx_parts.join(" "))
            .replace('_', " ")
    };

    let suffix = match locale {
        "ru" => " | Анализ рецепта",
        "pl" => " | Analiza przepisu",
        "uk" => " | Аналіз рецепту",
        _    => " | Recipe Analysis",
    };

    let base = format!("{}{}", capitalize_words(&names), ctx);
    smart_truncate(&format!("{}{}", base, suffix), 60)
}

/// Auto-generate SEO description (80–155 chars).
fn generate_description(ingredients: &[String], goal: Option<&str>, locale: &str) -> String {
    let count = ingredients.len();
    let names = ingredients.join(", ");
    let goal_text = goal
        .map(|g| format!(" for {}", g.replace('_', " ")))
        .unwrap_or_default();

    let desc = match locale {
        "ru" => format!(
            "Детальный анализ {count} ингредиентов ({names}){goal_text}: КБЖУ, совместимость вкусов, рекомендации шефа."
        ),
        "pl" => format!(
            "Szczegółowa analiza {count} składników ({names}){goal_text}: KBJU, kompatybilność smakowa, rekomendacje szefa."
        ),
        "uk" => format!(
            "Детальний аналіз {count} інгредієнтів ({names}){goal_text}: КБЖУ, сумісність смаків, рекомендації шефа."
        ),
        _ => format!(
            "Detailed analysis of {count} ingredients ({names}){goal_text}: macros, flavor compatibility, chef recommendations."
        ),
    };

    smart_truncate(&desc, 155)
}

/// Auto-generate H1 heading.
fn generate_h1(ingredients: &[String], goal: Option<&str>, meal_type: Option<&str>, locale: &str) -> String {
    let names = capitalize_words(&ingredients.join(" + "));
    let ctx = match (goal, meal_type) {
        (Some(g), Some(m)) => format!(" — {} {}", capitalize_words(&g.replace('_', " ")), m),
        (Some(g), None) => format!(" — {}", capitalize_words(&g.replace('_', " "))),
        (None, Some(m)) => format!(" — {}", capitalize_words(m)),
        _ => String::new(),
    };

    match locale {
        "ru" => format!("Анализ: {}{}", names, ctx),
        "pl" => format!("Analiza: {}{}", names, ctx),
        "uk" => format!("Аналіз: {}{}", names, ctx),
        _    => format!("Analysis: {}{}", names, ctx),
    }
}

/// Auto-generate intro paragraph.
fn generate_intro(ingredients: &[String], goal: Option<&str>, locale: &str) -> String {
    let count = ingredients.len();
    let names = ingredients.join(", ");
    let goal_text = goal
        .map(|g| format!(" optimized for {}", g.replace('_', " ")))
        .unwrap_or_default();

    match locale {
        "ru" => format!(
            "Полный анализ комбинации из {} ингредиентов: {}{}. \
             Включает нутриенты (КБЖУ), профиль вкуса, совместимость, \
             предложения по улучшению и варианты рецептов.",
            count, names, goal_text
        ),
        "pl" => format!(
            "Pełna analiza kombinacji {} składników: {}{}. \
             Zawiera makro- i mikroelementy (KBJU), profil smakowy, kompatybilność, \
             sugestie ulepszeń i warianty przepisów.",
            count, names, goal_text
        ),
        "uk" => format!(
            "Повний аналіз комбінації {} інгредієнтів: {}{}. \
             Включає нутрієнти (КБЖУ), профіль смаку, сумісність, \
             пропозиції покращень та варіанти рецептів.",
            count, names, goal_text
        ),
        _ => format!(
            "Complete analysis of {} ingredient combo: {}{}. \
             Includes macros (CPFC), flavor profile, compatibility, \
             improvement suggestions and recipe variants.",
            count, names, goal_text
        ),
    }
}

/// Auto-generate FAQ from SmartResponse data.
fn generate_faq(
    ingredients: &[String],
    smart: &serde_json::Value,
    locale: &str,
) -> serde_json::Value {
    let names = ingredients.join(", ");
    let mut faq = Vec::new();

    // Q1: What are the macros?
    let nutrition = smart.get("nutrition");
    if let Some(n) = nutrition {
        let q = match locale {
            "ru" => format!("Какова пищевая ценность комбинации {}?", names),
            "pl" => format!("Jaka jest wartość odżywcza kombinacji {}?", names),
            "uk" => format!("Яка харчова цінність комбінації {}?", names),
            _    => format!("What are the nutritional values of {}?", names),
        };
        let kcal = n.get("calories").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let prot = n.get("protein").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let a = match locale {
            "ru" => format!("На 100 г: {:.0} ккал, {:.1} г белка.", kcal, prot),
            "pl" => format!("Na 100 g: {:.0} kcal, {:.1} g białka.", kcal, prot),
            "uk" => format!("На 100 г: {:.0} ккал, {:.1} г білка.", kcal, prot),
            _    => format!("Per 100g: {:.0} kcal, {:.1}g protein.", kcal, prot),
        };
        faq.push(serde_json::json!({ "question": q, "answer": a }));
    }

    // Q2: Are these ingredients compatible?
    let confidence = smart.get("confidence").and_then(|c| c.get("overall")).and_then(|v| v.as_f64());
    if let Some(score) = confidence {
        let q = match locale {
            "ru" => format!("Хорошо ли сочетаются {}?", names),
            "pl" => format!("Czy {} dobrze do siebie pasują?", names),
            "uk" => format!("Чи добре поєднуються {}?", names),
            _    => format!("Do {} go well together?", names),
        };
        let label = if score >= 0.8 { "excellent" } else if score >= 0.6 { "good" } else { "moderate" };
        let a = match locale {
            "ru" => format!("Оценка совместимости: {:.0}% — {}.", score * 100.0, label),
            "pl" => format!("Ocena kompatybilności: {:.0}% — {}.", score * 100.0, label),
            "uk" => format!("Оцінка сумісності: {:.0}% — {}.", score * 100.0, label),
            _    => format!("Compatibility score: {:.0}% — {}.", score * 100.0, label),
        };
        faq.push(serde_json::json!({ "question": q, "answer": a }));
    }

    // Q3: What can I add?
    let suggestions = smart.get("suggestions").and_then(|s| s.as_array());
    if let Some(sugg) = suggestions {
        let top: Vec<String> = sugg.iter()
            .take(3)
            .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect();
        if !top.is_empty() {
            let q = match locale {
                "ru" => format!("Что добавить к {}?", names),
                "pl" => format!("Co dodać do {}?", names),
                "uk" => format!("Що додати до {}?", names),
                _    => format!("What should I add to {}?", names),
            };
            let a = match locale {
                "ru" => format!("Рекомендуем добавить: {}.", top.join(", ")),
                "pl" => format!("Polecamy dodać: {}.", top.join(", ")),
                "uk" => format!("Рекомендуємо додати: {}.", top.join(", ")),
                _    => format!("We recommend adding: {}.", top.join(", ")),
            };
            faq.push(serde_json::json!({ "question": q, "answer": a }));
        }
    }

    // Q4: Recipe variants
    let variants = smart.get("variants").and_then(|v| v.as_array());
    if let Some(vars) = variants {
        if !vars.is_empty() {
            let q = match locale {
                "ru" => format!("Какие рецепты можно приготовить из {}?", names),
                "pl" => format!("Jakie przepisy można zrobić z {}?", names),
                "uk" => format!("Які рецепти можна приготувати з {}?", names),
                _    => format!("What recipes can I make with {}?", names),
            };
            let variant_names: Vec<String> = vars.iter()
                .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect();
            let a = if variant_names.is_empty() {
                match locale {
                    "ru" => format!("Доступно {} вариантов рецептов.", vars.len()),
                    "pl" => format!("Dostępnych jest {} wariantów przepisów.", vars.len()),
                    "uk" => format!("Доступно {} варіантів рецептів.", vars.len()),
                    _    => format!("{} recipe variants available.", vars.len()),
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
            "ru" => format!("Комбинация {names} — сбалансированный выбор с хорошим вкусовым профилем."),
            "pl" => format!("Kombinacja {names} — zbalansowany wybór z dobrym profilem smakowym."),
            "uk" => format!("Комбінація {names} — збалансований вибір із гарним смаковим профілем."),
            _    => format!("The combination of {names} is a balanced choice with a good flavor profile."),
        };
    }

    format!("{}.", parts.join(". "))
}

// ── "How to Cook" Generator ──────────────────────────────────────────────────

/// Generate cooking steps from the SmartResponse variants data.
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

        // Step 1: Prep
        let prep_step = match locale {
            "ru" => "Подготовьте все ингредиенты: вымойте, очистите и нарежьте по необходимости.".to_string(),
            "pl" => "Przygotuj wszystkie składniki: umyj, obierz i pokrój w razie potrzeby.".to_string(),
            "uk" => "Підготуйте всі інгредієнти: вимийте, очистіть та наріжте за потреби.".to_string(),
            _    => "Prepare all ingredients: wash, peel, and cut as needed.".to_string(),
        };
        steps.push(serde_json::json!({
            "step": 1,
            "text": prep_step,
            "time_minutes": 5
        }));

        // Group ingredients by role for cooking order
        if let Some(vi) = variant_ingredients {
            // Proteins/bases first
            let bases: Vec<&serde_json::Value> = vi.iter()
                .filter(|i| i.get("role").and_then(|r| r.as_str()) == Some("base"))
                .collect();

            // Sides
            let sides: Vec<&serde_json::Value> = vi.iter()
                .filter(|i| i.get("role").and_then(|r| r.as_str()) == Some("side"))
                .collect();

            // Aromatics
            let aromatics: Vec<&serde_json::Value> = vi.iter()
                .filter(|i| {
                    let role = i.get("role").and_then(|r| r.as_str()).unwrap_or("");
                    role == "aromatic" || role == "fat" || role == "sauce"
                })
                .collect();

            let mut step_num = 2;

            // Cook base ingredients
            if !bases.is_empty() {
                let base_names: Vec<String> = bases.iter()
                    .filter_map(|b| b.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect();
                let base_grams: Vec<String> = bases.iter()
                    .filter_map(|b| {
                        let name = b.get("name").and_then(|n| n.as_str())?;
                        let grams = b.get("grams").and_then(|g| g.as_f64())?;
                        Some(format!("{name} ({grams:.0}g)"))
                    })
                    .collect();

                let step_text = match locale {
                    "ru" => format!("Приготовьте основу: {}. Обжарьте или отварите до готовности.", base_grams.join(", ")),
                    "pl" => format!("Przygotuj bazę: {}. Usmaż lub ugotuj do gotowości.", base_grams.join(", ")),
                    "uk" => format!("Приготуйте основу: {}. Обсмажте або зваріть до готовності.", base_grams.join(", ")),
                    _    => format!("Cook the base: {}. Sear, grill or boil until done.", base_grams.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 10,
                    "ingredients": base_names
                }));
                step_num += 1;
            }

            // Cook sides
            if !sides.is_empty() {
                let side_grams: Vec<String> = sides.iter()
                    .filter_map(|s| {
                        let name = s.get("name").and_then(|n| n.as_str())?;
                        let grams = s.get("grams").and_then(|g| g.as_f64())?;
                        Some(format!("{name} ({grams:.0}g)"))
                    })
                    .collect();

                let step_text = match locale {
                    "ru" => format!("Подготовьте гарнир: {}.", side_grams.join(", ")),
                    "pl" => format!("Przygotuj dodatki: {}.", side_grams.join(", ")),
                    "uk" => format!("Підготуйте гарнір: {}.", side_grams.join(", ")),
                    _    => format!("Prepare sides: {}.", side_grams.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 8,
                    "ingredients": sides.iter()
                        .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(String::from))
                        .collect::<Vec<_>>()
                }));
                step_num += 1;
            }

            // Add aromatics/sauces
            if !aromatics.is_empty() {
                let aro_names: Vec<String> = aromatics.iter()
                    .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect();

                let step_text = match locale {
                    "ru" => format!("Добавьте ароматику и соус: {}.", aro_names.join(", ")),
                    "pl" => format!("Dodaj aromaty i sos: {}.", aro_names.join(", ")),
                    "uk" => format!("Додайте ароматику та соус: {}.", aro_names.join(", ")),
                    _    => format!("Add aromatics and sauce: {}.", aro_names.join(", ")),
                };
                steps.push(serde_json::json!({
                    "step": step_num,
                    "text": step_text,
                    "time_minutes": 3,
                    "ingredients": aro_names
                }));
                step_num += 1;
            }

            // Final assembly
            let total_cal = variant.get("total_calories").and_then(|c| c.as_i64()).unwrap_or(0);
            let assemble = match locale {
                "ru" => format!("Соберите блюдо: выложите на тарелку, подайте. Итого ~{total_cal} ккал."),
                "pl" => format!("Złóż danie: ułóż na talerzu i podaj. Razem ~{total_cal} kcal."),
                "uk" => format!("Зберіть страву: викладіть на тарілку, подайте. Разом ~{total_cal} ккал."),
                _    => format!("Assemble the dish: plate up and serve. Total ~{total_cal} kcal."),
            };
            steps.push(serde_json::json!({
                "step": step_num,
                "text": assemble,
                "time_minutes": 2
            }));
        }
    }

    // Fallback: generic steps if no variant data
    if steps.is_empty() {
        let names = ingredients.join(", ");
        let s1 = match locale {
            "ru" => format!("Подготовьте ингредиенты: {}.", names),
            "pl" => format!("Przygotuj składniki: {}.", names),
            "uk" => format!("Підготуйте інгредієнти: {}.", names),
            _    => format!("Prepare your ingredients: {}.", names),
        };
        let s2 = match locale {
            "ru" => "Нарежьте, смешайте и приготовьте по вкусу.".to_string(),
            "pl" => "Pokrój, wymieszaj i gotuj według smaku.".to_string(),
            "uk" => "Наріжте, змішайте та приготуйте за смаком.".to_string(),
            _    => "Cut, mix, and cook to your preference.".to_string(),
        };
        let s3 = match locale {
            "ru" => "Подайте тёплым.".to_string(),
            "pl" => "Podaj na ciepło.".to_string(),
            "uk" => "Подайте теплим.".to_string(),
            _    => "Serve warm.".to_string(),
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
}

impl LabComboService {
    pub fn new(pool: PgPool, smart_service: Arc<SmartService>) -> Self {
        Self { pool, smart_service }
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
                why_it_works, how_to_cook, optimization_tips, image_url,
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
                why_it_works, how_to_cook, optimization_tips, image_url,
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
                why_it_works, how_to_cook, optimization_tips, image_url,
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
                updated_at = now()
            WHERE id = $1
            RETURNING id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips, image_url,
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
                why_it_works, how_to_cook, optimization_tips, image_url,
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
                why_it_works, how_to_cook, optimization_tips, image_url,
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
