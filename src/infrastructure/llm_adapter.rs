use crate::infrastructure::groq_service::{GroqService, UnifiedProductResponse, GroqTranslationResponse};
use crate::infrastructure::persistence::{AiCacheRepository, AiUsageStatsRepository};
use crate::domain::classification_rules::ClassificationRules;
use crate::shared::AppError;
use std::sync::Arc;
use serde_json::to_value;
use tokio::time::{timeout, Duration};
use std::time::Instant;

#[derive(Clone)]
pub struct LlmAdapter {
    groq_service: Arc<GroqService>,
    cache_repo: Arc<AiCacheRepository>,
    usage_repo: Arc<AiUsageStatsRepository>,
}

impl LlmAdapter {
    pub fn new(
        groq_service: Arc<GroqService>, 
        cache_repo: Arc<AiCacheRepository>,
        usage_repo: Arc<AiUsageStatsRepository>,
    ) -> Self {
        Self {
            groq_service,
            cache_repo,
            usage_repo,
        }
    }

    /// Helper to log usage stats
    async fn log_usage(&self, endpoint: &str, duration_ms: i32) {
        // In a real app, we'd extract tokens from the Groq response.
        // For now, we log the duration and endpoint.
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
        // SLA: 3 seconds max for translation
        let response = timeout(
            Duration::from_secs(3), 
            self.groq_service.translate(ingredient_name)
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Translation took too long"))??;
        
        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("translate_all", duration_ms).await;
        
        let val = to_value(&response).map_err(|e| AppError::internal(e.to_string()))?;
        self.cache_repo.set(&cache_key, val, "groq", "llama-3.1-8b-instant", 90).await?;

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
                name_pl: name.to_string(), // In a real app we might still want translations
                name_ru: name.to_string(), 
                name_uk: name.to_string(),
                category_slug: rule.category_slug,
                unit: rule.unit,
                confidence: 1.0, // Rule engine is 100% confident
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

        // 3. LLM (SLOWEST, costs money)
        tracing::info!("🤖 LLM call for: {}", name);
        let start = Instant::now();
        
        // SLA: 4 seconds max for unified processing
        let response = timeout(
            Duration::from_secs(4),
            self.groq_service.process_unified(name)
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Unified processing took too long"))??;

        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("process_unified", duration_ms).await;

        // 4. Store in Cache (TTL: 90 days)
        let val = to_value(&response).map_err(|e| AppError::internal(e.to_string()))?;
        self.cache_repo.set(&cache_key, val, "groq", "llama-3.1-8b-instant", 90).await?;

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
        // SLA: 5 seconds max for long text translation
        let translated = timeout(
            Duration::from_secs(5),
            self.groq_service.translate_to_language(text, target_lang)
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Text translation took too long"))??;
        
        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("translate_to_language", duration_ms).await;
        
        self.cache_repo.set(&cache_key, serde_json::Value::String(translated.clone()), "groq", "llama-3.1-8b-instant", 90).await?;

        Ok(translated)
    }

    /// Analyze recipe (using cache)
    pub async fn analyze_recipe(&self, prompt: &str, recipe_id: &str) -> Result<String, AppError> {
        // For recipe analysis, we use recipe_id as part of the key to avoid re-analyzing the same version
        let cache_key = format!("recipe_analysis:{}", recipe_id);

        if let Some(cached_val) = self.cache_repo.get(&cache_key).await? {
            if let Some(analysis) = cached_val.as_str() {
                tracing::info!("📦 Recipe analysis cache hit: {}", recipe_id);
                return Ok(analysis.to_string());
            }
        }

        let start = Instant::now();
        // SLA: 10 seconds max for complex recipe analysis
        let analysis = timeout(
            Duration::from_secs(10),
            self.groq_service.analyze_recipe(prompt)
        )
        .await
        .map_err(|_| AppError::internal("LLM Timeout: Recipe analysis took too long"))??;
        
        let duration_ms = start.elapsed().as_millis() as i32;
        self.log_usage("analyze_recipe", duration_ms).await;
        
        self.cache_repo.set(&cache_key, serde_json::Value::String(analysis.clone()), "groq", "llama-3.1-8b-instant", 30).await?;

        Ok(analysis)
    }
}
