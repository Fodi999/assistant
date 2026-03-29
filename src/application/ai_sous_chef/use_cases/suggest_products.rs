//! Use Case: AI Suggest Products — генерирует 5 вариантов продуктов которых нет в каталоге
//!
//! Флоу:
//! 1. Принять свободный ввод ("суперфуды", "экзотические фрукты", "протеин для спорта")
//! 2. AI генерирует 5 конкретных продуктов которые подходят под запрос
//! 3. Каждый вариант: name_en, короткое описание, product_type, почему стоит добавить
//! 4. Фронтенд показывает карточки — администратор выбирает один
//! 5. Выбранный вариант идёт в create_product_draft

use crate::application::admin_catalog::AdminCatalogService;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SuggestProductsRequest {
    /// Свободный ввод на любом языке: "суперфуды", "экзотические орехи", "семена для ЗОЖ"
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductSuggestion {
    /// Английское название (для последующего create_product_draft)
    pub name_en: String,
    /// Название на русском для отображения
    pub name_ru: String,
    /// Название на польском
    pub name_pl: String,
    /// Emoji для карточки
    pub emoji: String,
    /// Тип продукта (vegetable, fruit, supplement, nut_seed, ...)
    pub product_type: String,
    /// Короткое описание почему стоит добавить (1 предложение, по-русски)
    pub why_add: String,
    /// Примерные калории на 100г (для быстрого ориентира)
    pub calories_hint: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SuggestProductsResponse {
    pub suggestions: Vec<ProductSuggestion>,
    pub query: String,
    pub cached: bool,
}

impl AdminCatalogService {
    /// AI предлагает 5 продуктов по запросу — для добавления в каталог
    pub async fn ai_suggest_products(
        &self,
        req: SuggestProductsRequest,
    ) -> AppResult<SuggestProductsResponse> {
        let query = req.query.trim().to_string();
        if query.is_empty() {
            return Err(AppError::validation("Query cannot be empty"));
        }
        if query.len() > 200 {
            return Err(AppError::validation("Query too long (max 200 chars)"));
        }

        // ── Cache check ──
        let cache_key = format!("uc:suggest:v1:{}", sha256_short(&query.to_lowercase()));
        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            if let Ok(suggestions) = serde_json::from_value::<Vec<ProductSuggestion>>(cached) {
                tracing::info!("📦 Suggest cache hit: {}", &query[..query.len().min(40)]);
                return Ok(SuggestProductsResponse { suggestions, query, cached: true });
            }
        }

        let prompt = build_suggest_prompt(&query);
        let raw = self
            .llm_adapter
            .generate_with_quality(&prompt, 2000, AiQuality::Balanced)
            .await?;

        tracing::debug!("🤖 Suggest raw ({} chars): {}", raw.len(), &raw[..raw.len().min(300)]);

        let suggestions = parse_suggestions(&raw)?;

        if suggestions.is_empty() {
            return Err(AppError::internal("AI returned no suggestions"));
        }

        // ── Cache 24h ──
        if let Ok(val) = serde_json::to_value(&suggestions) {
            let _ = self.ai_cache.set(&cache_key, val, "gemini", "gemini-3.1-pro-preview", 1).await;
        }

        tracing::info!(
            "✅ AI suggested {} products for '{}'",
            suggestions.len(),
            &query[..query.len().min(40)]
        );

        Ok(SuggestProductsResponse { suggestions, query, cached: false })
    }
}

// ── Prompt builder ────────────────────────────────────────────────────────────

fn build_suggest_prompt(query: &str) -> String {
    format!(
        r#"You are a food catalog expert. An admin of a food ingredient catalog is looking to add new products.

Admin query: "{query}"

Suggest exactly 5 specific food ingredients/products that match this query and would be valuable to add to a food catalog.
Focus on products that are:
- Real, specific ingredients (not dishes or meals)
- Health-focused or interesting for cooking
- Can be described with standard nutritional data

Return ONLY valid JSON array with exactly 5 items:
[
  {{
    "name_en": "Chia Seeds",
    "name_ru": "Семена чиа",
    "name_pl": "Nasiona chia",
    "emoji": "🌱",
    "product_type": "nut_seed",
    "why_add": "Богаты омега-3, клетчаткой и белком — популярный суперфуд",
    "calories_hint": 486
  }}
]

product_type must be one of: vegetable, fruit, grain, legume, meat, poultry, fish, seafood, dairy, egg, fat_oil, nut_seed, herb_spice, mushroom, beverage, sweetener, condiment, bakery, supplement

Return ONLY the JSON array, no extra text."#,
        query = query,
    )
}

// ── Response parser ───────────────────────────────────────────────────────────

fn parse_suggestions(raw: &str) -> AppResult<Vec<ProductSuggestion>> {
    // Strip markdown fences
    let cleaned = {
        let t = raw.trim();
        let without_prefix = if t.starts_with("```json") {
            &t[7..]
        } else if t.starts_with("```") {
            &t[3..]
        } else {
            t
        };
        let without_suffix = if without_prefix.trim_end().ends_with("```") {
            let s = without_prefix.trim_end();
            &s[..s.len() - 3]
        } else {
            without_prefix
        };
        without_suffix.trim().to_string()
    };

    // Try direct parse as array
    if let Ok(suggestions) = serde_json::from_str::<Vec<ProductSuggestion>>(&cleaned) {
        return Ok(suggestions);
    }

    // Fallback: find [...] block
    if let Some(start) = cleaned.find('[') {
        if let Some(end) = cleaned.rfind(']') {
            if let Ok(suggestions) = serde_json::from_str::<Vec<ProductSuggestion>>(&cleaned[start..=end]) {
                return Ok(suggestions);
            }
        }
    }

    tracing::error!("Failed to parse suggestions. Raw: {}", &raw[..raw.len().min(500)]);
    Err(AppError::internal("AI returned invalid suggestions format"))
}

fn sha256_short(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}
