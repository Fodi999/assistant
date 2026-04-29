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

/// Token usage reported by Gemini for one Vision call.
/// Useful for cost dashboards and per-asset accounting.
#[derive(Debug, Clone, Copy, Default)]
pub struct VisionUsage {
    pub prompt_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// Result of a Vision call — the parsed spec plus token usage.
#[derive(Debug, Clone)]
pub struct VisionResult {
    pub spec: Product3DSpec,
    pub usage: VisionUsage,
}

/// Token budget for JSON output.
/// gemini-2.5-flash uses internal "thinking" tokens that count against
/// `maxOutputTokens`. With the default 2048 limit the thinking phase (~1 800
/// tokens) left only ~250 tokens for the actual JSON → truncated output →
/// parse failure (EOF while parsing a string).
/// We raise the limit to 8 192 and simultaneously suppress thinking entirely
/// (`thinkingBudget: 0`) because this is a deterministic structured-output
/// task — no chain-of-thought needed. That keeps latency and cost down while
/// guaranteeing the full JSON fits in the window.
const MAX_OUTPUT_TOKENS: u32 = 8_192;
const THINKING_BUDGET: u32 = 0; // disable CoT for structured-output

/// Gemini multimodal model that supports image input.
/// gemini-2.5-flash is the stable replacement for 2.0-flash-exp:
/// supports vision input, structured JSON output, recommended for production.
const VISION_MODEL: &str = "gemini-2.5-flash";
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
    "kind": "ceramic_bowl" | "glass_bowl" | "glass_bottle" | "glass_jar" | "white_plate" | "...",
    "material": "glass" | "ceramic" | "plastic" | "metal" | "unknown",
    "color_hex": "#RRGGBB",           // optional — opaque body colour (ceramic/plastic/metal)
    "tint_hex": "#RRGGBB",            // optional — glass tint colour (when material=glass)
    "transparency": 0.0..1.0,         // optional — glass transparency (0=opaque, 1=clear)
    "rim_darkness": 0.0..1.0,         // optional — how much darker the rim appears vs body
    "diameter_mm": number,            // optional, rough estimate
    "height_mm": number               // optional, rough estimate
  },
  "product": {
    "color_hex": "#RRGGBB",           // dominant colour of the product itself
    "viscosity": 0.0..1.0,            // optional, only for liquids/sauces (0=water, 1=paste)
    "gloss": 0.0..1.0,                // optional (0=matte, 1=mirror)
    "description": "short phrase",    // optional, e.g. "thick red sauce with herb specks"
    "surface": {                      // optional — fill for sauce_in_bowl and plate_food
      "pattern": "flat" | "swirl" | "spiral_swirl" | "mound" | "waves" | "chunky" | "unknown",
      "swirl_arms": 1..8,             // integer — number of visible swirl arms
      "ridge_height": 0.0..1.0,       // height of ridges (0=flat, 1=very tall)
      "groove_depth": 0.0..1.0,       // depth of grooves between ridges
      "center_peak": 0.0..1.0,        // height of centre peak (0=flat, 1=prominent dome)
      "fill_radius_ratio": 0.0..1.0,  // fraction of container radius the product fills (typical 0.88–0.97)
      "rim_gap_ratio": 0.0..0.20,     // gap between product edge and rim as fraction of radius
      "surface_irregularity": 0.0..1.0, // organic noise (0=perfect, 1=very rough/chunky)
      "highlight_strength": 0.0..1.0, // specular highlight in centre (0=none, 1=bright)
      "view_angle": "top_down" | "three_quarter" | "side" | "unknown",
      "fill_height_ratio": 0.0..1.0,  // how full the container is vertically (0=empty, 1=brim)
      "surface_thickness": 0.0..1.0,  // visible depth/thickness of the sauce layer (0=thin, 1=thick)
      "meniscus_height": 0.0..1.0,    // raised edge at the container wall (0=flat, 1=strong curve)
      "spiral_turns": 0.0..1.0,       // radial turns of spiral centre→rim (0=loose, 1=tight/dense)
      "frequency": 0.0..1.0,          // detail frequency multiplier (0=coarse few ridges, 1=many fine waves)
      "edge_softness": 0.0..1.0       // how early/softly displacement fades near the rim (0=abrupt, 1=early soft fade)
    }
  },
  "scene": {                          // optional
    "background": "...",
    "lighting": "..."
  },
  "shape_recipe": {                   // PR #28 — build plan for the geometry engine
    "layers": [                       // ordered bottom→top: container first, surface last
      {
        "layer": "container" | "fill" | "surface" | "label" | "cap" | "lighting_hint",
        "kind": "bowl" | "bottle" | "jar" | "plate" | "flat_card" | "sauce" | "swirl" | "mound" | "waves",
        "material_class": "glass" | "ceramic" | "plastic" | "metal" | "food" | "liquid" | "label",
        "color_hex": "#RRGGBB",       // dominant colour for this layer
        "roughness": 0.0..1.0,        // PBR roughness (0=mirror, 1=very matte)
        "metalness": 0.0..1.0,        // PBR metalness (0=dielectric, food/ceramic=0)
        "opacity": 0.0..1.0,          // 1=solid, <1=translucent/glass
        "notes": "..."                // optional hint for the generator
      }
    ],
    "lighting_hint": "soft overhead" | "HDRI studio" | "rim light" | "...",
    "hero_angle": "top_down" | "three_quarter" | "side" | "hero_45"
  }
}

