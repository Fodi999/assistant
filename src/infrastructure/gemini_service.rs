use crate::shared::AppError;
use base64::Engine;
use serde::Deserialize;
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
    /// High-volume recipe and CMS image model.
    recipe_image_model: String,
    /// Premium hero/cover image model.
    recipe_hero_image_model: String,
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
            recipe_image_model: std::env::var("GEMINI_RECIPE_IMAGE_MODEL")
                .unwrap_or_else(|_| "gemini-3.1-flash-image".to_string()),
            recipe_hero_image_model: std::env::var("GEMINI_RECIPE_HERO_IMAGE_MODEL")
                .unwrap_or_else(|_| "gemini-3-pro-image".to_string()),
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
            &trimmed[7..] // skip "```json"
        } else if trimmed.starts_with("```") {
            &trimmed[3..] // skip "```"
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
            tracing::warn!(
                "Gemini returned empty PL translation for: {}",
                ingredient_name
            );
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
            trimmed,
            result.name_en,
            result.category_slug,
            result.unit
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
            classification.category_slug,
            classification.unit
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

    /// Generate a dish photo using gemini-2.5-flash-image (native Gemini API).
    /// Returns base64-encoded PNG bytes (ready for data:image/png;base64,... URL).
    pub async fn generate_dish_image(
        &self,
        dish_name: &str,
        ingredients: &[String],
    ) -> Result<String, AppError> {
        let ingredients_hint = if ingredients.is_empty() {
            String::new()
        } else {
            format!(", made with {}", ingredients.join(", "))
        };

        let prompt = format!(
            "Professional food photography of {}{}: beautifully plated, natural lighting, \
             shallow depth of field, rustic wooden table background, restaurant quality, \
             appetizing and vibrant colors. No text, no watermarks.",
            dish_name, ingredients_hint
        );

        self.generate_image_from_prompt(&prompt, dish_name, "dish", &self.recipe_image_model)
            .await
    }

    /// Generate a consistent isolated product photo for catalog cards.
    pub async fn generate_catalog_product_image(
        &self,
        product_name: &str,
        description: Option<&str>,
    ) -> Result<String, AppError> {
        let description_hint = description
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("\nProduct identification context: {}.", value.trim()))
            .unwrap_or_default();
        let prompt = format!(
            r#"Create a premium ecommerce catalog photograph of the single food ingredient: "{product_name}".{description_hint}

CATALOG COMPOSITION STANDARD:
- pure seamless white background (#FFFFFF), including all corners
- one centered hero arrangement of only this ingredient, fully visible
- camera at a consistent slightly elevated three-quarter angle, about 25 degrees
- product occupies approximately 65% of the square frame with generous even white margins
- realistic natural proportions and color, crisp texture, soft diffused studio light
- subtle soft contact shadow directly beneath the product, no horizon line
- photorealistic commercial packshot, high detail, clean color calibration

STRICTLY EXCLUDE:
- plates, bowls, boards, packaging, labels, utensils, hands, people, table surfaces
- decorative props, unrelated ingredients, herbs, sauces, text, logos, watermarks
- dramatic shadows, colored backgrounds, gradients, cropped product, floating objects

Return one consistent square catalog image. The ingredient must be immediately recognizable."#,
            product_name = product_name,
            description_hint = description_hint,
        );

        self.generate_image_from_prompt(
            &prompt,
            product_name,
            "catalog product",
            &self.recipe_image_model,
        )
        .await
    }

    /// Generate one editorial image variant for a CMS article.
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
        let role = match variant {
            0 => "wide editorial hero cover",
            1 => "professional step-by-step process scene",
            2 => "tight macro detail",
            3 => "finished result in an elegant editorial composition",
            _ => "chronological recipe or technique step in a visual editorial series",
        };
        let scene_style = match scene_preset {
            "product-white" => "Ecommerce product packshot: isolate the subject on a pure seamless white #FFFFFF background, centered, fully visible, soft contact shadow, no props or table.",
            "recipe-table" => "Finished recipe on a tasteful natural table setting, warm restaurant-quality daylight, restrained relevant tableware, appetizing and realistic.",
            "home-interior" => "Lifestyle scene in a refined modern home kitchen or dining interior, natural window light, believable domestic atmosphere, subject remains dominant.",
            "cooking-process" => "Instructional cooking-process photograph showing one clear chronological action, clean workstation, hands only when needed, technique easy to understand.",
            "restaurant-plating" => "Premium restaurant plating on elegant tableware, controlled fine-dining light, precise composition, minimal sophisticated background.",
            "object-interior" => "Editorial object photograph in a modern home interior, realistic scale and materials, soft daylight, curated but uncluttered surroundings.",
            _ => "Premium culinary editorial scene with a clear subject, modern composition and realistic context.",
        };
        let prompt = format!(
            r#"Create a premium culinary magazine photograph for the article "{article_title}".
Image role: {role}.
Scene direction: {scene}.
Scene preset: {scene_style}
Scale and realism constraints: {scale_direction}

STYLE STANDARD:
- photorealistic professional editorial food photography
- clean modern 2026 culinary magazine aesthetic
- soft natural daylight, realistic color, controlled highlights
- intentional composition with clear subject and generous visual breathing room
- landscape 16:9 composition, suitable for a blog article
- visually consistent with a four-image editorial story
- preserve believable real-world proportions and perspective
- when a scale reference is requested, include it naturally and keep its standard size recognizable
- use the supplied physical dimensions as strict visual constraints; do not make the subject oversized or miniature

STRICTLY EXCLUDE:
- any text, letters, captions, logos, watermarks or UI
- rulers, dimension arrows, measurement labels or technical annotations
- distorted food, duplicate tools, impossible hands, clutter, stock-photo look
- unrelated ingredients or decorative elements that do not support the topic"#,
            article_title = article_title,
            role = role,
            scene = scene,
            scene_style = scene_style,
            scale_direction = scale_direction,
        );
        let model = if enhanced {
            &self.recipe_hero_image_model
        } else {
            &self.recipe_image_model
        };
        self.generate_image_from_prompt_with_references(
            &prompt,
            article_title,
            "blog article",
            model,
            reference_urls,
        )
        .await
    }

    async fn generate_image_from_prompt(
        &self,
        prompt: &str,
        subject_name: &str,
        image_kind: &str,
        model: &str,
    ) -> Result<String, AppError> {
        self.generate_image_from_prompt_with_references(
            prompt,
            subject_name,
            image_kind,
            model,
            &[],
        )
        .await
    }

    async fn generate_image_from_prompt_with_references(
        &self,
        prompt: &str,
        subject_name: &str,
        image_kind: &str,
        model: &str,
        reference_urls: &[String],
    ) -> Result<String, AppError> {
        let mut parts = vec![serde_json::json!({"text": prompt})];
        for reference_url in reference_urls.iter().take(2) {
            let response = self
                .http_client
                .get(reference_url)
                .send()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to load reference image: {}", e))
                })?;
            if !response.status().is_success() {
                return Err(AppError::validation(
                    "Reference image is not publicly accessible",
                ));
            }
            let mime_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .filter(|value| value.starts_with("image/"))
                .unwrap_or("image/jpeg")
                .to_string();
            let bytes = response.bytes().await.map_err(|e| {
                AppError::internal(format!("Failed to read reference image: {}", e))
            })?;
            if bytes.len() > 10 * 1024 * 1024 {
                return Err(AppError::validation(
                    "Reference image must be smaller than 10 MB",
                ));
            }
            parts.push(serde_json::json!({
                "inlineData": {
                    "mimeType": mime_type,
                    "data": base64::engine::general_purpose::STANDARD.encode(bytes)
                }
            }));
        }
        let body = serde_json::json!({
            "contents": [{"parts": parts}],
            "generationConfig": {"responseModalities": ["IMAGE", "TEXT"]}
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        tracing::info!(
            "🎨 Generating {} image for: {} (model={})",
            image_kind,
            subject_name,
            model
        );

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(if model.contains("pro") { 120 } else { 60 }),
            self.http_client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send(),
        )
        .await
        .map_err(|_| AppError::internal("Timeout: dish image generation took too long"))?
        .map_err(|e| AppError::internal(&format!("Image generation request failed: {}", e)))?;

        let status = result.status();
        if !status.is_success() {
            let err = result.text().await.unwrap_or_default();
            tracing::error!(
                "❌ Image generation failed (HTTP {}): {}",
                status,
                &err[..err.len().min(200)]
            );
            return Err(AppError::internal(&format!(
                "Image generation API error: {}",
                status
            )));
        }

        let json: serde_json::Value = result
            .json()
            .await
            .map_err(|e| AppError::internal(&format!("Failed to parse image response: {}", e)))?;

        // Extract base64 from candidates[0].content.parts[].inlineData.data
        let base64 = json
            .pointer("/candidates/0/content/parts")
            .and_then(|parts| parts.as_array())
            .and_then(|parts| {
                parts.iter().find_map(|p| {
                    p.pointer("/inlineData/data")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string())
                })
            })
            .ok_or_else(|| {
                tracing::error!(
                    "❌ No image data in Gemini response: {:?}",
                    json.pointer("/candidates/0/content/parts")
                );
                AppError::internal("No image data in Gemini response")
            })?;

        // Log token usage from usageMetadata
        if let Some(usage) = json.pointer("/usageMetadata") {
            let prompt_tokens = usage
                .pointer("/promptTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let output_tokens = usage
                .pointer("/candidatesTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let total_tokens = usage
                .pointer("/totalTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            // Gemini 2.5 Flash pricing (as of 2025):
            //   text input  = $0.15 / 1M tokens
            //   image output = $0.039 per image (fixed, billed as output tokens ~1272 tokens)
            let input_cost_usd = (prompt_tokens as f64) * 0.15 / 1_000_000.0;
            let output_cost_usd = (output_tokens as f64) * 0.039 / 1272.0; // ~$0.039 per image
            let total_cost_usd = input_cost_usd + output_cost_usd;
            let total_cost_pln = total_cost_usd * 4.05; // approx USD→PLN
            tracing::info!(
                "🪙 Dish image tokens for '{}': prompt={} output={} total={} | cache_hit=false | estimated cost: ${:.4} / {:.2} PLN",
                subject_name, prompt_tokens, output_tokens, total_tokens,
                total_cost_usd, total_cost_pln
            );
        } else {
            // No usageMetadata — log fixed estimate (~1 image = $0.039)
            let cost_usd = 0.039_f64;
            let cost_pln = cost_usd * 4.05;
            tracing::info!(
                "🪙 Dish image '{}': cache_hit=false | estimated cost: ${:.4} / {:.2} PLN (no usageMetadata)",
                subject_name, cost_usd, cost_pln
            );
        }

        tracing::info!(
            "✅ {} image generated for '{}' ({} base64 chars)",
            image_kind,
            subject_name,
            base64.len()
        );
        Ok(base64)
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
                Err(AppError::internal(&format!(
                    "Gemini API timeout ({}s)",
                    timeout_secs
                )))
            }
        }
    }

    /// Core HTTP call to Gemini OpenAI-compatible endpoint
    async fn send_gemini_request_inner(
        &self,
        request_body: &serde_json::Value,
    ) -> Result<String, AppError> {
        if self.api_key.trim().is_empty() {
            return Err(AppError::validation(
                "Gemini is not configured on the backend. Set GEMINI_API_KEY.",
            ));
        }

        tracing::debug!(
            "📤 Sending Gemini request: model={}",
            request_body
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
        );

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

        let choice = data.choices.get(0).ok_or_else(|| {
            tracing::error!("❌ Gemini returned empty choices array");
            AppError::internal("No response from Gemini")
        })?;

        let finish_reason = choice.finish_reason.as_deref().unwrap_or("unknown");
        if finish_reason == "length" {
            let preview = choice.message.content.as_deref().unwrap_or("");
            let safe_end = preview
                .char_indices()
                .nth(120)
                .map(|(i, _)| i)
                .unwrap_or(preview.len());
            tracing::warn!(
                "⚠️ Gemini output truncated (finish_reason=length) model={} content_preview={}",
                request_body
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?"),
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
                .strip_prefix("```json")
                .or_else(|| content.trim().strip_prefix("```"))
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
                        tracing::debug!(
                            "Extracted JSON from response: {}…",
                            &json_str[..json_str.len().min(200)]
                        );
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

        let cats = [
            "dairy_and_eggs",
            "fruits",
            "vegetables",
            "meat",
            "seafood",
            "grains",
            "beverages",
        ];
        if !cats.contains(&r.category_slug.as_str()) {
            return Err(AppError::validation(&format!(
                "Invalid category: {}",
                r.category_slug
            )));
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
