//! Gemini Vision adapter for Laboratory v2.
//!
//! Single responsibility: take raw image bytes + mime type → return a
//! `Product3DSpec` (the contract between Vision and the geometry generators).
//!
//! No model/OBJ generation here — that lives in PR #4.
//!
//! ## Prompt design
//!
//! We force Gemini to behave like a strict JSON producer:
//!   * no prose, no markdown fences;
//!   * choose `unknown` if uncertain (we'll fall back to `flat_card` server-side);
//!   * keep visual params conservative (one dominant colour, viscosity 0..1, …).
//!
//! ## Robustness
//!
//! Gemini will sometimes still wrap JSON in ```json fences or add a
//! trailing comment. We strip fences and `serde_json::from_str` the rest.
//! On parse failure we surface the raw text in the error message so it
//! ends up in tracing logs and a future failed-asset row.

use std::time::Duration;

use base64::Engine;
use serde::Serialize;

use crate::application::laboratory_v2::Product3DSpec;
use crate::shared::AppError;

/// Gemini multimodal model that supports image input.
const VISION_MODEL: &str = "gemini-2.0-flash-exp";

/// System prompt — deterministic, no creativity.
const VISION_PROMPT: &str = r##"Analyze this food/product image and return a strict JSON specification for a procedural 3D prototype generator.

Do not describe the image in prose. Do not return markdown. Return only JSON matching the schema.

The generator supports these object types:
- "sauce_in_bowl"  — sauce / soup / cream / yogurt inside an open container
- "bottled_sauce"  — liquid in a bottle (ketchup, soy, oil, …)
- "jar_product"    — paste / preserve in a jar (jam, honey, mustard, …)
- "plate_food"     — solid food on a plate (steak, pasta, salad, …)
- "flat_card"      — packaged product, label, anything where 3D shape is irrelevant
- "unknown"        — choose this if the image is unclear

If the object is ambiguous, choose "unknown". Estimate visual parameters conservatively.

Schema (all keys required unless marked optional):
{
  "object_type": "sauce_in_bowl" | "bottled_sauce" | "jar_product" | "plate_food" | "flat_card" | "unknown",
  "confidence": 0.0..1.0,
  "container": {                      // optional — omit for flat_card / unknown
    "kind": "ceramic_bowl" | "glass_bottle" | "glass_jar" | "white_plate" | "...",
    "color_hex": "#RRGGBB",           // optional
    "diameter_mm": number,            // optional, rough estimate
    "height_mm": number               // optional, rough estimate
  },
  "product": {
    "color_hex": "#RRGGBB",           // dominant colour of the product itself
    "viscosity": 0.0..1.0,            // optional, only for liquids/sauces (0=water, 1=paste)
    "gloss": 0.0..1.0,                // optional (0=matte, 1=mirror)
    "description": "short phrase"     // optional, e.g. "thick red sauce with herb specks"
  },
  "scene": {                          // optional
    "background": "...",
    "lighting": "..."
  }
}

Return ONLY this JSON. No backticks. No commentary."##;

#[derive(Clone)]
pub struct GeminiVision3D {
    api_key: String,
    http_client: reqwest::Client,
    model: String,
}

impl GeminiVision3D {
    pub fn new(api_key: String) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client for Gemini Vision 3D");

        Self {
            api_key,
            http_client,
            model: VISION_MODEL.to_string(),
        }
    }

    /// Send `image_bytes` (already-decoded raw bytes) to Gemini Vision and
    /// parse the response into a [`Product3DSpec`].
    pub async fn analyze_image_for_3d(
        &self,
        image_bytes: Vec<u8>,
        mime_type: &str,
    ) -> Result<Product3DSpec, AppError> {
        if self.api_key.is_empty() {
            return Err(AppError::internal(
                "GEMINI_API_KEY is not configured — Laboratory v2 generate-model is unavailable",
            ));
        }
        if image_bytes.is_empty() {
            return Err(AppError::validation("vision_3d: empty image bytes"));
        }

        let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

        let body = GeminiRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text {
                        text: VISION_PROMPT.to_string(),
                    },
                    Part::InlineData {
                        inline_data: InlineData {
                            mime_type: mime_type.to_string(),
                            data: b64,
                        },
                    },
                ],
            }],
            generation_config: GenerationConfig {
                temperature: 0.0,
                response_mime_type: "application/json".to_string(),
                max_output_tokens: 1024,
            },
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        tracing::info!(
            "🔍 Gemini Vision 3D: analyzing {} bytes ({})",
            image_bytes.len(),
            mime_type
        );

        let res = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("vision_3d: request failed: {e}")))?;

        let status = res.status();
        let raw = res
            .text()
            .await
            .map_err(|e| AppError::internal(format!("vision_3d: read body: {e}")))?;

        if !status.is_success() {
            let preview = &raw[..raw.len().min(400)];
            tracing::error!("❌ vision_3d: HTTP {} body={}", status, preview);
            return Err(AppError::internal(format!(
                "vision_3d: Gemini API error {}: {}",
                status, preview
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
            AppError::internal(format!("vision_3d: outer JSON parse: {e} — raw: {raw}"))
        })?;

        let content_text = json
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::internal(format!(
                    "vision_3d: no text in candidates[0].content.parts[0] — raw: {raw}"
                ))
            })?;

        let cleaned = strip_markdown_fences(content_text);

        let spec: Product3DSpec = serde_json::from_str(&cleaned).map_err(|e| {
            tracing::error!(
                "❌ vision_3d: spec JSON parse failed: {e}\nraw_text:\n{}",
                content_text
            );
            AppError::internal(format!(
                "vision_3d: spec JSON parse: {e} — payload: {cleaned}"
            ))
        })?;

        tracing::info!(
            "✅ vision_3d: object_type={} confidence={:.2}",
            spec.object_type.as_str(),
            spec.confidence
        );

        Ok(spec)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Strip ```json …``` fences that thinking-style Gemini models occasionally
/// add even when `responseMimeType: application/json` is requested.
fn strip_markdown_fences(text: &str) -> String {
    let trimmed = text.trim();
    let without_prefix = if let Some(rest) = trimmed.strip_prefix("```json") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest
    } else {
        return trimmed.to_string();
    };
    let without_suffix = without_prefix
        .trim_end()
        .strip_suffix("```")
        .unwrap_or(without_prefix);
    without_suffix.trim().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Request DTOs (kept private — they only model the wire format)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inline_data")]
        inline_data: InlineData,
    },
}

#[derive(Serialize)]
struct InlineData {
    #[serde(rename = "mime_type")]
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}
