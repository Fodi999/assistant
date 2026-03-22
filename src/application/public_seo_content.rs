//! Public SEO Content Service — AI-generated content for programmatic SEO pages
//!
//! Pipeline:
//! 1. Validate input (intent_type, entity_a, locale)
//! 2. Check cache (SHA-256 of intent+entity_a+entity_b+locale)
//! 3. If miss → build prompt → call LLM (Fast model = cheap)
//! 4. Parse strict JSON response
//! 5. Cache result (TTL: 30 days — SEO content rarely changes)
//! 6. Return JSON to frontend

use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::infrastructure::persistence::AiCacheRepository;
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Cache TTL for SEO content (days)
const SEO_CONTENT_CACHE_TTL_DAYS: i32 = 30;

// ── Request / Response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SeoContentRequest {
    pub intent_type: String,
    pub entity_a: String,
    pub entity_b: Option<String>,
    pub locale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeoContentFaq {
    pub q: String,
    pub a: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeoContentResponse {
    pub title: String,
    pub description: String,
    pub answer: String,
    pub faq: Vec<SeoContentFaq>,
}

// ── Valid intent types ───────────────────────────────────────────────────────

const VALID_INTENTS: &[&str] = &["question", "comparison", "goal", "combo"];
const VALID_LOCALES: &[&str] = &["en", "pl", "ru", "uk"];

// ── Service ──────────────────────────────────────────────────────────────────

pub struct PublicSeoContentService {
    llm_adapter: Arc<LlmAdapter>,
    ai_cache: AiCacheRepository,
}

impl PublicSeoContentService {
    pub fn new(llm_adapter: Arc<LlmAdapter>, ai_cache: AiCacheRepository) -> Self {
        Self { llm_adapter, ai_cache }
    }

    /// Generate SEO content for a programmatic page.
    ///
    /// Cached aggressively — same input always returns same content.
    pub async fn generate(&self, req: &SeoContentRequest) -> AppResult<SeoContentResponse> {
        // ── 1. Validate ──
        let intent = req.intent_type.to_lowercase();
        if !VALID_INTENTS.contains(&intent.as_str()) {
            return Err(AppError::validation(&format!(
                "Invalid intent_type '{}'. Must be one of: {}",
                intent,
                VALID_INTENTS.join(", ")
            )));
        }

        let locale = req.locale.to_lowercase();
        if !VALID_LOCALES.contains(&locale.as_str()) {
            return Err(AppError::validation(&format!(
                "Invalid locale '{}'. Must be one of: {}",
                locale,
                VALID_LOCALES.join(", ")
            )));
        }

        let entity_a = req.entity_a.trim().to_lowercase();
        if entity_a.is_empty() || entity_a.len() > 100 {
            return Err(AppError::validation("entity_a must be 1-100 characters"));
        }

        let entity_b = req.entity_b
            .as_deref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        if intent == "comparison" && entity_b.is_none() {
            return Err(AppError::validation(
                "entity_b is required for intent_type 'comparison'"
            ));
        }

        // ── 2. Cache check ──
        let fingerprint = format!(
            "seo_content:{}:{}:{}:{}",
            intent,
            entity_a,
            entity_b.as_deref().unwrap_or(""),
            locale,
        );
        let cache_key = format!("uc:seo_content:{}", hash(&fingerprint));

        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            if let Ok(response) = serde_json::from_value::<SeoContentResponse>(cached) {
                tracing::info!("📦 SEO content cache hit: {} / {} / {}", intent, entity_a, locale);
                return Ok(response);
            }
        }

        // ── 3. Build prompt ──
        let prompt = build_prompt(&intent, &entity_a, entity_b.as_deref(), &locale);

        // ── 4. Call LLM (Fast model — cheap, ~$0.0003) ──
        let raw = self.llm_adapter
            .groq_raw_request_with_model(&prompt, 800, "llama-3.3-70b-versatile")
            .await?;

        // ── 5. Parse JSON ──
        let response = parse_response(&raw)?;

        // ── 6. Cache ──
        if let Ok(val) = serde_json::to_value(&response) {
            if let Err(e) = self.ai_cache.set(
                &cache_key,
                val,
                "groq",
                "llama-3.3-70b-versatile",
                SEO_CONTENT_CACHE_TTL_DAYS,
            ).await {
                tracing::warn!("Failed to cache SEO content: {}", e);
            }
        }

        tracing::info!(
            "✅ SEO content generated: {} / {} / {} (cached 30d)",
            intent, entity_a, locale
        );

        Ok(response)
    }

    /// Generate SEO content with a SPECIFIC search query (for targeted sub-intents).
    ///
    /// Instead of generic "question about salmon", this answers a specific query like
    /// "Is salmon good for weight loss?" — producing much better long-tail SEO content.
    pub async fn generate_with_query(&self, req: &SeoContentRequest, search_query: &str) -> AppResult<SeoContentResponse> {
        // ── 1. Validate (same as generate) ──
        let intent = req.intent_type.to_lowercase();
        if !VALID_INTENTS.contains(&intent.as_str()) {
            return Err(AppError::validation(&format!(
                "Invalid intent_type '{}'. Must be one of: {}",
                intent, VALID_INTENTS.join(", ")
            )));
        }

        let locale = req.locale.to_lowercase();
        if !VALID_LOCALES.contains(&locale.as_str()) {
            return Err(AppError::validation(&format!(
                "Invalid locale '{}'. Must be one of: {}",
                locale, VALID_LOCALES.join(", ")
            )));
        }

        let entity_a = req.entity_a.trim().to_lowercase();
        let entity_b = req.entity_b.as_deref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());

        // ── 2. Cache check (include search_query in fingerprint) ──
        let fingerprint = format!(
            "seo_content_q:{}:{}:{}:{}:{}",
            intent, entity_a, entity_b.as_deref().unwrap_or(""), locale, search_query,
        );
        let cache_key = format!("uc:seo_content:{}", hash(&fingerprint));

        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            if let Ok(response) = serde_json::from_value::<SeoContentResponse>(cached) {
                tracing::info!("📦 SEO content cache hit: '{}' / {}", search_query, locale);
                return Ok(response);
            }
        }

        // ── 3. Build targeted prompt ──
        let prompt = build_targeted_prompt(&intent, &entity_a, entity_b.as_deref(), &locale, search_query);

        // ── 4. Call LLM ──
        let raw = self.llm_adapter
            .groq_raw_request_with_model(&prompt, 800, "llama-3.3-70b-versatile")
            .await?;

        // ── 5. Parse ──
        let response = parse_response(&raw)?;

        // ── 6. Cache ──
        if let Ok(val) = serde_json::to_value(&response) {
            let _ = self.ai_cache.set(&cache_key, val, "groq", "llama-3.3-70b-versatile", SEO_CONTENT_CACHE_TTL_DAYS).await;
        }

        tracing::info!("✅ SEO content generated: '{}' / {} (cached 30d)", search_query, locale);
        Ok(response)
    }
}