For sauce_in_bowl: do not only classify the product — estimate the visible surface geometry.
If the sauce has a spiral swirl, set pattern="spiral_swirl" and estimate swirl_arms, ridge_height,
groove_depth, center_peak, fill_radius_ratio and rim_gap_ratio from what is visible in the image.
If the surface is flat or featureless, set pattern="flat" and leave other surface fields at 0.

Always estimate fill volume for sauce_in_bowl and jar_product:
- fill_height_ratio: how full the container appears (0=empty, 1=filled to brim). Typical 0.40–0.85.
- surface_thickness: how thick/deep the sauce layer looks from side view (0=thin film, 1=thick paste). Typical 0.15–0.65.
- meniscus_height: how strongly the sauce curves up at the container wall (0=flat, 1=water-glass curve). Typical 0.05–0.50.
If only a top-down view is visible, estimate fill_height_ratio=0.72, surface_thickness=0.45, meniscus_height=0.20 as safe defaults.

Always estimate spiral geometry detail for sauce_in_bowl and plate_food:
- spiral_turns: how tightly wound the swirl looks — loose open spiral → 0.25, tight multi-turn spiral → 0.75. Default 0.45.
- frequency: how fine the wave detail is — broad smooth waves → 0.25, many tight ridges → 0.70. Default 0.45.
- edge_softness: how naturally the sauce fades at the bowl rim — hard cutoff → 0.20, smooth natural fade → 0.70. Default 0.50.

For plate_food: estimate the food mound geometry on the plate.
- pattern: "smooth_mound" for purée/cream/hummus, "chunky" for salad/grains/roasted veg/potatoes,
  "swirl" for mashed-potato-swirl or sauce-drizzle, "flat" for flatbread/pizza/galette.
- center_peak: height of the apex (0=flat, 1=tall dome). Typical purée=0.40–0.65, salad=0.55–0.80.
- surface_irregularity: how textured/lumpy the surface looks (purée≈0.15, salad≈0.75, mash≈0.40).
- fill_radius_ratio: how much of the plate's edible area the food covers (typical 0.75–0.95).
- highlight_strength: sheen/gloss visible on the food surface (sauce on top → 0.60–0.85, dry food → 0.05–0.20).
Always set container.material="ceramic" for the plate unless visibly glass or other material.

For container material: always set container.material explicitly.
If the container is transparent or semi-transparent glass and appears brown/red/amber because the
product is visible through it, set material="glass", tint_hex to the dominant glass wall tint
(e.g. "#3D1A0A" for dark brown), transparency to 0.55–0.85, and rim_darkness to how much the rim
appears darker than the body. Do not omit tint_hex or color_hex if the rim or wall colour is visible.

For shape_recipe: always populate it. List every visible layer bottom→top.
A ceramic bowl with red sauce would have two layers:
  [{"layer":"container","kind":"bowl","material_class":"ceramic","roughness":0.55,...},
   {"layer":"fill","kind":"sauce","material_class":"liquid","roughness":0.10,...}]
