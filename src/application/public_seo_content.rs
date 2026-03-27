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
    /// AI-generated SEO-friendly slug (in content language, auto-transliterated to ASCII)
    #[serde(default)]
    pub slug: Option<String>,
    /// Structured article: heading / text / image blocks
    #[serde(default)]
    pub content_blocks: Vec<ContentBlock>,
    /// Short OG/social description (≤100 chars) for social media previews
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub og_description: Option<String>,
}

/// A single content block inside a structured SEO article.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Heading { level: u8, text: String },
    Text { content: String },
    Image { key: String, alt: String },
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

    /// Invalidate AI cache for a specific entity so next generate() calls LLM fresh.
    /// Deletes ALL cache entries matching "uc:seo_content:" prefix for this entity.
    pub async fn invalidate_cache(&self, req: &SeoContentRequest) {
        let intent = req.intent_type.to_lowercase();
        let locale = req.locale.to_lowercase();
        let entity_a = req.entity_a.trim().to_lowercase();
        let entity_b = req.entity_b.as_deref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());

        // Invalidate generic prompt cache
        let fingerprint = format!(
            "seo_content:{}:{}:{}:{}",
            intent, entity_a, entity_b.as_deref().unwrap_or(""), locale,
        );
        let cache_key = format!("uc:seo_content:{}", hash(&fingerprint));
        let _ = self.ai_cache.delete(&cache_key).await;

        tracing::info!("🗑️ SEO cache invalidated: {} / {} / {}", intent, entity_a, locale);
    }

    /// Invalidate AI cache for a specific entity+query (targeted sub-intents).
    pub async fn invalidate_cache_with_query(&self, req: &SeoContentRequest, search_query: &str) {
        let intent = req.intent_type.to_lowercase();
        let locale = req.locale.to_lowercase();
        let entity_a = req.entity_a.trim().to_lowercase();
        let entity_b = req.entity_b.as_deref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());

        let fingerprint = format!(
            "seo_content_q:{}:{}:{}:{}:{}",
            intent, entity_a, entity_b.as_deref().unwrap_or(""), locale, search_query,
        );
        let cache_key = format!("uc:seo_content:{}", hash(&fingerprint));
        let _ = self.ai_cache.delete(&cache_key).await;

        tracing::info!("🗑️ SEO cache invalidated: '{}' / {}", search_query, locale);
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

        // ── 4. Call LLM (Gemini Pro — high quality) ──
        let raw = self.llm_adapter
            .groq_raw_request_with_model(&prompt, 3200, "gemini-2.5-pro")
            .await?;

        // ── 5. Parse JSON ──
        let response = parse_response(&raw)?;

        // ── 5b. Enforce SEO limits (title ≤60, desc 120-155, etc.) ──
        let response = enforce_seo_limits(response);

        // ── 6. Cache ──
        if let Ok(val) = serde_json::to_value(&response) {
            if let Err(e) = self.ai_cache.set(
                &cache_key,
                val,
                "gemini",
                "gemini-2.5-pro",
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
            .groq_raw_request_with_model(&prompt, 3200, "gemini-2.5-pro")
            .await?;

        // ── 5. Parse ──
        let response = parse_response(&raw)?;

        // ── 5b. Enforce SEO limits ──
        let response = enforce_seo_limits(response);

        // ── 6. Cache ──
        if let Ok(val) = serde_json::to_value(&response) {
            let _ = self.ai_cache.set(&cache_key, val, "gemini", "gemini-2.5-pro", SEO_CONTENT_CACHE_TTL_DAYS).await;
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
        r#"You are a professional culinary SEO writer (chef + nutritionist).

Generate a REAL, high-quality SEO page that ranks in Google.

---

INPUT:
Intent Type: {intent}
Main Ingredient: {entity_a}
{entity_b_line}
Language: {locale}

---

OUTPUT FORMAT (STRICT JSON):

{{
  "slug": "...",
  "title": "...",
  "description": "...",
  "answer": "...",
  "faq": [
    {{"q": "...", "a": "..."}},
    {{"q": "...", "a": "..."}},
    {{"q": "...", "a": "..."}},
    {{"q": "...", "a": "..."}}
  ],
  "content_blocks": [
    {{"type": "heading", "level": 1, "text": "H1 — вопрос/тема"}},
    {{"type": "text", "content": "2-3 предложения — краткий ответ, сразу ценность"}},
    {{"type": "image", "key": "hero", "alt": "{{{{keyword}}}} — внешний вид продукта, appetizing photo"}},
    {{"type": "text", "content": "1-2 предложения — переход к следующей секции, micro-value"}},
    {{"type": "heading", "level": 2, "text": "Польза / свойства"}},
    {{"type": "text", "content": "3-4 предложения с конкретными витаминами, минералами, свойствами"}},
    {{"type": "image", "key": "benefits", "alt": "польза {{{{ingredient}}}} для здоровья, витамины и свойства"}},
    {{"type": "text", "content": "1-2 предложения — кому особенно полезен, при каких состояниях"}},
    {{"type": "heading", "level": 2, "text": "Калории и состав"}},
    {{"type": "text", "content": "3-4 предложения: точные цифры kcal, белки, жиры, углеводы, витамины на 100г"}},
    {{"type": "image", "key": "nutrition", "alt": "{{{{ingredient}}}} калории на 100г, белки, жиры, углеводы, витамины"}},
    {{"type": "text", "content": "1-2 предложения — сравнение с аналогами или суточная норма"}},
    {{"type": "heading", "level": 2, "text": "Как употреблять / советы"}},
    {{"type": "text", "content": "3-4 предложения: способы приготовления, порции, сочетания, температура"}},
    {{"type": "image", "key": "cooking", "alt": "как готовить {{{{ingredient}}}}, блюда и способы приготовления"}},
    {{"type": "text", "content": "1-2 предложения — итоговый совет, рекомендация шефа"}}
  ]
}}

---

STRICT RULES FOR CONTENT_BLOCKS:

The article MUST follow this EXACT pattern (16 blocks):

[H1] → [TEXT intro] → [IMAGE hero] → [TEXT bridge]
→ [H2 Польза] → [TEXT] → [IMAGE benefits] → [TEXT]
→ [H2 Калории] → [TEXT] → [IMAGE nutrition] → [TEXT]
→ [H2 Как употреблять] → [TEXT] → [IMAGE cooking] → [TEXT]

EVERY image block MUST be followed by a TEXT block (1-2 sentences, micro-value).
EVERY image block MUST be preceded by a TEXT block (main content of that section).
This creates a rhythm: text → image → text → heading → text → image → text...

EXACTLY 4 image blocks: hero, benefits, nutrition, cooking.
EXACTLY 4 heading blocks: 1x H1 + 3x H2.
EXACTLY 8 text blocks.
Total: 16 blocks.

IMAGE ALT REQUIREMENTS:
- In content language ({locale})
- SEO-rich: include ingredient name + specific benefit/data
- 80-150 characters
- hero: product appearance, appetizing look
- benefits: health benefits, specific vitamins (e.g. "витамин C, калий, антиоксиданты")
- nutrition: calorie data, macros (e.g. "47 ккал, 3г белка на 100г")
- cooking: cooking method, dish type (e.g. "запеченный при 180°C с травами")

TEXT BLOCK REQUIREMENTS:
- Short paragraphs: 2-4 sentences max
- Use SPECIFIC data: numbers, vitamin names, temperatures, weights
- No filler phrases, no "вода"
- Entity enrichment: named vitamins, minerals, temperatures, weights, conditions
- Each text block adds unique value (never repeat)

---

STRICT SEO RULES:

0. SLUG (in the CONTENT language, 3-6 words separated by hyphens):
- Must reflect the MAIN search intent as a real user search query
- Write the slug in the SAME language as the content (ru/pl/uk/en)
- The system will auto-transliterate to Latin if needed
- Examples:
  - RU: question about artichoke health → "polezen-li-artishok"
  - RU: salmon calories → "kalorijnost-lososya"
  - EN: is artichoke healthy → "is-artichoke-healthy"
  - PL: czy karczoch jest zdrowy → "czy-karczoch-jest-zdrowy"
  - UK: чи корисний артишок → "chi-korisnij-artishok"
- NEVER use generic slugs like "question-artichoke" or "goal-salmon"

1. TITLE (HARD LIMIT: 50-60 characters, NEVER exceed 60):
- Count characters carefully. Google truncates at 60.
- Include main keyword + benefit/hook
- Example (ru): "Артишок: польза, калории и витамины"
- Example (en): "Olives: Nutrition, Benefits & Calories"
- If over 60 chars → shorten. This is non-negotiable.

2. DESCRIPTION (HARD LIMIT: 120-155 characters, NEVER exceed 155):
- Count characters carefully. Google truncates at 155.
- Start with ingredient + key data
- Include CTA: "Узнайте" / "Learn" / "Dowiedz się"

3. ANSWER (400-800 characters, 4-6 sentences):
- Sentence 1: direct answer
- Sentences 2-3: specific numbers (kcal, protein, vitamins)
- Sentence 4: practical advice (how to cook, portion)
- Sentence 5: who benefits most
- Sentence 6: strong takeaway
- ENTITY ENRICHMENT: named entities only, NO generic words

4. FAQ (exactly 4 questions):
- a) Daily use: "Можно ли есть X каждый день?"
- b) Nutrition: "Сколько калорий в X?"
- c) Health goal: "Помогает ли X для похудения?"
- d) Cooking: "Как лучше готовить X?"
- Each answer: 2-3 sentences with SPECIFIC data

