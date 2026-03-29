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
    #[serde(default)]
    pub name_ru: String,
    /// Название на польском
    #[serde(default)]
    pub name_pl: String,
    /// Emoji для карточки
    #[serde(default)]
    pub emoji: String,
    /// Тип продукта (vegetable, fruit, supplement, nut_seed, ...)
    #[serde(default)]
    pub product_type: String,
    /// Короткое описание почему стоит добавить (1 предложение, по-русски)
    #[serde(default)]
    pub why_add: String,
    /// Примерные калории на 100г (для быстрого ориентира)
    #[serde(default)]
    pub calories_hint: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SuggestProductsResponse {
    pub suggestions: Vec<ProductSuggestion>,
    pub query: String,
    pub cached: bool,
}

impl AdminCatalogService {
    /// Быстрый запрос: список name_en всех активных продуктов (для exclude-листа)
    /// Возвращает компактную строку: "Avocado, Chicken Breast, Turmeric, ..."
    /// ~500 продуктов ≈ 2000 символов ≈ 500 токенов — дёшево!
    async fn get_existing_product_names(&self) -> AppResult<String> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name_en FROM catalog_ingredients WHERE is_active = true ORDER BY name_en"
        )
        .fetch_all(&self.pool)
        .await?;

        let names: Vec<String> = rows.into_iter().map(|(n,)| n).collect();
        tracing::info!("📋 Catalog has {} active products for exclude-list", names.len());
        Ok(names.join(", "))
    }

    /// AI предлагает 5 продуктов по запросу — исключая то что уже есть в каталоге
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

        // ── Загрузить существующие продукты (дёшево — только name_en) ──
        let existing_names = self.get_existing_product_names().await?;

        // ── Cache check (включаем кол-во продуктов в ключ — инвалидация при изменении каталога) ──
        let product_count = existing_names.matches(',').count() + if existing_names.is_empty() { 0 } else { 1 };
        let cache_key = format!(
            "uc:suggest:v2:{}:n{}",
            sha256_short(&query.to_lowercase()),
            product_count,
        );
        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            if let Ok(suggestions) = serde_json::from_value::<Vec<ProductSuggestion>>(cached) {
                tracing::info!("📦 Suggest cache hit: {}", &query[..query.len().min(40)]);
                return Ok(SuggestProductsResponse { suggestions, query, cached: true });
            }
        }

        let prompt = build_suggest_prompt(&query, &existing_names);
        let raw = self
            .llm_adapter
            .generate_with_quality(&prompt, 4000, AiQuality::Balanced)
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
            "✅ AI suggested {} products for '{}' (excluded {} existing)",
            suggestions.len(),
            &query[..query.len().min(40)],
            product_count,
        );

        Ok(SuggestProductsResponse { suggestions, query, cached: false })
    }
}

// ── Prompt builder ────────────────────────────────────────────────────────────

fn build_suggest_prompt(query: &str, existing_products: &str) -> String {
    let exclude_block = if existing_products.is_empty() {
        String::from("The catalog is currently empty — suggest any relevant products.")
    } else {
        format!(
            "ALREADY IN CATALOG (DO NOT suggest these):\n{}\n\nYou MUST suggest products that are NOT in the list above.",
            existing_products
        )
    };

    format!(
        r#"You are a food catalog expert. An admin of a food ingredient catalog is looking to add NEW products that are NOT yet in the catalog.

Admin query: "{query}"

{exclude_block}

Suggest exactly 5 specific food ingredients/products that:
- Match the admin's query
- Are NOT already in the catalog (see list above)
- Are real, specific ingredients (not dishes or meals)
- Are health-focused or interesting for cooking
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
        exclude_block = exclude_block,
    )
}

// ── Response parser ───────────────────────────────────────────────────────────

fn strip_markdown_fences(raw: &str) -> String {
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
}

fn parse_suggestions(raw: &str) -> AppResult<Vec<ProductSuggestion>> {
    let cleaned = strip_markdown_fences(raw);

    // 1) Try direct parse as array
    if let Ok(suggestions) = serde_json::from_str::<Vec<ProductSuggestion>>(&cleaned) {
        return Ok(suggestions);
    }

    // 2) Find [...] block
    if let Some(start) = cleaned.find('[') {
        if let Some(end) = cleaned.rfind(']') {
            if let Ok(suggestions) = serde_json::from_str::<Vec<ProductSuggestion>>(&cleaned[start..=end]) {
                return Ok(suggestions);
            }
        }

        // 3) JSON truncated — try to close it and parse partial results
        //    Find the last complete object (ends with '}')
        let array_body = &cleaned[start..];
        if let Some(last_brace) = array_body.rfind('}') {
            let partial = format!("{}]", &array_body[..=last_brace]);
            if let Ok(suggestions) = serde_json::from_str::<Vec<ProductSuggestion>>(&partial) {
                if !suggestions.is_empty() {
                    tracing::warn!(
                        "⚠️ Suggest JSON was truncated — recovered {} of 5 items",
                        suggestions.len()
                    );
                    return Ok(suggestions);
                }
            }

            // 4) Even that failed — try individual objects between { }
            let mut items = Vec::new();
            let mut search_from = 0;
            let haystack = array_body;
            while let Some(obj_start) = haystack[search_from..].find('{') {
                let abs_start = search_from + obj_start;
                if let Some(obj_end) = haystack[abs_start..].find('}') {
                    let obj_str = &haystack[abs_start..=abs_start + obj_end];
                    if let Ok(item) = serde_json::from_str::<ProductSuggestion>(obj_str) {
                        items.push(item);
                    }
                    search_from = abs_start + obj_end + 1;
                } else {
                    break;
                }
            }
            if !items.is_empty() {
                tracing::warn!(
                    "⚠️ Suggest JSON deeply broken — extracted {} items individually",
                    items.len()
                );
                return Ok(items);
            }
        }
    }

    tracing::error!("Failed to parse suggestions. Raw ({} chars): {}", raw.len(), &raw[..raw.len().min(500)]);
    Err(AppError::internal("AI returned invalid suggestions format"))
}

fn sha256_short(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}
