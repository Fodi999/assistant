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

    pub async fn generate_material_scene_image(
        &self,
        material_title: &str,
        material_text: &str,
        scene: &str,
    ) -> Result<String, AppError> {
        let prompt = format!(
            r#"Create a photorealistic commercial construction-material image for ALMABUILD/KAZAXBUD in Almaty.

Material: {material_title}
Context: {material_text}
Scene direction: {scene}

Visual requirements:
- real construction/retail environment, not an isolated white-background packshot
- show how the material is used, stored, installed, cut, stacked, mounted or prepared
- suitable for a building materials website for shop renovation and commercial fit-out
- modern warehouse/store or active construction interior, clean professional lighting
- no readable text, no logos, no watermarks, no UI, no duplicated impossible objects
- make this image visibly different from other material cards
- landscape composition, sharp, realistic, high detail"#,
            material_title = material_title,
            material_text = material_text,
            scene = scene,
        );

        self.generate_image_from_prompt(
            &prompt,
            material_title,
            "construction material scene",
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
        if scene_preset == "orthodox-icon-restoration" {
            let reference_contract = if reference_urls.is_empty() {
                r#"REFERENCE STATUS:
- No visual reference was supplied. Create a traditional Orthodox icon based only on the subject and instruction."#
            } else {
                r#"REFERENCE ICON CONTRACT — HIGHEST PRIORITY:
- Treat the uploaded reference image as the exact iconographic source, not as loose inspiration.
- Preserve the same saint or feast, figures, gestures, composition, halos, garments, riza, colors, border proportions and sacred icon style.
- Improve clarity, crop, light balance, color depth and fine detail only.
- Do not create a new church interior, candle scene, realistic photograph, modern portrait or unrelated icon.
- Do not add readable inscriptions, logos, watermarks or UI."#
            };
            let prompt = format!(
                r#"Generate ONE IMAGE ONLY. Do not write JSON, markdown, captions, explanations or article text.

Create a faithful Orthodox icon image for "{article_title}".
Admin instruction: {scene}
Scale and style: {scale_direction}

{reference_contract}

STYLE STANDARD:
- traditional Orthodox iconography, flat sacred perspective, egg tempera and gold leaf feel
- reverent museum-quality restoration, high detail, clear composition
- cream/gold/deep natural pigments, calm liturgical mood
- image suitable for a Orthodox icon catalog page

STRICTLY EXCLUDE:
- photorealistic people, church interior photo as main subject, random candles, stock-photo look
- readable text, captions, logos, watermarks, UI
- distorted faces, extra hands, invented figures, changed subject"#,
                article_title = article_title,
                scene = scene,
                scale_direction = scale_direction,
                reference_contract = reference_contract,
            );
            let model = if enhanced {
                &self.recipe_hero_image_model
            } else {
                &self.recipe_image_model
            };
            return self
                .generate_image_from_prompt_with_references(
                    &prompt,
                    article_title,
                    "orthodox icon",
                    model,
                    reference_urls,
                )
                .await;
        }

        if scene_preset == "orthodox-icon-product-mockup" {
            let reference_contract = if reference_urls.is_empty() {
                r#"REFERENCE STATUS:
- No visual reference was supplied. Create an interactive Orthodox prayer icon product mockup based on the subject and instruction."#
            } else {
                r#"REFERENCE PRODUCT MOCKUP CONTRACT — HIGHEST PRIORITY:
- Reference 1 is the original sacred icon artwork. It must become the visible icon inside the product frame/mockup.
- If other references show a product mockup, wood frame, QR module, phone/audio interface, stand, lighting, or camera angle, use those only as product-format references.
- If a product/mockup reference contains a different sacred artwork, replace that artwork with Reference 1.
- Preserve Reference 1's saint or feast, figures, gestures, composition, halos, clothing colors, border proportions and sacred icon style.
- Do not replace Reference 1 with a generic Mother of God, another saint, church interior, candle photo, or realistic people."#
            };
            let prompt = format!(
                r#"Generate ONE IMAGE ONLY. Do not write JSON, markdown, captions, explanations or article text.

Create a premium Orthodox interactive prayer icon product mockup for "{article_title}".
Admin instruction: {scene}
Scale and style: {scale_direction}

{reference_contract}

PRODUCT DETAILS TO INCLUDE WHEN APPROPRIATE:
- carved or wooden standing icon frame
- warm edge light or soft product lighting
- QR module or QR plate near the icon
- optional phone/audio prayer presentation if it is requested by the admin instruction
- clean catalog composition, realistic object proportions, high detail, 4K-quality look

STRICTLY EXCLUDE:
- readable new inscriptions, captions, logos, watermarks or UI
- unrelated church interiors, random candles, photorealistic live people
- changed sacred subject, invented figures, distorted faces or hands"#,
                article_title = article_title,
                scene = scene,
                scale_direction = scale_direction,
                reference_contract = reference_contract,
            );
            return self
                .generate_image_from_prompt_with_references(
                    &prompt,
                    article_title,
                    "orthodox icon product mockup",
                    &self.recipe_image_model,
                    reference_urls,
                )
                .await;
        }

        let commerce_product_mode = scene_preset == "delivery-product"
            || (!reference_urls.is_empty() && scene_preset != "cooking-process");
        let role = match (commerce_product_mode, variant) {
            (true, 0) => "primary delivery-product hero image showing the complete referenced product",
            (true, 1) => "alternate clean background treatment showing the same complete referenced product",
            (true, 2) => "safe closer crop of the same referenced product without changing its perspective or arrangement",
            (true, 3) => "alternate delivery catalog background showing the same complete referenced product",
            (true, _) => "another clean background treatment of the same complete referenced product",
            (false, 0) => "wide editorial hero cover",
            (false, 1) => "professional step-by-step process scene",
            (false, 2) => "tight macro detail",
            (false, 3) => "finished result in an elegant editorial composition",
            (false, _) => "chronological recipe or technique step in a visual editorial series",
        };
        let scene_style = match scene_preset {
            "product-white" => "Ecommerce product packshot: isolate the subject on a pure seamless white #FFFFFF background, centered, fully visible, soft contact shadow, no props or table.",
            "delivery-product" => "Food-delivery catalog hero: the complete product is the sole subject, placed on a clean neutral delivery-ready surface or inside minimal unbranded delivery packaging. No people, hands, preparation, extra dishes or decorative food.",
            "recipe-table" => "Finished recipe on a tasteful natural table setting, warm restaurant-quality daylight, restrained relevant tableware, appetizing and realistic.",
            "home-interior" => "Lifestyle scene in a refined modern home kitchen or dining interior, natural window light, believable domestic atmosphere, subject remains dominant.",
            "cooking-process" => "Instructional cooking-process photograph showing one clear chronological action, clean workstation, hands only when needed, technique easy to understand.",
            "restaurant-plating" => "Premium restaurant plating on elegant tableware, controlled fine-dining light, precise composition, minimal sophisticated background.",
            "object-interior" => "Editorial object photograph in a modern home interior, realistic scale and materials, soft daylight, curated but uncluttered surroundings.",
            _ => "Premium culinary editorial scene with a clear subject, modern composition and realistic context.",
        };
        let reference_contract = if commerce_product_mode && !reference_urls.is_empty() {
            r#"REFERENCE PRODUCT CONTRACT — HIGHEST PRIORITY:
- Treat the uploaded reference image as the exact product being photographed, not as inspiration and not as a general topic.
- Preserve the exact food assortment, item count, portions, colors, toppings, arrangement, silhouette and relative positions from the reference.
- Visually cut out/copy the referenced product and place that same product into the requested background and lighting.
- The complete product must remain fully visible, unobstructed and dominant, occupying approximately 65–80% of the frame.
- Keep the original product viewpoint and perspective. Change only safe crop, background, surface, lighting and subtle contact shadow.
- Do not invent hidden or unseen sides of the product.
- Do not redesign, cook, plate, rearrange, replace, add or remove any part of the referenced product.
- Do not infer a preparation process from the article title or scene direction.
- No humans, chefs, faces, bodies, hands, arms or human interaction.
- No additional food, ingredients, dishes, utensils, bowls, cups, plants, packaging or props unless explicitly requested as a scale reference.
- If article instructions conflict with this contract, preserve the referenced product and ignore the conflicting instruction."#
        } else if commerce_product_mode {
            r#"COMMERCIAL PRODUCT CONTRACT — HIGHEST PRIORITY:
- Show one exact sellable product described in the request as the sole dominant subject.
- The complete product must remain fully visible and unobstructed, occupying approximately 65–80% of the frame.
- Create a clean ecommerce or food-delivery catalog image, not an editorial story or preparation process.
- No humans, chefs, faces, bodies, hands, arms or human interaction.
- No additional food, ingredients, dishes, utensils, bowls, cups, plants or decorative props.
- Do not invent package contents, accessories, variants or additional products."#
        } else {
            r#"REFERENCE GUIDANCE:
- Use uploaded references to preserve recognizable subject details and visual identity.
- Humans or hands are allowed only when the selected cooking-process scene explicitly requires them."#
        };
        let prompt = format!(
            r#"Create a premium culinary magazine photograph for the article "{article_title}".
Image role: {role}.
Scene direction: {scene}.
Scene preset: {scene_style}
Scale and realism constraints: {scale_direction}

{reference_contract}

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
- unrelated ingredients or decorative elements that do not support the topic
- when the reference product contract is active: people, hands, chefs, preparation actions, added food, changed assortment, changed quantity or obstructed product"#,
            article_title = article_title,
            role = role,
            scene = scene,
            scene_style = scene_style,
            scale_direction = scale_direction,
            reference_contract = reference_contract,
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

    /// Generate a construction project / commercial fit-out image with optional visual references.
    pub async fn generate_construction_project_image(
        &self,
        project_title: &str,
        description: &str,
        scene: &str,
        variant: usize,
        enhanced: bool,
        reference_urls: &[String],
    ) -> Result<String, AppError> {
        let role = match variant {
            0 => "hero image: wide finished commercial interior based on the references",
            1 => "alternate angle: entrance, facade or main retail zone based on the references",
            2 => "detail angle: ceiling, lighting, shelves, wall finish and material quality",
            3 => "wide gallery angle: complete shop fit-out, clean architectural composition",
            _ => "additional gallery angle: realistic commercial renovation detail",
        };
        let reference_contract = if reference_urls.is_empty() {
            r#"REFERENCE STATUS:
- No visual reference was supplied. Build a realistic commercial fit-out image from the project description only."#
        } else {
            r#"REFERENCE CONTRACT — HIGHEST PRIORITY:
- Use the uploaded reference photos as the exact design brief for the commercial space.
- Preserve the recognizable layout, storefront/interior geometry, shelving rhythm, ceiling lighting, color palette, material finishes and premium retail atmosphere from the references.
- Generate a polished finished-project photo, not a different imaginary store.
- You may improve sharpness, lighting, cleanliness and professional architectural composition.
- Do not add people, workers, clutter, construction mess, random products, random signage or extra brands.
- Do not invent a new logo or readable text. If signage is visible in the reference, preserve only its placement, glow and general visual identity without creating new text artifacts.
- Each variant should be a different believable camera angle of the same place."#
        };
        let prompt = format!(
            r#"Create a photorealistic architectural case-study image for ALMABUILD / KAZAXBUD.
Project: {project_title}
Project facts: {description}
Image role: {role}
Admin scene direction: {scene}

{reference_contract}

STYLE STANDARD:
- premium commercial retail interior / mall store fit-out photography
- realistic architectural lighting, clean materials, precise perspective and straight verticals
- show finished renovation quality: ceiling, lighting, floor, wall finishes, shelves, counter, facade or entrance when relevant
- landscape composition suitable for a construction portfolio SEO page
- high detail, natural lens, no exaggerated CGI look

STRICTLY EXCLUDE:
- people, faces, bodies, workers, safety helmets
- watermarks, UI, captions, distorted readable text, random brand logos
- impossible lighting, duplicated shelves, warped architecture, clutter, dirty construction stage"#,
            project_title = project_title,
            description = description,
            role = role,
            scene = scene,
            reference_contract = reference_contract,
        );
        let model = if enhanced {
            &self.recipe_hero_image_model
        } else {
            &self.recipe_image_model
        };
        self.generate_image_from_prompt_with_references(
            &prompt,
            project_title,
            "construction project",
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
        for reference_url in reference_urls.iter().take(4) {
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

    pub async fn analyze_image_json(
        &self,
        prompt: &str,
        image_bytes: &[u8],
        mime_type: &str,
    ) -> Result<String, AppError> {
        self.analyze_images_json(prompt, &[(image_bytes, mime_type)])
            .await
    }

    pub async fn analyze_images_json(
        &self,
        prompt: &str,
        images: &[(&[u8], &str)],
    ) -> Result<String, AppError> {
        if images.is_empty() {
            return Err(AppError::validation("Image file is empty"));
        }
        let mut parts = vec![serde_json::json!({"text": prompt})];
        for (image_bytes, mime_type) in images.iter().take(2) {
            if image_bytes.is_empty() {
                return Err(AppError::validation("Image file is empty"));
            }
            if image_bytes.len() > 10 * 1024 * 1024 {
                return Err(AppError::validation("Image must be smaller than 10 MB"));
            }
            parts.push(serde_json::json!({
                "inlineData": {
                    "mimeType": mime_type,
                    "data": base64::engine::general_purpose::STANDARD.encode(image_bytes)
                }
            }));
        }

        let body = serde_json::json!({
            "contents": [{"parts": parts}],
            "generationConfig": {
                "temperature": 0.2,
                "responseMimeType": "application/json"
            }
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.smart_model, self.api_key
        );

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            self.http_client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send(),
        )
        .await
        .map_err(|_| AppError::internal("Timeout: Gemini Vision took too long"))?
        .map_err(|e| AppError::internal(format!("Gemini Vision request failed: {e}")))?;

        let status = result.status();
        if !status.is_success() {
            let err = result.text().await.unwrap_or_default();
            tracing::error!(
                "Gemini Vision failed (HTTP {}): {}",
                status,
                &err[..err.len().min(300)]
            );
            return Err(AppError::internal(format!(
                "Gemini Vision API error: {status}"
            )));
        }

        let json: serde_json::Value = result.json().await.map_err(|e| {
            AppError::internal(format!("Failed to parse Gemini Vision response: {e}"))
        })?;

        json.pointer("/candidates/0/content/parts")
            .and_then(|parts| parts.as_array())
            .and_then(|parts| {
                parts
                    .iter()
                    .find_map(|part| part.get("text").and_then(|text| text.as_str()))
            })
            .map(|text| text.to_string())
            .ok_or_else(|| {
                tracing::error!(
                    "No text JSON in Gemini Vision response: {:?}",
                    json.pointer("/candidates/0/content/parts")
                );
                AppError::internal("No text JSON in Gemini Vision response")
            })
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