---

INTENT LOGIC:

question → Answer directly, explain why with data
comparison → Compare both, highlight 3+ differences, give recommendation
goal → Focus on the goal, suggest best approach with this ingredient
combo → Explain synergy, nutritional balance, recipe ideas

---

LANGUAGE: {locale}
- All content in {locale}. Write natively, DO NOT translate from English.
- Slug in content language (auto-transliterated by system).
- Use natural phrases that REAL users search for in this language.

TONE: Expert chef + nutritionist. Data-driven, practical, confident. No fluff.

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
        r#"You are a professional culinary SEO writer (chef + nutritionist).

A user searched Google for: "{search_query}"

Create a HIGH-QUALITY SEO page that answers this EXACT query and ranks in Google.

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
  "slug": "...",
  "title": "...",
  "description": "...",
  "answer": "...",
  "faq": [
    {{"q": "...", "a": "..."}},
    {{"q": "...", "a": "..."}},
    {{"q": "...", "a": "..."}},
    {{"q": "...", "a": "..."}}
  ],
  "content_blocks": [
    {{"type": "heading", "level": 1, "text": "H1 — вопрос/тема из поиска"}},
    {{"type": "text", "content": "2-3 предложения — прямой ответ на запрос, сразу ценность"}},
    {{"type": "image", "key": "hero", "alt": "{{{{keyword}}}} — внешний вид продукта, appetizing photo"}},
    {{"type": "text", "content": "1-2 предложения — переход к пользе, micro-value"}},
    {{"type": "heading", "level": 2, "text": "Польза / свойства"}},
    {{"type": "text", "content": "3-4 предложения с конкретными витаминами, минералами, свойствами"}},
    {{"type": "image", "key": "benefits", "alt": "польза {{{{ingredient}}}} для здоровья, витамины и свойства"}},
    {{"type": "text", "content": "1-2 предложения — кому особенно полезен, при каких состояниях"}},
    {{"type": "heading", "level": 2, "text": "Калории и состав"}},
    {{"type": "text", "content": "3-4 предложения: точные цифры kcal, белки, жиры, углеводы, витамины на 100г"}},
    {{"type": "image", "key": "nutrition", "alt": "{{{{ingredient}}}} калории на 100г, белки, жиры, углеводы, витамины"}},
    {{"type": "text", "content": "1-2 предложения — сравнение с аналогами или суточная норма"}},
    {{"type": "heading", "level": 2, "text": "Как употреблять / советы"}},
    {{"type": "text", "content": "3-4 предложения: способы приготовления, порции, сочетания, температура"}},
    {{"type": "image", "key": "cooking", "alt": "как готовить {{{{ingredient}}}}, блюда и способы приготовления"}},
    {{"type": "text", "content": "1-2 предложения — итоговый совет, рекомендация шефа"}}
  ]
}}

