use crate::domain::classification_rules::ClassificationRules;
use crate::infrastructure::gemini_service::GeminiService;
use crate::infrastructure::groq_service::{GroqTranslationResponse, UnifiedProductResponse};
use crate::infrastructure::persistence::{AiCacheRepository, AiUsageStatsRepository};
use crate::shared::AppError;
use serde_json::to_value;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{timeout, Duration};

#[derive(Clone)]
pub struct LlmAdapter {
    gemini_service: Arc<GeminiService>,
    cache_repo: Arc<AiCacheRepository>,
    usage_repo: Arc<AiUsageStatsRepository>,
}

impl LlmAdapter {
    pub fn new(
        gemini_service: Arc<GeminiService>,
        cache_repo: Arc<AiCacheRepository>,
        usage_repo: Arc<AiUsageStatsRepository>,
    ) -> Self {
        Self {
            gemini_service,
            cache_repo,
            usage_repo,
        }
    }

    /// Helper to log usage stats
    async fn log_usage(&self, endpoint: &str, duration_ms: i32) {
        let _ = self
            .usage_repo
            .log_usage(endpoint, 0, 0, 0, duration_ms)
            .await;
    }

    /// Translation for a product to all supported UI languages (using cache)
    pub async fn translate(
        &self,
        ingredient_name: &str,
    ) -> Result<GroqTranslationResponse, AppError> {
        let cache_key = format!("translate_all:{}", ingredient_name.to_lowercase().trim());

        if let Some(cached_val) = self.cache_repo.get(&cache_key).await? {
            if let Ok(response) = serde_json::from_value::<GroqTranslationResponse>(cached_val) {
                tracing::info!("📦 Translation cache hit: {}", ingredient_name);
                return Ok(response);
            }
        }

        let start = Instant::now();
        let response = timeout(
            Duration::from_secs(10),
            self.gemini_service.translate(ingredient_name),
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Translation took too long"))??;

        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("translate_all", duration_ms).await;

        let val = to_value(&response).map_err(|e| AppError::internal(e.to_string()))?;
        self.cache_repo
            .set(&cache_key, val, "gemini", "gemini-3-flash-preview", 90)
            .await?;

        Ok(response)
    }

    /// Unified classification and translation for a product.
    /// Order: Cache -> LLM (with optional Rule Engine hint for category/unit)
    ///
    /// Rule Engine provides category_slug + unit hints but NEVER translations.
    /// Translations ALWAYS come from Cache or LLM.
    pub async fn process_unified(&self, name: &str) -> Result<UnifiedProductResponse, AppError> {
        // 1. Cache (FAST, low latency)
        let cache_key = format!("product_unified:{}", name.to_lowercase().trim());
        if let Some(cached_val) = self.cache_repo.get(&cache_key).await? {
            if let Ok(response) = serde_json::from_value::<UnifiedProductResponse>(cached_val) {
                tracing::info!("📦 Cache hit: {}", name);
                return Ok(response);
            }
        }

        // 2. Rule Engine hint (category + unit only, NO translations)
        let rule_hint = ClassificationRules::try_classify(name);
        if rule_hint.is_some() {
            tracing::info!("🚀 Rule Engine matched category/unit for: {}", name);
        }

        // 3. LLM (Gemini) — ALWAYS called for translations
        tracing::info!("🔮 Gemini LLM call for: {}", name);
        let start = Instant::now();

        let mut response = timeout(
            Duration::from_secs(15),
            self.gemini_service.process_unified(name),
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Unified processing took too long"))??;

        // Override category/unit with Rule Engine hints if available (more reliable)
        if let Some(rule) = rule_hint {
            tracing::info!(
                "📏 Overriding AI category '{}' with rule '{}'",
                response.category_slug,
                rule.category_slug
            );
            response.category_slug = rule.category_slug;
            response.unit = rule.unit;
            response.confidence = 1.0;
        }

        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("process_unified", duration_ms).await;

        let val = to_value(&response).map_err(|e| AppError::internal(e.to_string()))?;
        self.cache_repo
            .set(&cache_key, val, "gemini", "gemini-3-flash-preview", 90)
            .await?;

        Ok(response)
    }

    /// Pure translation (using cache)
    pub async fn translate_to_language(
        &self,
        text: &str,
        target_lang: &str,
    ) -> Result<String, AppError> {
        let cache_key = format!("translate:{}:{}", target_lang, text.to_lowercase().trim());

        if let Some(cached_val) = self.cache_repo.get(&cache_key).await? {
            if let Some(translated) = cached_val.as_str() {
                tracing::debug!("📦 Translation cache hit: {}", text);
                return Ok(translated.to_string());
            }
        }

        let start = Instant::now();
        let translated = timeout(
            Duration::from_secs(15),
            self.gemini_service.translate_to_language(text, target_lang),
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Text translation took too long"))??;

        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("translate_to_language", duration_ms).await;

        self.cache_repo
            .set(
                &cache_key,
                serde_json::Value::String(translated.clone()),
                "gemini",
                "gemini-3-flash-preview",
                90,
            )
            .await?;

        Ok(translated)
    }

    /// Analyze recipe (using cache)
    pub async fn analyze_recipe(&self, prompt: &str, recipe_id: &str) -> Result<String, AppError> {
        let cache_key = format!("recipe_analysis:{}", recipe_id);

        if let Some(cached_val) = self.cache_repo.get(&cache_key).await? {
            if let Some(analysis) = cached_val.as_str() {
                tracing::info!("📦 Recipe analysis cache hit: {}", recipe_id);
                return Ok(analysis.to_string());
            }
        }

        let start = Instant::now();
        let analysis = timeout(
            Duration::from_secs(30),
            self.gemini_service.analyze_recipe(prompt),
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Recipe analysis took too long"))??;

        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("analyze_recipe", duration_ms).await;

        self.cache_repo
            .set(
                &cache_key,
                serde_json::Value::String(analysis.clone()),
                "gemini",
                "gemini-3.1-pro-preview",
                30,
            )
            .await?;

        Ok(analysis)
    }

    /// Generate a dish photo using gemini-2.5-flash-image.
    /// Returns base64-encoded PNG (cached by dish slug for 90 days).
    pub async fn generate_dish_image(
        &self,
        dish_slug: &str,
        dish_name: &str,
        ingredients: &[String],
    ) -> Result<String, AppError> {
        let cache_key = format!("dish_image_v1:{}", dish_slug);

        // Check cache first — images are expensive to generate
        if let Some(cached_val) = self.cache_repo.get(&cache_key).await? {
            if let Some(b64) = cached_val.as_str() {
                tracing::info!(
                    "🖼 Dish image cache_hit=true slug={} | estimated cost: $0.000 / 0.00 PLN",
                    dish_slug
                );
                return Ok(b64.to_string());
            }
        }

        let start = Instant::now();
        let base64 = timeout(
            Duration::from_secs(65),
            self.gemini_service
                .generate_dish_image(dish_name, ingredients),
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Dish image generation took too long"))??;

        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("generate_dish_image", duration_ms).await;

        // Cache for 90 days — dish images rarely need refreshing
        self.cache_repo
            .set(
                &cache_key,
                serde_json::Value::String(base64.clone()),
                "gemini",
                "gemini-2.5-flash-image",
                90,
            )
            .await?;

        Ok(base64)
    }

    /// Generate a uniform white-background product packshot for the catalog.
    pub async fn generate_catalog_product_image(
        &self,
        cache_slug: &str,
        product_name: &str,
        description: Option<&str>,
    ) -> Result<String, AppError> {
        let cache_key = format!("catalog_product_image_v2:{}", cache_slug);
        if let Some(cached_val) = self.cache_repo.get(&cache_key).await? {
            if let Some(b64) = cached_val.as_str() {
                return Ok(b64.to_string());
            }
        }

        let start = Instant::now();
        let base64 = timeout(
            Duration::from_secs(65),
            self.gemini_service
                .generate_catalog_product_image(product_name, description),
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: catalog image generation took too long"))??;

        self.log_usage(
            "generate_catalog_product_image",
            start.elapsed().as_millis() as i32,
        )
        .await;
        self.cache_repo
            .set(
                &cache_key,
                serde_json::Value::String(base64.clone()),
                "gemini",
                "gemini-2.5-flash-image",
                90,
            )
            .await?;
        Ok(base64)
    }

    pub async fn generate_blog_article_image(
        &self,
        article_title: &str,
        scene: &str,
        variant: usize,
        enhanced: bool,
        reference_urls: &[String],
        scene_preset: &str,
        scale_direction: &str,
    ) -> Result<String, AppError> {
        let start = Instant::now();
        let base64 = timeout(
            Duration::from_secs(if enhanced { 125 } else { 65 }),
            self.gemini_service.generate_blog_article_image(
                article_title,
                scene,
                variant,
                enhanced,
                reference_urls,
                scene_preset,
                scale_direction,
            ),
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: article image generation took too long"))??;
        self.log_usage(
            if enhanced {
                "generate_blog_article_image_pro"
            } else {
                "generate_blog_article_image"
            },
            start.elapsed().as_millis() as i32,
        )
        .await;
        Ok(base64)
    }

    pub async fn generate_material_scene_image(
        &self,
        material_title: &str,
        material_text: &str,
        scene: &str,
    ) -> Result<String, AppError> {
        let start = Instant::now();
        let base64 = timeout(
            Duration::from_secs(65),
            self.gemini_service
                .generate_material_scene_image(material_title, material_text, scene),
        )
        .await
        .map_err(|_| {
            AppError::internal("LLM Timeout: material scene image generation took too long")
        })??;
        self.log_usage(
            "generate_material_scene_image",
            start.elapsed().as_millis() as i32,
        )
        .await;
        Ok(base64)
    }

    /// Raw AI request — no cache, no rule engine.
    /// Used for one-off admin operations like AI autofill.
    pub async fn groq_raw_request(
        &self,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<String, AppError> {
        self.groq_raw_request_with_model(prompt, max_tokens, "gemini-3.1-pro-preview")
            .await
    }

    /// Raw AI request with explicit model selection.
    /// Used by AiClient trait implementation for quality tiers.
    pub async fn groq_raw_request_with_model(
        &self,
        prompt: &str,
        max_tokens: u32,
        model: &str,
    ) -> Result<String, AppError> {
        let start = Instant::now();

        // Thinking models (Pro) sometimes leak chain-of-thought text into the output.
        // A system message with strict "JSON only" instruction reduces this.
        let messages = if model.contains("pro") {
            serde_json::json!([
                {"role": "system", "content": "You are a JSON API. Return ONLY valid JSON — no markdown, no explanations, no thinking, no commentary. If the user asks for an array, return [...]. If the user asks for an object, return {...}."},
                {"role": "user", "content": prompt}
            ])
        } else {
            serde_json::json!([{"role": "user", "content": prompt}])
        };

        let request_body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": 0.1,
            "max_tokens": max_tokens,
        });
        let result = tokio::time::timeout(
            Duration::from_secs(120),
            self.gemini_service.send_raw_request(&request_body),
        )
        .await
        .map_err(|_| AppError::internal(&format!("AI timeout (120s) for model {}", model)))??;
        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage(&format!("raw_{}", model), duration_ms).await;
        Ok(result)
    }

    pub async fn analyze_image_json(
        &self,
        prompt: &str,
        image_bytes: &[u8],
        mime_type: &str,
    ) -> Result<String, AppError> {
        let start = Instant::now();
        let result = timeout(
            Duration::from_secs(125),
            self.gemini_service
                .analyze_image_json(prompt, image_bytes, mime_type),
        )
        .await
        .map_err(|_| AppError::internal("AI timeout (125s) for Gemini Vision"))??;
        self.log_usage("gemini_vision_json", start.elapsed().as_millis() as i32)
            .await;
        Ok(result)
    }

    pub async fn analyze_images_json(
        &self,
        prompt: &str,
        images: &[(&[u8], &str)],
    ) -> Result<String, AppError> {
        let start = Instant::now();
        let result = timeout(
            Duration::from_secs(125),
            self.gemini_service.analyze_images_json(prompt, images),
        )
        .await
        .map_err(|_| AppError::internal("AI timeout (125s) for Gemini Vision"))??;
        self.log_usage("gemini_vision_json", start.elapsed().as_millis() as i32)
            .await;
        Ok(result)
    }
}
