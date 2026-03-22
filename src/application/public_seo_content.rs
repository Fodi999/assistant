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
    /// AI-generated SEO-friendly slug (always English, e.g. "is-artichoke-healthy")
    #[serde(default)]
    pub slug: Option<String>,
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
            .groq_raw_request_with_model(&prompt, 1200, "llama-3.3-70b-versatile")
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
            .groq_raw_request_with_model(&prompt, 1200, "llama-3.3-70b-versatile")
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
  ]
}}

---

STRICT SEO RULES:

0. SLUG (ALWAYS in English, 3-6 words separated by hyphens):
- Must reflect the MAIN search intent, not just "intent-ingredient"
- Use real search query keywords
- Examples:
  - question about artichoke health → "is-artichoke-healthy"
  - question about salmon calories → "salmon-calories-nutrition"
  - comparison salmon vs tuna → "salmon-vs-tuna-nutrition"
  - goal: artichoke for skin → "artichoke-benefits-for-skin"
- NEVER use generic slugs like "question-artichoke" or "goal-salmon"
- ALWAYS English, even if content language is ru/pl/uk

1. TITLE:
- MUST be 50-60 characters (this is critical for Google SERP)
- Include main keyword + a benefit/hook
- Example (ru): "Полезен ли артишок для здоровья? Польза, калории и витамины"
- Example (en): "Is Artichoke Healthy? Benefits, Calories & Nutrition"
- DO NOT write titles shorter than 45 characters

2. DESCRIPTION:
- MUST be 120-155 characters (Google truncates at 155)
- Start with ingredient + key data (calories, protein)
- Include "Узнайте" / "Learn" / "Dowiedz się" — a search CTA
- Example (ru): "Артишок — 60 калорий и 3 г белка на 100 г. Узнайте, чем он полезен, какие витамины содержит и как влияет на здоровье."
- DO NOT write descriptions shorter than 120 characters

3. ANSWER:
- MUST be 400-800 characters (4-6 sentences)
- Sentence 1: answer the question directly
- Sentences 2-3: specific numbers (calories per 100g, protein grams, key vitamins)
- Sentence 4: practical advice (how to use, when to eat, portion size)
- Sentence 5: who benefits most (a "Кому полезен" block, e.g. "для пищеварения, иммунитета, кожи")
- End with a strong takeaway
- Write naturally, like a chef explaining to a colleague

4. FAQ:
- MUST have exactly 4 questions
- Real user questions from Google (People Also Ask)
- Each answer: 2-3 sentences with SPECIFIC data
- Cover these 4 angles:
  a) Daily use: "Можно ли есть X каждый день?" / "Can you eat X every day?"
  b) Nutrition: "Сколько калорий в X?" / "How many calories in X?"
  c) Health goal: "Помогает ли X для похудения?" / "Does X help with weight loss?"
  d) Cooking: "Как лучше готовить X?" / "How to cook X?"

---

INTENT LOGIC:

question → Answer directly, explain why with data
comparison → Compare both, highlight 3+ differences, give recommendation
goal → Focus on the goal, suggest best approach with this ingredient
combo → Explain synergy, nutritional balance, recipe ideas

---

LANGUAGE: {locale}
- Title, description, answer, FAQ → write in {locale}
- Slug → ALWAYS in English
- Use natural phrases that REAL users search for in this language
- DO NOT translate from English — write natively
- Match the search intent in that language's culture

TONE: Expert chef + nutritionist. Practical, data-driven, confident.

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
  ]
}}

---

STRICT SEO RULES:

0. SLUG (ALWAYS in English, 3-6 words separated by hyphens):
- Must reflect the search query intent
- Use the same keywords as the search query (transliterated to English if needed)
- Examples:
  - "Is salmon healthy?" → "is-salmon-healthy"
  - "калории лосось" → "salmon-calories-nutrition"
  - "salmon vs tuna nutrition" → "salmon-vs-tuna-nutrition"
  - "артишок для кожи" → "artichoke-benefits-for-skin"
- NEVER use generic slugs like "question-artichoke" or "goal-salmon"
- ALWAYS English, regardless of content language

1. TITLE (50-60 characters, NEVER shorter):
- Must closely match the search query
- Add a benefit/hook after the main keyword
- Natural, clickable in Google SERP
- Example for "is salmon healthy": "Is Salmon Healthy? Nutrition Facts, Benefits & Risks"
- Example for "калории лосось": "Калорийность лосося: БЖУ на 100г, польза и сравнение"

2. DESCRIPTION (120-155 characters, NEVER shorter than 120):
- Start with ingredient + key data (calories, protein)
- Include "Узнайте" / "Learn" / "Dowiedz się" — a search CTA
- Example: "Лосось — 208 ккал, 20г белка и омега-3. Узнайте, как он влияет на здоровье и сколько можно есть в день."

3. ANSWER (400-800 characters, 4-6 sentences):
- Sentence 1: Answer the search query DIRECTLY
- Sentences 2-3: Specific numbers (calories per 100g, protein, key vitamins)
- Sentence 4: Practical advice (how to cook, when to eat, portion size)
- Sentence 5: Who benefits most ("Кому полезен: для пищеварения, иммунитета, кожи")
- Sentence 6: Strong takeaway
- Write naturally like a chef explaining to a colleague
- DO NOT use generic filler phrases

4. FAQ (exactly 4 questions):
- Related questions from Google "People Also Ask"
- Each answer: 2-3 sentences with SPECIFIC data
- Cover these 4 angles:
  a) Daily use: "Можно ли есть X каждый день?"
  b) Nutrition: "Сколько калорий в X?"
  c) Health goal: "Помогает ли X для похудения?"
  d) Cooking: "Как лучше готовить X?"

---

LANGUAGE: {locale}
- Title, description, answer, FAQ → write in {locale}
- Slug → ALWAYS in English
- Use phrases that REAL users type into Google in this language
- DO NOT translate from English — write natively
- Match cultural context of that language

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
        tracing::error!("Failed to parse AI SEO content response: {} | raw: {}", e, &raw[..raw.len().min(200)]);
        AppError::internal("AI returned invalid JSON for SEO content")
    })
}
