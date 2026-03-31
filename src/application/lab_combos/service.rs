// ─── LabComboService — CRUD + generation pipeline ───────────────────────────
//
// Orchestrates the full combo page lifecycle:
//   1. Resolve ingredient names → slugs
//   2. Call SmartService for analysis
//   3. Calculate nutrition (NutritionTotals)
//   4. Generate template SEO text
//   5. Build structured ingredients from catalog
//   6. Save to DB
//   7. Spawn async AI enrichment (with DishClassifier + RecipeValidator)
//   8. CRUD operations (list, get, update, delete, publish, archive)

use crate::application::smart_service::{CulinaryContext, SmartService};
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::infrastructure::R2Client;
use crate::shared::{AppError, AppResult};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::enrichment;
use super::nutrition::{self, default_portion_grams, nutrition_per_100g};
use super::templates::{self, capitalize_words, smart_truncate};
use super::types::*;

pub struct LabComboService {
    pool: PgPool,
    smart_service: Arc<SmartService>,
    r2_client: R2Client,
    llm_adapter: Arc<LlmAdapter>,
}

impl LabComboService {
    pub fn new(
        pool: PgPool,
        smart_service: Arc<SmartService>,
        r2_client: R2Client,
        llm_adapter: Arc<LlmAdapter>,
    ) -> Self {
        Self {
            pool,
            smart_service,
            r2_client,
            llm_adapter,
        }
    }

    // ── Resolve ingredient names (any language) → English slugs ──────────

    pub async fn resolve_ingredient_slugs(&self, inputs: &[String]) -> AppResult<Vec<String>> {
        let mut slugs = Vec::with_capacity(inputs.len());

        for input in inputs {
            let normalized = input.trim().to_lowercase();

            // Try exact slug match first
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT slug FROM catalog_ingredients WHERE slug = $1 AND is_active = true LIMIT 1",
            )
            .bind(&normalized)
            .fetch_optional(&self.pool)
            .await?;

            if let Some((slug,)) = row {
                slugs.push(slug);
                continue;
            }

            // Try match by name in any language
            let row: Option<(String,)> = sqlx::query_as(
                r#"SELECT slug FROM catalog_ingredients
                   WHERE is_active = true
                     AND (
                       LOWER(name_en) = $1
                       OR LOWER(name_ru) = $1
                       OR LOWER(name_pl) = $1
                       OR LOWER(name_uk) = $1
                     )
                   LIMIT 1"#,
            )
            .bind(&normalized)
            .fetch_optional(&self.pool)
            .await?;

            if let Some((slug,)) = row {
                slugs.push(slug);
                continue;
            }

            // Fallback: use input as slug
            let fallback = normalized.replace(' ', "-");
            tracing::warn!(
                "⚠️ Ingredient '{}' not found in catalog, using as slug: {}",
                input, fallback
            );
            slugs.push(fallback);
        }