// ── Prompt builder ───────────────────────────────────────────────────────────

fn build_prompt(intent: &str, entity_a: &str, entity_b: Option<&str>, locale: &str) -> String {
    let entity_b_line = match entity_b {
        Some(b) => format!("Secondary Ingredient: {}", b),
        None => "Secondary Ingredient: none".to_string(),
    };

    format!(
        r#"You are an AI culinary and nutrition assistant.

Your task is to generate SHORT, FACTUAL, and USEFUL content for a SEO page.

IMPORTANT RULES:
- DO NOT write long articles
- DO NOT use fluff or generic phrases
- DO NOT repeat the same sentences
- KEEP answers concise and data-driven
- WRITE like a professional chef + nutrition expert
- ALWAYS be helpful to a real user decision

---

INPUT:
Intent Type: {intent}
Main Ingredient: {entity_a}
{entity_b_line}
Language: {locale}

---

OUTPUT FORMAT (STRICT JSON):

{{
  "title": "...",
  "description": "...",
  "answer": "...",
  "faq": [
    {{
      "q": "...",
      "a": "..."
    }},
    {{
      "q": "...",
      "a": "..."
    }}
  ]
}}

---

CONTENT RULES:

1. TITLE:
- max 60 characters
- match search intent exactly

2. DESCRIPTION:
- max 140 characters
- clear and practical

3. ANSWER:
- 2–3 sentences maximum
- include:
  - nutrition insight
  - practical advice
  - comparison if needed

4. FAQ:
- 2 questions only
- real user questions
- short answers

---

INTENT LOGIC:

If intent_type = "question":
→ Answer directly (e.g. "Is salmon healthy?")
→ Explain why (protein, fat, vitamins)

If intent_type = "comparison":
→ Compare both ingredients
→ Highlight differences (protein, calories, fat)
→ Give recommendation

If intent_type = "goal":
→ Focus on goal (weight loss, protein)
→ Suggest best option

If intent_type = "combo":
→ Explain how ingredients work together
→ Balance (protein/fat/carbs)

---

LANGUAGE RULES:

- Write ONLY in {locale}
- Adapt to natural search phrases in that language
- DO NOT translate literally
- Use natural wording used by real users

---

TONE:

- expert but simple
- chef + nutritionist
- practical, not academic

Return ONLY the JSON, no other text."#,
        intent = intent,
        entity_a = entity_a,
        entity_b_line = entity_b_line,
        locale = locale,
    )
}