---

STRICT RULES FOR CONTENT_BLOCKS:

The article MUST follow this EXACT pattern (16 blocks):

[H1] → [TEXT intro] → [IMAGE hero] → [TEXT bridge]
→ [H2 Польза] → [TEXT] → [IMAGE benefits] → [TEXT]
→ [H2 Калории] → [TEXT] → [IMAGE nutrition] → [TEXT]
→ [H2 Как употреблять] → [TEXT] → [IMAGE cooking] → [TEXT]

EVERY image block MUST be followed by a TEXT block (1-2 sentences, micro-value).
EVERY image block MUST be preceded by a TEXT block (main content of that section).
This creates a rhythm: text → image → text → heading → text → image → text...

EXACTLY 4 image blocks: hero, benefits, nutrition, cooking.
EXACTLY 4 heading blocks: 1x H1 + 3x H2.
EXACTLY 8 text blocks.
Total: 16 blocks.

IMAGE ALT REQUIREMENTS:
- In content language ({locale})
- SEO-rich: include ingredient name + specific benefit/data
- 80-150 characters
- hero: product appearance, appetizing look
- benefits: health benefits, specific vitamins (e.g. "витамин C, калий, антиоксиданты")
- nutrition: calorie data, macros (e.g. "47 ккал, 3г белка на 100г")
- cooking: cooking method, dish type (e.g. "запеченный при 180°C с травами")

