use crate::infrastructure::gemini_service::GeminiService;
use crate::infrastructure::groq_service::{UnifiedProductResponse, GroqTranslationResponse};
use crate::infrastructure::persistence::{AiCacheRepository, AiUsageStatsRepository};
use crate::domain::classification_rules::ClassificationRules;
use crate::shared::AppError;
use std::sync::Arc;
use serde_json::to_value;
use tokio::time::{timeout, Duration};
use std::time::Instant;

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
        let _ = self.usage_repo.log_usage(endpoint, 0, 0, 0, duration_ms).await;
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
            self.gemini_service.translate(ingredient_name)
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Translation took too long"))??;
        
        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("translate_all", duration_ms).await;
        
        let val = to_value(&response).map_err(|e| AppError::internal(e.to_string()))?;
        self.cache_repo.set(&cache_key, val, "gemini", "gemini-2.5-flash", 90).await?;

        Ok(response)
    }

    /// Unified classification and translation for a product.
    /// Order: Rule Engine -> Cache -> LLM
    pub async fn process_unified(
        &self,
        name: &str,
    ) -> Result<UnifiedProductResponse, AppError> {
        // 1. Rule Engine (FASTEST, 0 costs)
        if let Some(rule) = ClassificationRules::try_classify(name) {
            tracing::info!("🚀 Rule Engine matched: {}", name);
            return Ok(UnifiedProductResponse {
                name_en: name.to_string(),
                name_pl: name.to_string(),
                name_ru: name.to_string(), 
                name_uk: name.to_string(),
                category_slug: rule.category_slug,
                unit: rule.unit,
                confidence: 1.0,
            });
        }

        // 2. Cache (FAST, low latency)
        let cache_key = format!("product_unified:{}", name.to_lowercase().trim());
        if let Some(cached_val) = self.cache_repo.get(&cache_key).await? {
            if let Ok(response) = serde_json::from_value::<UnifiedProductResponse>(cached_val) {
                tracing::info!("📦 Cache hit: {}", name);
                return Ok(response);
            }
        }

        // 3. LLM (Gemini)
        tracing::info!("🔮 Gemini LLM call for: {}", name);
        let start = Instant::now();
        
        let response = timeout(
            Duration::from_secs(15),
            self.gemini_service.process_unified(name)
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Unified processing took too long"))??;

        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("process_unified", duration_ms).await;

        let val = to_value(&response).map_err(|e| AppError::internal(e.to_string()))?;
        self.cache_repo.set(&cache_key, val, "gemini", "gemini-2.5-flash", 90).await?;

        Ok(response)
    }

    /// Pure translation (using cache)
    pub async fn translate_to_language(&self, text: &str, target_lang: &str) -> Result<String, AppError> {
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
            self.gemini_service.translate_to_language(text, target_lang)
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Text translation took too long"))??;
        
        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("translate_to_language", duration_ms).await;
        
        self.cache_repo.set(&cache_key, serde_json::Value::String(translated.clone()), "gemini", "gemini-2.5-flash", 90).await?;

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
            self.gemini_service.analyze_recipe(prompt)
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Recipe analysis took too long"))??;
        
        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("analyze_recipe", duration_ms).await;
        
        self.cache_repo.set(&cache_key, serde_json::Value::String(analysis.clone()), "gemini", "gemini-2.5-pro", 30).await?;
        
        Ok(analysis)
    }

    /// Raw AI request — no cache, no rule engine.
    /// Used for one-off admin operations like AI autofill.
    pub async fn groq_raw_request(&self, prompt: &str, max_tokens: u32) -> Result<String, AppError> {
        self.groq_raw_request_with_model(prompt, max_tokens, "gemini-2.5-pro").await
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
        let request_body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.1,
            "max_tokens": max_tokens,
        });
        let result = tokio::time::timeout(
            Duration::from_secs(60),
            self.gemini_service.send_raw_request(&request_body),
        )
        .await
        .map_err(|_| AppError::internal(&format!("AI timeout (60s) for model {}", model)))?? ;
        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage(&format!("raw_{}", model), duration_ms).await;
        Ok(result)
    }
}