        Ok(slugs)
    }

    // ── Build structured ingredients from catalog DB ─────────────────────

    async fn build_structured_ingredients(
        &self,
        slugs: &[String],
        locale: &str,
    ) -> AppResult<serde_json::Value> {
        let mut items = Vec::new();

        for slug in slugs {
            let row: Option<(
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
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

            if let Some((
                s, name_en, name_ru, name_pl, name_uk, img, pt, cal100, prot100, fat100, carb100,
            )) = row
            {
                let name = match locale {
                    "ru" => name_ru.as_deref().or(name_en.as_deref()).unwrap_or(&s),
                    "pl" => name_pl.as_deref().or(name_en.as_deref()).unwrap_or(&s),
                    "uk" => name_uk.as_deref().or(name_en.as_deref()).unwrap_or(&s),
                    _ => name_en.as_deref().unwrap_or(&s),
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

    // ── Backfill structured_ingredients ──────────────────────────────────

    pub async fn backfill_structured_ingredients(&self) -> AppResult<usize> {
        let rows: Vec<(uuid::Uuid, Vec<String>, String)> = sqlx::query_as(
            r#"SELECT id, ingredients, locale
               FROM lab_combo_pages
               WHERE structured_ingredients = '[]'::jsonb
               ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = rows.len();
        tracing::info!(
            "🔄 Backfilling structured_ingredients for {} records",
            total
        );

        let mut updated = 0;
        for (id, ingredients, locale) in rows {
            let structured = self
                .build_structured_ingredients(&ingredients, &locale)
                .await?;

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

        tracing::info!(
            "✅ Backfill complete: {}/{} records updated",
            updated, total
        );
        Ok(updated)
    }

    // ── Generate (Admin) ─────────────────────────────────────────────────

    pub async fn generate(&self, req: GenerateComboRequest) -> AppResult<LabComboPage> {
        if req.ingredients.is_empty() {
            return Err(AppError::validation("ingredients array must not be empty"));
        }
        if req.ingredients.len() > 10 {
            return Err(AppError::validation("max 10 ingredients per combo"));
        }

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

        // Check uniqueness
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM lab_combo_pages WHERE slug = $1 AND locale = $2")
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

        // Call SmartService
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
        let mut smart_json = serde_json::to_value(&smart_result).map_err(|e| {
            AppError::internal(format!("failed to serialize SmartResponse: {}", e))
        })?;

        // ── Calculate nutrition ──────────────────────────────────────────
        let nt = nutrition::calculate_nutrition(&ingredients);

        // ── NUTRITION SAFETY NET ────────────────────────────────────────
        {
            if let Some(nutrition_val) = smart_json.get_mut("nutrition") {
                let current_protein = nutrition_val
                    .get("protein")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let current_calories = nutrition_val
                    .get("calories")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                if current_protein < 2.0 && nt.protein_per_serving > 5.0 {
                    let protein_per_100g = nt.protein_per_serving / 3.0;
                    nutrition_val.as_object_mut().map(|n| {
                        n.insert(
                            "protein".to_string(),
                            serde_json::json!(protein_per_100g),
                        );
                    });
                    tracing::warn!(
                        "⚠️ SmartService returned protein={:.1}g — overriding with {:.1}g/100g",
                        current_protein, protein_per_100g
                    );
                }

                if current_calories < 10.0 && nt.calories_per_serving > 50.0 {
                    let calories_per_100g = nt.calories_per_serving / 3.0;
                    nutrition_val.as_object_mut().map(|n| {
                        n.insert(
                            "calories".to_string(),
                            serde_json::json!(calories_per_100g),
                        );
                    });
                    tracing::warn!(
                        "⚠️ SmartService returned calories={:.0} — overriding with {:.0}/100g",
                        current_calories, calories_per_100g
                    );
                }
            }
        }

        // Generate SEO metadata
        let title = if let Some(ref dn) = req.dish_name {
            let est_protein = nt.protein_per_serving.round() as i64;
            smart_truncate(
                &format!("{} ({}g Protein, 15 Min)", dn, est_protein),
                60,
            )
        } else {
            templates::generate_title(
                &ingredients,
                req.goal.as_deref(),
                req.meal_type.as_deref(),
                &req.locale,
                &nt,
            )
        };
        let description =
            templates::generate_description(&ingredients, req.goal.as_deref(), &req.locale, &nt);
        let h1 = if let Some(ref dn) = req.dish_name {
            smart_truncate(dn, 70)
        } else {
            templates::generate_h1(
                &ingredients,
                req.goal.as_deref(),
                req.meal_type.as_deref(),
                &req.locale,
            )
        };
        let intro =
            templates::generate_intro(&ingredients, req.goal.as_deref(), &req.locale, &nt);
        let faq = templates::generate_faq(&ingredients, &smart_json, &req.locale, &nt);
        let why_it_works = templates::generate_why_it_works(
            &ingredients,
            &smart_json,
            req.goal.as_deref(),
            &req.locale,
            &nt,
        );
        let how_to_cook =
            templates::generate_how_to_cook(&ingredients, &smart_json, &req.locale);
        let optimization_tips =
            templates::generate_optimization_tips(&smart_json, &req.locale);

        // ── Build structured ingredients from catalog ────────────────────
        let structured_ingredients = self
            .build_structured_ingredients(&ingredients, &req.locale)
            .await?;

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
        let qs = templates::quality_score(&page);
        sqlx::query("UPDATE lab_combo_pages SET quality_score = $1 WHERE id = $2")
            .bind(qs)
            .bind(id)
            .execute(&self.pool)
            .await?;

        // ── AI Enrichment (async) ───────────────────────────────────────
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
        let dish_name_bg = req.dish_name.clone();
        let model_bg = ai_model.to_string();
        let nt_bg = nt.clone();
        tokio::spawn(async move {
            if let Err(e) = enrichment::enrich_seo_with_ai(
                &pool_bg,
                &llm_bg,
                id,
                &ingredients_bg,
                &locale_bg,
                goal_bg.as_deref(),
                meal_type_bg.as_deref(),
                dish_name_bg.as_deref(),
                &model_bg,
                &nt_bg,
            )
            .await
            {
                tracing::warn!("⚠️ AI enrichment failed for combo {}: {}", id, e);
            }
        });

        Ok(page)
    }

    // ── Generate for ALL locales ─────────────────────────────────────────

    pub async fn generate_all_locales(
        &self,
        req: GenerateComboRequest,
    ) -> AppResult<Vec<LabComboPage>> {
        const LOCALES: [&str; 4] = ["en", "pl", "ru", "uk"];

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
                dish_name: req.dish_name.clone(),
                model: req.model.clone(),
            };

            match self.generate(locale_req).await {
                Ok(page) => {
                    tracing::info!("✅ Generated lab combo [{}] locale={}", page.slug, locale);
                    pages.push(page);
                }
                Err(e) => {
                    let msg = format!("{}", e);
                    if msg.contains("already exists") {
                        tracing::info!(
                            "⏭️ Lab combo already exists for locale={}, skipping",
                            locale
                        );
                    } else {
                        tracing::error!(
                            "❌ Failed to generate lab combo locale={}: {}",
                            locale, e
                        );
                        return Err(e);
                    }
                }
            }
        }

        Ok(pages)
    }

    // ── Publish ──────────────────────────────────────────────────────────

    pub async fn publish(&self, id: Uuid) -> AppResult<LabComboPage> {
        let page = sqlx::query_as::<_, LabComboPage>(
            r#"
            UPDATE lab_combo_pages
            SET status = 'published', published_at = now(), updated_at = now()
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

    // ── Archive ──────────────────────────────────────────────────────────

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

    // ── Delete ───────────────────────────────────────────────────────────

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

    // ── Update ───────────────────────────────────────────────────────────

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

        let qs = templates::quality_score(&page);
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

    // ── List ─────────────────────────────────────────────────────────────

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

    pub async fn get_published(
        &self,
        slug: &str,
        locale: &str,
    ) -> AppResult<Option<LabComboPage>> {
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

    // ── Public: related combos ───────────────────────────────────────────

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

    // ── Public: "People also cook" ───────────────────────────────────────

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

    // ── Public: sitemap ──────────────────────────────────────────────────

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

    // ── Bulk generate popular combos ─────────────────────────────────────

    pub async fn generate_popular_combos(
        &self,
        _locale: &str,
        limit: usize,
    ) -> AppResult<Vec<String>> {
        let popular_combos: Vec<(Vec<&str>, Option<&str>, Option<&str>)> = vec![
            (vec!["chicken", "broccoli", "rice"], Some("high_protein"), Some("dinner")),
            (vec!["salmon", "avocado", "rice"], Some("high_protein"), Some("lunch")),
            (vec!["eggs", "spinach", "tomato"], Some("high_protein"), Some("breakfast")),
            (vec!["tuna", "quinoa", "cucumber"], Some("high_protein"), Some("lunch")),
            (vec!["chicken", "sweet-potato", "green-beans"], Some("high_protein"), Some("dinner")),
            (vec!["chicken", "salad", "cucumber"], Some("weight_loss"), Some("lunch")),
            (vec!["salmon", "asparagus", "lemon"], Some("low_carb"), Some("dinner")),
            (vec!["eggs", "avocado"], Some("keto"), Some("breakfast")),
            (vec!["pasta", "tomato", "basil"], None, Some("dinner")),
            (vec!["rice", "chicken", "soy-sauce"], None, Some("dinner")),
            (vec!["potato", "onion", "mushroom"], None, Some("dinner")),
            (vec!["banana", "oats", "milk"], None, Some("breakfast")),
            (vec!["tofu", "rice", "broccoli"], Some("high_protein"), Some("dinner")),
            (vec!["lentils", "rice", "tomato"], Some("high_protein"), Some("lunch")),
        ];

        let mut generated = Vec::new();

        for (ings, goal, meal) in popular_combos.into_iter().take(limit) {
            let slug = combo_slug(
                &ings.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                goal,
                meal,
                None,
                None,
                None,
                None,
            );

            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM lab_combo_pages WHERE slug = $1 AND locale IN ('en','pl','ru','uk')",
            )
            .bind(&slug)
            .fetch_one(&self.pool)
            .await?;

            if count.0 >= 4 {
                continue;
            }

            match self
                .generate_all_locales(GenerateComboRequest {
                    ingredients: ings.iter().map(|s| s.to_string()).collect(),
                    locale: "en".to_string(),
                    goal: goal.map(String::from),
                    meal_type: meal.map(String::from),
                    diet: None,
                    cooking_time: None,
                    budget: None,
                    cuisine: None,
                    dish_name: None,
                    model: Some("pro".to_string()),
                })
                .await
            {
                Ok(pages) => {
                    let locales: Vec<String> = pages.iter().map(|p| p.locale.clone()).collect();
                    generated.push(format!("✅ {} [{}]", slug, locales.join(",")));
                }
                Err(e) => generated.push(format!("❌ {} — {}", slug, e)),
            }
        }

        Ok(generated)
    }

    // ── Image upload ─────────────────────────────────────────────────────

    pub async fn get_image_upload_url(
        &self,
        id: Uuid,
        content_type: &str,
    ) -> AppResult<crate::application::user::AvatarUploadResponse> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lab_combo_pages WHERE id = $1)")
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
        let upload_url = self
            .r2_client
            .generate_presigned_upload_url(&key, content_type)
            .await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(crate::application::user::AvatarUploadResponse {
            upload_url,
            public_url,
        })
    }

    pub async fn save_image_url(&self, id: Uuid, image_url: String) -> AppResult<LabComboPage> {
        self.save_typed_image_url(id, "hero", image_url).await
    }

    pub async fn get_typed_image_upload_url(
        &self,
        id: Uuid,
        kind: &str,
        content_type: &str,
    ) -> AppResult<crate::application::user::AvatarUploadResponse> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lab_combo_pages WHERE id = $1)")
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
        let upload_url = self
            .r2_client
            .generate_presigned_upload_url(&key, content_type)
            .await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(crate::application::user::AvatarUploadResponse {
            upload_url,
            public_url,
        })
    }

    pub async fn save_typed_image_url(
        &self,
        id: Uuid,
        kind: &str,
        url: String,
    ) -> AppResult<LabComboPage> {
        let column = match kind {
            "process" => "process_image_url",
            "detail" => "detail_image_url",
            _ => "image_url",
        };

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