TEXT BLOCK REQUIREMENTS:
- Short paragraphs: 2-4 sentences max
- Use SPECIFIC data: numbers, vitamin names, temperatures, weights
- No filler phrases, no "вода"
- Entity enrichment: named vitamins, minerals, temperatures, weights, conditions
- Each text block adds unique value (never repeat)

---

STRICT SEO RULES:

0. SLUG (in the CONTENT language, 3-6 words separated by hyphens):
- Must reflect the search query intent
- Write the slug in the SAME language as the content
- The system will auto-transliterate to Latin if needed

1. TITLE (HARD LIMIT: 50-60 characters, NEVER exceed 60):
- Count characters carefully. Google truncates at 60.
- Must closely match the search query
- Add a benefit/hook after the main keyword
- If over 60 chars → shorten. This is non-negotiable.

2. DESCRIPTION (HARD LIMIT: 120-155 characters, NEVER exceed 155):
- Count characters carefully. Google truncates at 155.
- Start with ingredient + key data
- Include CTA: "Узнайте" / "Learn" / "Dowiedz się"

3. ANSWER (400-800 characters, 4-6 sentences):
- Sentence 1: Answer the search query DIRECTLY
- Sentences 2-3: specific numbers (kcal, protein, vitamins)
- Sentence 4: practical advice
- Sentence 5: who benefits most
- Sentence 6: strong takeaway
- ENTITY ENRICHMENT: named entities only, NO generic words

4. FAQ (exactly 4 questions):
- a) Daily use  b) Nutrition  c) Health goal  d) Cooking
- Each answer: 2-3 sentences with SPECIFIC data

---

LANGUAGE: {locale}
- All content in {locale}. Write natively, DO NOT translate from English.
- Slug in content language (auto-transliterated by system).
- Use natural search phrases for this language.

