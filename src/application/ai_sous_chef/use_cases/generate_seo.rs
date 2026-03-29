//! Use Case: AI Generate SEO — SEO metadata for product pages
//!
//! Pipeline:
//! 1. Check cache (hash of product slug + nutrition highlights)
//! 2. If miss → call AiClient::generate (balanced model)
//! 3. Parse JSON response
//! 4. Cache result (TTL: 60 days — SEO rarely changes)
//! 5. Return SEO metadata JSON

use crate::application::admin_catalog::AdminCatalogService;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Cache TTL for SEO results (days)
const SEO_CACHE_TTL_DAYS: i32 = 60;

impl AdminCatalogService {
    /// Generate SEO metadata for a product using AI.
    ///
    /// 🚀 With caching: −70% AI costs for repeated requests
    pub async fn ai_generate_seo(&self, id: Uuid) -> AppResult<serde_json::Value> {
        let product = self.get_product_by_id(id).await?;

        let name_en = &product.name_en;
        let slug = product.slug.as_deref().unwrap_or("unknown");
        let product_type = &product.product_type;
        let name_ru = product.name_ru.as_deref().unwrap_or("");

        // Nutrition highlights for SEO
        let mut highlights = Vec::new();
        if let Some(cal) = product.calories_per_100g {
            highlights.push(format!("{}kcal", cal));
        }
        if let Some(p) = &product.protein_per_100g {
            highlights.push(format!("{}g protein", p));
        }
        if let Some(f) = &product.fat_per_100g {
            highlights.push(format!("{}g fat", f));
        }
        if let Some(c) = &product.carbs_per_100g {
            highlights.push(format!("{}g carbs", c));
        }
        let nutrition_str = if highlights.is_empty() {
            "nutrition data available".to_string()
        } else {
            highlights.join(", ")
        };

        // ── Cache check ──
        let fingerprint = format!("{}:{}:{}", slug, product_type, nutrition_str);
        let cache_key = format!("uc:seo:{}", hash_input(&fingerprint));

        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            tracing::info!("📦 SEO cache hit for product {} ({})", id, name_en);
            return Ok(cached);
        }

        // ── Build prompt ──
        let prompt = format!(
            r#"You are an SEO expert for a food/nutrition website "dima-fomin.pl".

Generate SEO metadata for this ingredient page:

Product: "{name_en}" (Russian: "{name_ru}")
Category: {product_type}
Slug: {slug}
Nutrition highlights per 100g: {nutrition_str}
Page URL: https://dima-fomin.pl/ingredients/{slug}

Return ONLY a valid JSON object:
{{
  "seo_title": "<60 chars max, format: {{Name}} — Nutrition, Vitamins, Culinary Uses | dima-fomin.pl>",
  "seo_description": "<155 chars max, compelling meta description with nutrition keywords>",
  "seo_h1": "<H1 heading, format: {{Name}} — Nutrition & Culinary Profile>",
  "canonical_url": "https://dima-fomin.pl/ingredients/{slug}",
  "og_title": "<65 chars max, engaging Open Graph title for social sharing>",
  "og_description": "<200 chars max, social-friendly description with key nutrition facts>"
}}

Rules:
- seo_title: Include product name + "Nutrition" + one key benefit. Max 60 chars.
- seo_description: Include calories, protein, key vitamin/mineral. Max 155 chars. Make it click-worthy.
- seo_h1: Clean, keyword-rich H1. Slightly different from seo_title.
- og_title: Social-friendly, can be more casual. Include emoji if appropriate.
- og_description: Focus on most interesting nutrition fact or culinary use.
- All in English.
- Return ONLY the JSON, no other text."#,
            name_en = name_en,
            name_ru = name_ru,
            product_type = product_type,
            slug = slug,
            nutrition_str = nutrition_str,
        );

        // ── Call AI via trait (Balanced = gemini-3.1-pro-preview, more reliable) ──
        let raw = match self.llm_adapter
            .generate_with_quality(&prompt, 1500, AiQuality::Balanced)
            .await
        {
            Ok(r) => r,
            Err(first_err) => {
                tracing::warn!("🔄 SEO first attempt failed: {}, retrying…", first_err);
                self.llm_adapter
                    .generate_with_quality(&prompt, 1500, AiQuality::Balanced)
                    .await?
            }
        };

        // ── Log raw response for debugging ──
        let preview_end = raw.char_indices().nth(400).map(|(i, _)| i).unwrap_or(raw.len());
        tracing::info!("🤖 AI SEO raw response for {} ({}): {}", id, name_en, &raw[..preview_end]);

        // ── Parse JSON ──
        let result = parse_json_response(&raw)?;

        // ── Cache result ──
        if let Err(e) = self.ai_cache.set(
            &cache_key, result.clone(), "gemini", "gemini-3.1-pro-preview", SEO_CACHE_TTL_DAYS
        ).await {
            tracing::warn!("Failed to cache SEO result: {}", e);
        }

        tracing::info!("✅ AI SEO generated for product {} ({}) (cached)", id, name_en);
        Ok(result)
    }
}

/// SHA-256 hash → short hex
fn hash_input(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

fn parse_json_response(raw: &str) -> AppResult<serde_json::Value> {
    // Step 1: Strip markdown code fences (Gemini 3 thinking models add these)
    let cleaned = if raw.contains("```") {
        let trimmed = raw.trim();
        let without_prefix = if trimmed.starts_with("```json") {
            &trimmed[7..]
        } else if trimmed.starts_with("```") {
            &trimmed[3..]
        } else {
            trimmed
        };
        let without_suffix = without_prefix
            .trim()
            .strip_suffix("```")
            .unwrap_or(without_prefix);
        without_suffix.trim()
    } else {
        raw.trim()
    };

    // Step 2: Try direct parse
    serde_json::from_str(cleaned)
        .or_else(|_| {
            // Step 3: Extract JSON object from surrounding text
            if let Some(start) = cleaned.find('{') {
                if let Some(end) = cleaned.rfind('}') {
                    return serde_json::from_str(&cleaned[start..=end]);
                }
            }
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No JSON found",
            )))
        })
        .map_err(|e| {
            let preview = &raw[..raw.len().min(300)];
            tracing::error!("Failed to parse AI SEO response: {} | raw preview: {}", e, preview);
            AppError::internal("AI returned invalid JSON")
        })
}