/// Build a TARGETED prompt for a specific user search query.
/// This produces much better long-tail SEO content than the generic prompt.
fn build_targeted_prompt(intent: &str, entity_a: &str, entity_b: Option<&str>, locale: &str, search_query: &str) -> String {
    let entity_b_line = match entity_b {
        Some(b) => format!("Secondary Ingredient: {}", b),
        None => "Secondary Ingredient: none".to_string(),
    };

    format!(
        r#"You are an AI culinary and nutrition expert.

A user searched Google for: "{search_query}"

Your task: create a SHORT, FACTUAL SEO page that answers this EXACT query.

---

INPUT:
Search Query: {search_query}
Intent Type: {intent}
Main Ingredient: {entity_a}
{entity_b_line}
Language: {locale}

---

OUTPUT FORMAT (STRICT JSON):

{{
  "title": "...",
  "description": "...",
  "answer": "...",
  "faq": [
    {{"q": "...", "a": "..."}},
    {{"q": "...", "a": "..."}}
  ]
}}

---

RULES:

1. TITLE (max 60 chars):
- Must closely match the search query
- Natural, clickable in Google SERP
- Example for "is salmon healthy": "Is Salmon Healthy? Nutrition Facts & Benefits"

2. DESCRIPTION (max 140 chars):
- Concise snippet for Google
- Include key data point (calories, protein, etc.)

3. ANSWER (2-3 sentences):
- Answer the search query DIRECTLY in first sentence
- Include specific numbers (calories per 100g, protein grams, vitamins)
- End with practical advice

4. FAQ (exactly 2):
- Related questions a user would ask NEXT
- Short, data-driven answers
- NOT generic — specific to this ingredient

---

LANGUAGE: Write ONLY in {locale}. Use natural search phrases in that language.

TONE: Expert chef + nutritionist. Practical, not academic. Data-driven.

Return ONLY the JSON, no other text."#,
        search_query = search_query,
        intent = intent,
        entity_a = entity_a,
        entity_b_line = entity_b_line,
        locale = locale,
    )
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

fn parse_response(raw: &str) -> AppResult<SeoContentResponse> {
    // Try direct parse first
    if let Ok(r) = serde_json::from_str::<SeoContentResponse>(raw) {
        return Ok(r);
    }

    // Try extracting JSON from markdown code blocks or surrounding text
    let cleaned = raw.trim();
    let json_str = if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            &cleaned[start..=end]
        } else {
            cleaned
        }
    } else {
        cleaned
    };

    serde_json::from_str::<SeoContentResponse>(json_str).map_err(|e| {
        tracing::error!("Failed to parse AI SEO content response: {} | raw: {}", e, &raw[..raw.len().min(200)]);
        AppError::internal("AI returned invalid JSON for SEO content")
    })
}
