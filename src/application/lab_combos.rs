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

    // +1: smart_response has nutrition + flavor_profile
    let has_nutrition = page.smart_response.get("nutrition").is_some();
    let has_flavor = page.smart_response.get("flavor_profile").is_some();
    if has_nutrition && has_flavor {
        score += 1;
    }

    // +1: FAQ ≥ 3 entries
    let faq_count = page.faq.as_array().map(|a| a.len()).unwrap_or(0);
    if faq_count >= 3 {
        score += 1;
    }

    score
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

        let id = Uuid::new_v4();

        let page = sqlx::query_as::<_, LabComboPage>(
            r#"
            INSERT INTO lab_combo_pages (
                id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                smart_response, faq, status
            ) VALUES (
                $1, $2, $3, $4,
                $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14,
                $15, $16, 'draft'
            )
            RETURNING id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
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

    // ── List (Admin) ─────────────────────────────────────────────────────

    pub async fn list(&self, query: ListCombosQuery) -> AppResult<Vec<LabComboPage>> {
        let limit = query.limit.unwrap_or(50).min(200);
        let offset = query.offset.unwrap_or(0);

        let rows = sqlx::query_as::<_, LabComboPage>(
            r#"
            SELECT id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
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
