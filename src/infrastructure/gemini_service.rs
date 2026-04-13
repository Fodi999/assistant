use crate::shared::AppError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Re-export types from groq_service for backward compatibility ─────────────
// These types are used throughout the codebase; keeping the same shape
// means zero changes in application layer.
pub use crate::infrastructure::groq_service::{
    AiClassification, GroqTranslationResponse, UnifiedProductResponse,
};

/// Google Gemini API service via OpenAI-compatible endpoint.
///
/// Drop-in replacement for GroqService — same public interface,
/// backed by `https://generativelanguage.googleapis.com/v1beta/openai/`.
#[derive(Clone)]
pub struct GeminiService {
    api_key: String,
    http_client: reqwest::Client,
    /// Fast model for translations & simple tasks
    fast_model: String,
    /// Smart model for complex generation (SEO, autofill, analysis)
    smart_model: String,
}

impl GeminiService {
    pub fn new(api_key: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120)) // SEO/measure prompts need 60-90s on Gemini
            .build()
            .expect("Failed to build HTTP client for Gemini");

        Self {
            api_key,
            http_client,
            fast_model: "gemini-3-flash-preview".to_string(),
            smart_model: "gemini-3.1-pro-preview".to_string(),
        }
    }

    // ── Public API (same signatures as GroqService) ─────────────────────────

    /// Check if input is likely English (ASCII letters + digits + basic punctuation)
    fn is_likely_english(text: &str) -> bool {
        if text.trim().is_empty() {
            return false;
        }
        text.chars()
            .all(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || c == '-' || c == '\'')
    }

    /// Strip markdown code fences that Gemini 3 thinking models add around JSON.
    /// Handles ```json\n...\n```, ```\n...\n```, and nested variations.
    fn strip_markdown_fences(text: &str) -> String {
        let trimmed = text.trim();
        // Check for ```json or ``` prefix
        let without_prefix = if trimmed.starts_with("```json") {
            &trimmed[7..]  // skip "```json"
        } else if trimmed.starts_with("```") {
            &trimmed[3..]  // skip "```"
        } else {
            return trimmed.to_string();
        };
        // Strip trailing ```
        let without_suffix = if without_prefix.trim_end().ends_with("```") {
            let s = without_prefix.trim_end();
            &s[..s.len() - 3]
        } else {
            without_prefix
        };
        without_suffix.trim().to_string()
    }

    /// Normalize any-language input to English
    pub async fn normalize_to_english(&self, input: &str) -> Result<String, AppError> {
        let trimmed = input.trim();
        if Self::is_likely_english(trimmed) {
            return Ok(trimmed.to_string());
        }
        self.translate_to_language(trimmed, "English").await
    }

    /// Translate ingredient name → PL, RU, UK
    pub async fn translate(
        &self,
        ingredient_name: &str,
    ) -> Result<GroqTranslationResponse, AppError> {
        if ingredient_name.len() > 50 {
            return Err(AppError::validation(
                "Ingredient name too long for automatic translation",
            ));
        }

        let prompt = format!(
            r#"Translate "{}" to Polish(pl), Russian(ru), Ukrainian(uk).
Respond with ONLY valid JSON, no other text:
{{"pl":"<Polish>","ru":"<Russian>","uk":"<Ukrainian>"}}"#,
            ingredient_name
        );

        // Thinking models (gemini-3-flash) spend ~80% of max_tokens on chain-of-thought,
        // so we need much higher limits than the expected output size.
        let body = self.build_request(&self.fast_model, &prompt, 0.0, 2000);

        tracing::info!("🔮 Gemini translation request for: {}", ingredient_name);

        let content = self.send_with_retry(&body, 1).await?;

        let translation: GroqTranslationResponse = self.parse_json_response(&content)?;

        if translation.pl.trim().is_empty() {
            tracing::warn!("Gemini returned empty PL translation for: {}", ingredient_name);
        }

        tracing::info!("✅ Gemini translation successful for: {}", ingredient_name);
        Ok(translation)
    }

    /// Translate text to a target language
    pub async fn translate_to_language(
        &self,
        text: &str,
        target_lang: &str,
    ) -> Result<String, AppError> {
        if text.len() > 5000 {
            return Err(AppError::validation("Text too long for translation"));
        }

        let prompt = format!(
            r#"Translate the following text to {}.
Return ONLY the translated text, nothing else.

Text: {}"#,
            target_lang, text
        );

        let body = self.build_request(&self.fast_model, &prompt, 0.0, 2000);

        let content = self.send_with_retry(&body, 1).await?;

        let cleaned = content
            .trim()
            .trim_end_matches('.')
            .trim_end_matches(',')
            .trim()
            .to_string();

        Ok(cleaned)
    }

    /// Analyze recipe — generate insights
    pub async fn analyze_recipe(&self, prompt: &str) -> Result<String, AppError> {
        if prompt.len() > 10000 {
            return Err(AppError::validation("Prompt too long for AI analysis"));
        }

        let body = self.build_request(&self.smart_model, prompt, 0.3, 4000);

        tracing::info!("🔮 Requesting recipe analysis from Gemini AI");

        let content = self.send_with_retry(&body, 1).await?;
        tracing::debug!("🔮 Received AI analysis ({} chars)", content.len());
        Ok(content)
    }

    /// Unified classification + translation (single call)
    pub async fn process_unified(
        &self,
        name_input: &str,
    ) -> Result<UnifiedProductResponse, AppError> {
        let trimmed = name_input.trim();
        if trimmed.is_empty() {
            return Err(AppError::validation("Input cannot be empty"));
        }

        let prompt = format!(
            r#"You are a food product data extraction and classification AI.

Input product name (may be in ANY language): "{}"

Extract and classify the product. Return ONLY valid JSON, no other text:
{{
  "name_en": "<English product name>",
  "name_pl": "<Polish translation>",
  "name_ru": "<Russian translation>",
  "name_uk": "<Ukrainian translation>",
  "category_slug": "<category>",
  "unit": "<unit>",
  "confidence": 0.95
}}

Categories: dairy_and_eggs, fruits, vegetables, meat, seafood, grains, beverages
Units: piece, kilogram, gram, liter, milliliter

Rules:
1. name_en MUST be in English (translate if needed)
2. All translations must be single words when possible, but allow 2-3 word compounds
3. category_slug must be one of the allowed values
4. unit must be one of the allowed values
5. confidence must be a float between 0.0 and 1.0 indicating how sure you are about the classification
6. Do not add explanations, just JSON"#,
            trimmed
        );

        let body = self.build_request(&self.fast_model, &prompt, 0.0, 4000);

        tracing::info!("🔮 Gemini unified processing for: {}", trimmed);

        let content = self.send_with_retry(&body, 1).await?;

        let result: UnifiedProductResponse = self.parse_json_response(&content)?;

        self.validate_unified_response(&result)?;

        tracing::info!(
            "✅ Gemini unified OK: {} → en={}, cat={}, unit={}",
            trimmed, result.name_en, result.category_slug, result.unit
        );

        Ok(result)
    }

    /// AI classification (legacy, kept for backward compat)
    pub async fn classify_product(&self, name_en: &str) -> Result<AiClassification, AppError> {
        if name_en.len() > 50 {
            return Err(AppError::validation(
                "Product name too long for classification",
            ));
        }

        let prompt = format!(
            r#"You are a food classification AI.

Given product name: "{}"

Return ONLY valid JSON (no other text):
{{"category_slug":"","unit":""}}

Allowed categories: dairy_and_eggs, fruits, vegetables, meat, seafood, grains, beverages
Allowed units: piece, kilogram, gram, liter, milliliter

Pick the best match. Do not invent values."#,
            name_en
        );

        let body = self.build_request(&self.fast_model, &prompt, 0.0, 2000);
        let content = self.send_with_retry(&body, 1).await?;
        let classification: AiClassification = self.parse_json_response(&content)?;

        tracing::info!(
            "✅ Gemini classification: cat={}, unit={}",
            classification.category_slug, classification.unit
        );

        Ok(classification)
    }

    /// Raw request — bypasses cache, used for admin AI autofill
    pub async fn send_raw_request(
        &self,
        request_body: &serde_json::Value,
    ) -> Result<String, AppError> {
        // Override the URL to point to Gemini, but use caller's model/messages/etc.
        self.send_gemini_request_inner(request_body).await
    }

    // ── Internal helpers ────────────────────────────────────────────────────

    fn build_request(
        &self,
        model: &str,
        prompt: &str,
        temperature: f64,
        max_tokens: u32,
    ) -> serde_json::Value {
        serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": temperature,
            "max_tokens": max_tokens,
        })
    }

    async fn send_with_retry(
        &self,
        body: &serde_json::Value,
        max_retries: u32,
    ) -> Result<String, AppError> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match self.send_gemini_request(body).await {
                Ok(content) => return Ok(content),
                Err(e) if attempt <= max_retries => {
                    tracing::warn!("🔮 Gemini attempt {} failed, retrying…", attempt);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn send_gemini_request(
        &self,
        request_body: &serde_json::Value,
    ) -> Result<String, AppError> {
        // Pro (thinking) models need more time — chain-of-thought adds latency.
        // Flash: ~10-20s, Pro: ~30-60s.
        let timeout_secs = if request_body
            .get("model")
            .and_then(|m| m.as_str())
            .map(|m| m.contains("pro"))
            .unwrap_or(false)
        {
            90
        } else {
            45
        };

        let result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            self.send_gemini_request_inner(request_body),
        )
        .await;

        match result {
            Ok(Ok(content)) => Ok(content),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                tracing::error!("🔮 Gemini request timeout ({}s exceeded)", timeout_secs);
                Err(AppError::internal(&format!("Gemini API timeout ({}s)", timeout_secs)))
            }
        }
    }

    /// Core HTTP call to Gemini OpenAI-compatible endpoint
    async fn send_gemini_request_inner(
        &self,
        request_body: &serde_json::Value,
    ) -> Result<String, AppError> {
        tracing::debug!("📤 Sending Gemini request: model={}", 
            request_body.get("model").and_then(|v| v.as_str()).unwrap_or("?"));

        let response = self
            .http_client
            .post("https://generativelanguage.googleapis.com/v1beta/openai/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("❌ Gemini API request failed: {:?}", e);
                AppError::internal(&format!("Gemini API error: {}", e))
            })?;

        let status = response.status();
        tracing::debug!("📥 Gemini response status: {}", status);

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());
            tracing::error!("❌ Gemini API error (HTTP {}): {}", status, error_body);
            return Err(AppError::internal(&format!(
                "Gemini API returned error: {} - {}",
                status, error_body
            )));
        }

        let data: GeminiResponse = response.json().await.map_err(|e| {
            tracing::error!("❌ Failed to parse Gemini JSON response: {:?}", e);
            AppError::internal(&format!("Failed to parse Gemini response: {}", e))
        })?;

        let choice = data
            .choices
            .get(0)
            .ok_or_else(|| {
                tracing::error!("❌ Gemini returned empty choices array");
                AppError::internal("No response from Gemini")
            })?;

        let finish_reason = choice.finish_reason.as_deref().unwrap_or("unknown");
        if finish_reason == "length" {
            let preview = choice.message.content.as_deref().unwrap_or("");
            let safe_end = preview.char_indices().nth(120).map(|(i, _)| i).unwrap_or(preview.len());
            tracing::warn!(
                "⚠️ Gemini output truncated (finish_reason=length) model={} content_preview={}",
                request_body.get("model").and_then(|v| v.as_str()).unwrap_or("?"),
                &preview[..safe_end]
            );
        }

        let content = choice
            .message
            .content
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();

        if content.is_empty() {
            tracing::error!("❌ Gemini returned empty/null content (finish_reason={}, thinking model may need higher max_tokens)", finish_reason);
            return Err(AppError::internal("Gemini returned empty response"));
        }

        // Gemini 3 thinking models often wrap JSON in ```json code blocks.
        // Strip them at the source so every caller gets clean content.
        let content = Self::strip_markdown_fences(&content);

        tracing::debug!("✅ Gemini response content: {} chars", content.len());
        Ok(content)
    }

    /// Parse JSON from AI response with fallback extraction.
    /// Handles Gemini 3's tendency to wrap JSON in ```json code blocks.
    fn parse_json_response<T: serde::de::DeserializeOwned>(
        &self,
        content: &str,
    ) -> Result<T, AppError> {
        // Step 1: Strip markdown code fences (```json ... ``` or ``` ... ```)
        let cleaned = if content.contains("```") {
            content
                .trim()
                .strip_prefix("```json").or_else(|| content.trim().strip_prefix("```"))
                .unwrap_or(content)
                .trim()
                .strip_suffix("```")
                .unwrap_or(content)
                .trim()
        } else {
            content.trim()
        };

        // Step 2: Try direct parse
        serde_json::from_str(cleaned)
            .or_else(|_| {
                // Step 3: Fallback — extract JSON object from surrounding text
                if let Some(start) = cleaned.find('{') {
                    if let Some(end) = cleaned.rfind('}') {
                        let json_str = &cleaned[start..=end];
                        tracing::debug!("Extracted JSON from response: {}…", &json_str[..json_str.len().min(200)]);
                        return serde_json::from_str(json_str);
                    }
                }
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No JSON found in response",
                )))
            })
            .map_err(|e| {
                tracing::error!("Failed to parse AI JSON response: {}", e);
                tracing::debug!("Raw response: {}", &content[..content.len().min(500)]);
                AppError::internal("Invalid AI response format")
            })
    }

    fn validate_unified_response(&self, r: &UnifiedProductResponse) -> Result<(), AppError> {
        if r.name_en.trim().is_empty() {
            return Err(AppError::internal("AI returned empty English name"));
        }

        let cats = ["dairy_and_eggs", "fruits", "vegetables", "meat", "seafood", "grains", "beverages"];
        if !cats.contains(&r.category_slug.as_str()) {
            return Err(AppError::validation(&format!("Invalid category: {}", r.category_slug)));
        }

        let units = ["piece", "kilogram", "gram", "liter", "milliliter"];
        if !units.contains(&r.unit.as_str()) {
            return Err(AppError::validation(&format!("Invalid unit: {}", r.unit)));
        }

        Ok(())
    }
}

// ── OpenAI-compatible response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    choices: Vec<GeminiChoice>,
}

#[derive(Debug, Deserialize)]
struct GeminiChoice {
    message: GeminiMessage,
    /// "stop", "length", "content_filter", etc.
    #[serde(default)]
    finish_reason: Option<String>,
}

/// Gemini 3 thinking models may return `content: null` while including
/// thought_signature in extra_content. We handle that gracefully.
#[derive(Debug, Deserialize)]
struct GeminiMessage {
    #[serde(default)]
    content: Option<String>,
}