TONE: Expert chef + nutritionist. Data-driven, practical, confident. No fluff.

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
        let end = raw.char_indices().nth(200).map(|(i, _)| i).unwrap_or(raw.len());
        tracing::error!("Failed to parse AI SEO content response: {} | raw: {}", e, &raw[..end]);
        AppError::internal("AI returned invalid JSON for SEO content")
    })
}

// ── SEO post-processing ─────────────────────────────────────────────────────

/// Maximum character limits per Google SEO best practices (2024-2026)
const MAX_TITLE_CHARS: usize = 60;
const MAX_DESCRIPTION_CHARS: usize = 155;
const MAX_ANSWER_CHARS: usize = 800;
const MAX_OG_DESCRIPTION_CHARS: usize = 100;

/// Enforce SEO character limits on AI output.
///
/// Google truncates titles >60 chars and descriptions >155 chars.
/// This function guarantees compliance even when AI ignores the prompt rules.
fn enforce_seo_limits(mut resp: SeoContentResponse) -> SeoContentResponse {
    // ── Title: hard cap at 60 chars ──
    if resp.title.chars().count() > MAX_TITLE_CHARS {
        resp.title = smart_truncate(&resp.title, MAX_TITLE_CHARS);
        tracing::info!("✂️ SEO title truncated to {} chars", resp.title.chars().count());
    }

    // ── Description: 120-155 chars ──
    let desc_len = resp.description.chars().count();
    if desc_len > MAX_DESCRIPTION_CHARS {
        resp.description = smart_truncate(&resp.description, MAX_DESCRIPTION_CHARS);
        tracing::info!("✂️ SEO description truncated to {} chars", resp.description.chars().count());
    }

    // ── Answer: cap at 800 chars ──
    if resp.answer.chars().count() > MAX_ANSWER_CHARS {
        resp.answer = smart_truncate(&resp.answer, MAX_ANSWER_CHARS);
    }

    // ── OG description: derive from description if AI didn't provide one ──
    if resp.og_description.is_none() || resp.og_description.as_deref().map_or(true, |s| s.trim().is_empty()) {
        // Take first sentence of description, cap at 100 chars
        let first_sentence = resp.description
            .split_once(". ")
            .map(|(s, _)| format!("{}.", s))
            .unwrap_or_else(|| resp.description.clone());
        resp.og_description = Some(smart_truncate(&first_sentence, MAX_OG_DESCRIPTION_CHARS));
    }

    // ── H1 from content_blocks: ensure exists and ≤60 chars ──
    if let Some(blocks) = resp.content_blocks.first_mut() {
        if let ContentBlock::Heading { level: 1, ref mut text } = blocks {
            if text.chars().count() > MAX_TITLE_CHARS {
                *text = smart_truncate(text, MAX_TITLE_CHARS);
            }
        }
    }

    // ── FAQ answers: cap each at 300 chars ──
    for faq in &mut resp.faq {
        if faq.a.chars().count() > 300 {
            faq.a = smart_truncate(&faq.a, 300);
        }
    }

    resp
}

/// Truncate a string to max_chars on a word boundary, adding "…" if truncated.
/// Respects UTF-8 multi-byte characters.
fn smart_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }

    // Reserve 1 char for "…"
    let target = max_chars.saturating_sub(1);
    let mut result = String::new();
    let mut char_count = 0;
    let mut last_space = 0;

    for ch in s.chars() {
        if char_count >= target {
            break;
        }
        if ch == ' ' {
            last_space = result.len();
        }
        result.push(ch);
        char_count += 1;
    }

    // Cut at last word boundary if possible (avoid cutting mid-word)
    if last_space > 0 && last_space > result.len() / 2 {
        result.truncate(last_space);
    }

    // Remove trailing punctuation before ellipsis
    let trimmed = result.trim_end_matches(|c: char| c == ',' || c == ';' || c == ':' || c == '-' || c == ' ');
    format!("{}…", trimmed)
}