For liquids: roughness 0.05–0.15, metalness 0, opacity 1.0.
For ceramics: roughness 0.45–0.65, metalness 0, opacity 1.0.
For glass: roughness 0.04–0.10, metalness 0, opacity 0.25–0.55.
Always set hero_angle to the camera angle that best shows the product.

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
        self.analyze_image_for_3d_with_usage(image_bytes, mime_type)
            .await
            .map(|r| r.spec)
    }

    /// Same as [`analyze_image_for_3d`] but also returns Gemini-reported
    /// token usage (`promptTokenCount` / `candidatesTokenCount` / total).
    /// Use this in callers that want to persist or display per-asset cost.
    pub async fn analyze_image_for_3d_with_usage(
        &self,
        image_bytes: Vec<u8>,
        mime_type: &str,
    ) -> Result<VisionResult, AppError> {
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
                max_output_tokens: MAX_OUTPUT_TOKENS,
                thinking_config: ThinkingConfig {
                    thinking_budget: THINKING_BUDGET,
                },
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

        // ── Token usage (Gemini returns this in every response) ──
        // We log it so callers can see exactly how expensive each 3D
        // generation is. Image input dominates promptTokenCount.
        let prompt_tokens = json
            .pointer("/usageMetadata/promptTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let output_tokens = json
            .pointer("/usageMetadata/candidatesTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total_tokens = json
            .pointer("/usageMetadata/totalTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(prompt_tokens + output_tokens);
        tracing::info!(
            "💰 vision_3d tokens: prompt={} output={} total={} (model={})",
            prompt_tokens,
            output_tokens,
            total_tokens,
            self.model,
        );
        let usage = VisionUsage {
            prompt_tokens,
            output_tokens,
            total_tokens,
        };

        let cleaned = strip_markdown_fences(content_text);

        let spec: Product3DSpec = serde_json::from_str(&cleaned).map_err(|e| {
            tracing::error!(
                "❌ vision_3d: spec JSON parse failed: {e}\nraw_text:\n{}",
                content_text
            );
            // Limit payload in error to avoid enormous log/response lines.
            let preview = &cleaned[..cleaned.len().min(800)];
            AppError::internal(format!(
                "vision_3d: spec JSON parse: {e} — payload: {preview}"
            ))
        })?;

        tracing::info!(
            "✅ vision_3d: object_type={} confidence={:.2}\n📋 raw_gemini_response:\n{}",
            spec.object_type.as_str(),
            spec.confidence,
            cleaned,
        );

        Ok(VisionResult { spec, usage })
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
    #[serde(rename = "thinkingConfig")]
    thinking_config: ThinkingConfig,
}

/// Suppress chain-of-thought for structured-output calls.
/// `thinkingBudget: 0` tells Gemini 2.5 Flash not to spend any tokens on
/// internal reasoning — the task is deterministic JSON extraction, not
/// open-ended reasoning, so thinking only wastes the output window.
#[derive(Serialize)]
struct ThinkingConfig {
    #[serde(rename = "thinkingBudget")]
    thinking_budget: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::laboratory_v2::{Product3DObjectType, Product3DSpec};

    const CLEAN_JSON: &str = r##"{
        "object_type": "sauce_in_bowl",
        "confidence": 0.92,
        "container": {
            "kind": "ceramic_bowl",
            "color_hex": "#F2EFE7",
            "diameter_mm": 120.0,
            "height_mm": 55.0
        },
        "product": {
            "color_hex": "#B8321F",
            "viscosity": 0.6,
            "gloss": 0.3,
            "description": "thick red tomato sauce",
            "surface": {
                "pattern": "spiral_swirl",
                "swirl_arms": 4,
                "ridge_height": 0.8,
                "groove_depth": 0.7,
                "center_peak": 0.6,
                "fill_radius_ratio": 0.96,
                "rim_gap_ratio": 0.04,
                "surface_irregularity": 0.3,
                "highlight_strength": 0.9,
                "view_angle": "top_down"
            }
        },
        "scene": {
            "background": "white",
            "lighting": "soft overhead"
        }
    }"##;

    #[test]
    fn parse_clean_json_into_product3d_spec() {
        let spec: Product3DSpec =
            serde_json::from_str(CLEAN_JSON).expect("should parse clean JSON");
        assert_eq!(spec.object_type, Product3DObjectType::SauceInBowl);
        assert!((spec.confidence - 0.92).abs() < 1e-4);
        let container = spec.container.as_ref().expect("container should be present");
        assert_eq!(container.kind, "ceramic_bowl");
        assert_eq!(container.diameter_mm, Some(120.0));
        assert_eq!(spec.product.color_hex, "#B8321F");
        assert_eq!(spec.effective_object_type(), Product3DObjectType::SauceInBowl);
        let surface = spec.product.surface.as_ref().expect("surface should be present");
        assert_eq!(surface.pattern.as_deref(), Some("spiral_swirl"));
        assert_eq!(surface.swirl_arms, Some(4));
        assert!((surface.ridge_height.unwrap() - 0.8).abs() < 1e-4);
        assert_eq!(surface.view_angle.as_deref(), Some("top_down"));
    }

    #[test]
    fn parse_fenced_json_via_strip_markdown_fences() {
        let fenced = format!("```json\n{}\n```", CLEAN_JSON);
        let cleaned = strip_markdown_fences(&fenced);
        let spec: Product3DSpec =
            serde_json::from_str(&cleaned).expect("should parse after stripping fences");
        assert_eq!(spec.object_type, Product3DObjectType::SauceInBowl);
    }

    #[test]
    fn low_confidence_falls_back_to_flat_card() {
        let mut spec: Product3DSpec = serde_json::from_str(CLEAN_JSON).unwrap();
        spec.confidence = 0.4; // below MIN_CONFIDENCE
        assert_eq!(spec.effective_object_type(), Product3DObjectType::FlatCard);
    }

    #[test]
    fn unknown_type_falls_back_to_flat_card() {
        let json = r##"{
            "object_type": "unknown",
            "confidence": 0.8,
            "product": { "color_hex": "#AAAAAA" }
        }"##;
        let spec: Product3DSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.effective_object_type(), Product3DObjectType::FlatCard);
    }

    #[test]
    fn strip_plain_backtick_fences() {
        let fenced = format!("```\n{}\n```", CLEAN_JSON);
        let cleaned = strip_markdown_fences(&fenced);
        assert!(serde_json::from_str::<Product3DSpec>(&cleaned).is_ok());
    }
}
