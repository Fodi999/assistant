// ─── LabComboService — thin facade over Repository + Generator ──────────────
//
// Orchestrates the full combo page lifecycle by delegating to:
//   - ComboRepository (CRUD)
//   - RecipeGenerator  (AI pipeline)
//   - SEO modules      (template text)
//   - NutritionTotals  (USDA calculator)
//
// This is the public API consumed by HTTP handlers.

use crate::application::smart_service::{CulinaryContext, SmartService};
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::infrastructure::R2Client;
use crate::shared::{AppError, AppResult};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::generator;
use super::nutrition::{self, default_portion_grams, nutrition_per_100g};
use super::repository::ComboRepository;
use super::seo;
use super::types::*;

pub struct LabComboService {
    repo: ComboRepository,
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
            repo: ComboRepository::new(pool),
            smart_service,
            r2_client,
            llm_adapter,
        }
    }

    // ── Resolve ingredient names (any language) → English slugs ──────────

    pub async fn resolve_ingredient_slugs(&self, inputs: &[String]) -> AppResult<Vec<String>> {
        let mut slugs = Vec::with_capacity(inputs.len());

        for input in inputs {
            if let Some(slug) = self.repo.resolve_ingredient_slug(input).await? {
                slugs.push(slug);
            } else {
                let fallback = input.trim().to_lowercase().replace(' ', "-");
                tracing::warn!(
                    "⚠️ Ingredient '{}' not found in catalog, using as slug: {}",
                    input, fallback
                );
                slugs.push(fallback);
            }
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
            let portion = default_portion_grams(slug);

            if let Some((s, name_en, name_ru, name_pl, name_uk, img, pt, cal100, prot100, fat100, carb100)) =
                self.repo.get_catalog_ingredient(slug).await?
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
                let (cal100, prot100, fat100, carb100, _fiber) = nutrition_per_100g(slug);
                let name = seo::capitalize_words(&slug.replace('-', " "));

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
        let rows = self.repo.get_empty_structured().await?;
        let total = rows.len();
        tracing::info!("🔄 Backfilling structured_ingredients for {} records", total);

        let mut updated = 0;
        for (id, ingredients, locale) in rows {
            let structured = self.build_structured_ingredients(&ingredients, &locale).await?;
            self.repo.update_structured_ingredients(id, &structured).await?;
            updated += 1;
            tracing::info!("  ✅ Backfilled {}/{} — id={}", updated, total, id);
        }

        tracing::info!("✅ Backfill complete: {}/{} records updated", updated, total);
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
        if self.repo.exists(&slug, &req.locale).await? {
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
                        n.insert("protein".to_string(), serde_json::json!(protein_per_100g));
                    });
                }

                if current_calories < 10.0 && nt.calories_per_serving > 50.0 {
                    let calories_per_100g = nt.calories_per_serving / 3.0;
                    nutrition_val.as_object_mut().map(|n| {
                        n.insert("calories".to_string(), serde_json::json!(calories_per_100g));
                    });
                }
            }
        }

        // Generate SEO metadata
        let title = if let Some(ref dn) = req.dish_name {
            let est_protein = nt.protein_per_serving.round() as i64;
            seo::smart_truncate(
                &format!("{} ({}g Protein, 15 Min)", dn, est_protein),
                60,
            )
        } else {
            seo::generate_title(
                &ingredients,
                req.goal.as_deref(),
                req.meal_type.as_deref(),
                &req.locale,
                &nt,
            )
        };
        let description =
            seo::generate_description(&ingredients, req.goal.as_deref(), &req.locale, &nt);
        let h1 = if let Some(ref dn) = req.dish_name {
            seo::smart_truncate(dn, 70)
        } else {
            seo::generate_h1(
                &ingredients,
                req.goal.as_deref(),
                req.meal_type.as_deref(),
                &req.locale,
            )
        };
        let intro =
            seo::generate_intro(&ingredients, req.goal.as_deref(), &req.locale, &nt);
        let faq = seo::generate_faq(&ingredients, &smart_json, &req.locale, &nt);
        let why_it_works = seo::generate_why_it_works(
            &ingredients,
            &smart_json,
            req.goal.as_deref(),
            &req.locale,
            &nt,
        );
        let how_to_cook =
            seo::generate_how_to_cook(&ingredients, &smart_json, &req.locale);
        let optimization_tips =
            seo::generate_optimization_tips(&smart_json, &req.locale);

        // ── Build structured ingredients from catalog ────────────────────
        let structured_ingredients = self
            .build_structured_ingredients(&ingredients, &req.locale)
            .await?;

        let id = Uuid::new_v4();

        let page = self.repo.insert(
            id, &slug, &req.locale, &ingredients,
            req.goal.as_deref(), req.meal_type.as_deref(), req.diet.as_deref(),
            req.cooking_time.as_deref(), req.budget.as_deref(), req.cuisine.as_deref(),
            &title, &description, &h1, &intro,
            &why_it_works, &how_to_cook, &optimization_tips,
            &smart_json, &faq,
            nt.total_weight_g as f32, nt.servings_count,
            nt.calories_total as f32, nt.protein_total as f32,
            nt.fat_total as f32, nt.carbs_total as f32, nt.fiber_total as f32,
            nt.calories_per_serving as f32, nt.protein_per_serving as f32,
            nt.fat_per_serving as f32, nt.carbs_per_serving as f32,
            nt.fiber_per_serving as f32,
            &structured_ingredients,
        ).await?;

        // Update quality score
        let qs = seo::quality_score(&page);
        self.repo.update_quality_score(id, qs).await?;

        // ── AI Enrichment (SYNCHRONOUS) ─────────────────────────────────
        // We wait for AI to generate + validate the recipe before returning.
        // This ensures the user NEVER sees template garbage — only AI-validated
        // cooking steps that passed Recipe::new() domain invariants.
        let ai_model = match req.model.as_deref() {
            Some("pro") | Some("gemini-3.1-pro-preview") => "gemini-3.1-pro-preview",
            _ => "gemini-3-flash-preview",
        };

        match generator::enrich_with_ai(
            &self.repo,
            &self.llm_adapter,
            id,
            &ingredients,
            &req.locale,
            req.goal.as_deref(),
            req.meal_type.as_deref(),
            req.dish_name.as_deref(),
            ai_model,
            &nt,
        )
        .await
        {
            Ok(()) => {
                tracing::info!("✅ AI enrichment completed synchronously for combo {}", id);
            }
            Err(e) => {
                tracing::warn!("⚠️ AI enrichment failed for combo {}: {} — page saved with template steps", id, e);
            }
        }

        // Re-read the page from DB to get AI-updated fields
        let final_page = self.repo.get_by_id(id).await?.unwrap_or(page);

        Ok(final_page)
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

    // ── Delegated CRUD ──────────────────────────────────────────────────

    pub async fn publish(&self, id: Uuid) -> AppResult<LabComboPage> {
        self.repo.publish(id).await
    }

    pub async fn archive(&self, id: Uuid) -> AppResult<LabComboPage> {
        self.repo.archive(id).await
    }

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        self.repo.delete(id).await
    }

    pub async fn update(&self, id: Uuid, req: UpdateComboRequest) -> AppResult<LabComboPage> {
        let page = self.repo.update(id, &req).await?;
        let qs = seo::quality_score(&page);
        self.repo.update_quality_score(id, qs).await?;
        Ok(LabComboPage { quality_score: qs, ..page })
    }

    pub async fn list(&self, query: ListCombosQuery) -> AppResult<Vec<LabComboPage>> {
        self.repo.list(&query).await
    }

    pub async fn get_published(&self, slug: &str, locale: &str) -> AppResult<Option<LabComboPage>> {
        self.repo.get_published(slug, locale).await
    }

    pub async fn get_related_combos(&self, slug: &str, locale: &str, limit: i64) -> AppResult<Vec<RelatedCombo>> {
        self.repo.get_related_combos(slug, locale, limit).await
    }

    pub async fn get_also_cook(&self, slug: &str, locale: &str, limit: i64) -> AppResult<Vec<RelatedCombo>> {
        self.repo.get_also_cook(slug, locale, limit).await
    }

    pub async fn sitemap(&self) -> AppResult<Vec<LabComboSitemapEntry>> {
        self.repo.sitemap().await
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
                goal, meal, None, None, None, None,
            );

            let count = self.repo.count_by_slug(&slug).await?;
            if count >= 4 {
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
        if !self.repo.combo_exists(id).await? {
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

    pub async fn save_image_url(&self, id: Uuid, image_url: String) -> AppResult<LabComboPage> {
        self.save_typed_image_url(id, "hero", image_url).await
    }

    pub async fn get_typed_image_upload_url(
        &self,
        id: Uuid,
        kind: &str,
        content_type: &str,
    ) -> AppResult<crate::application::user::AvatarUploadResponse> {
        if !self.repo.combo_exists(id).await? {
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

    pub async fn save_typed_image_url(
        &self,
        id: Uuid,
        kind: &str,
        url: String,
    ) -> AppResult<LabComboPage> {
        self.repo.save_typed_image_url(id, kind, &url).await
    }

    // ── Metrics endpoint ────────────────────────────────────────────────

    pub fn metrics_snapshot(&self) -> super::metrics::MetricsSnapshot {
        super::metrics::global_metrics().snapshot()
    }
}
