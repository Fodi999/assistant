//! Use Case: AI Suggest Products
//!
//! Архитектура: AI → Rust filter → supplement → stop (max 3 попытки)
//!
//! Флоу:
//! 1. Загрузить Set<slug> из БД — deterministic, бесплатно, быстро
//! 2. AI генерирует ~10 кандидатов
//! 3. Rust фильтрует дубли через is_duplicate() — без AI, без токенов
//! 4. Если мало (<5) → догенерация: AI получает rejected + accepted списки
//! 5. max_attempts = 3, если всё ещё мало → вернуть partial (лучше чем ничего)

use crate::application::admin_catalog::AdminCatalogService;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const TARGET: usize = 5;       // сколько нужно вернуть
const MAX_ATTEMPTS: usize = 3; // лимит попыток

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SuggestProductsRequest {
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductSuggestion {
    pub name_en: String,
    #[serde(default)]
    pub name_ru: String,
    #[serde(default)]
    pub name_pl: String,
    #[serde(default)]
    pub emoji: String,
    #[serde(default)]
    pub product_type: String,
    #[serde(default)]
    pub why_add: String,
    #[serde(default)]
    pub calories_hint: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SuggestProductsResponse {
    pub suggestions: Vec<ProductSuggestion>,
    pub query: String,
    pub cached: bool,
    /// Сколько попыток потребовалось AI (1-3)
    pub attempts: usize,
}

// ── Deterministic duplicate checker (NO AI, NO tokens) ───────────────────────

/// Нормализует название в slug для сравнения.
/// "Chia Seeds" == "chia seeds" == "chia-seeds"
fn normalize_slug(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Проверяет является ли кандидат дублём.
/// Использует предзагруженный Set<slug> — O(1), 0 токенов.
fn is_duplicate(name_en: &str, existing_slugs: &HashSet<String>) -> bool {
    existing_slugs.contains(&normalize_slug(name_en))
}

/// Проверяет дубль внутри текущей сессии (уже принятые кандидаты)
fn is_already_accepted(name_en: &str, accepted: &[ProductSuggestion]) -> bool {
    let slug = normalize_slug(name_en);
    accepted.iter().any(|a| normalize_slug(&a.name_en) == slug)
}

// ── Service impl ──────────────────────────────────────────────────────────────

impl AdminCatalogService {
    /// Загружает Set<normalized_slug> всех активных продуктов из БД.
    /// Дёшево: один SQL запрос, только name_en. 500 продуктов < 1мс.
    async fn load_catalog_slugs(&self) -> AppResult<(HashSet<String>, usize)> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name_en FROM catalog_ingredients WHERE is_active = true",
        )
        .fetch_all(&self.pool)
        .await?;

        let count = rows.len();
        let slugs: HashSet<String> = rows
            .into_iter()
            .map(|(n,)| normalize_slug(&n))
            .collect();

        tracing::info!("📋 Catalog slugs loaded: {} products", count);
        Ok((slugs, count))
    }

    /// Главный метод: AI → filter → supplement → max 3 попытки
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

        // ── 1. Загрузить БД slugs (deterministic, бесплатно) ──
        let (catalog_slugs, catalog_count) = self.load_catalog_slugs().await?;

        // ── 2. Cache check ──
        let cache_key = format!(
            "uc:suggest:v3:{}:n{}",
            sha256_short(&query.to_lowercase()),
            catalog_count,
        );
        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            if let Ok(suggestions) = serde_json::from_value::<Vec<ProductSuggestion>>(cached) {
                tracing::info!("📦 Suggest cache hit for '{}'", &query[..query.len().min(40)]);
                return Ok(SuggestProductsResponse {
                    suggestions,
                    query,
                    cached: true,
                    attempts: 0,
                });
            }
        }

        // ── 3. Основной цикл: AI → filter → supplement ──
        let mut accepted: Vec<ProductSuggestion> = Vec::new();
        let mut rejected_names: Vec<String> = Vec::new(); // для следующего промпта
        let mut attempts = 0;

        while accepted.len() < TARGET && attempts < MAX_ATTEMPTS {
            attempts += 1;
            let need = TARGET - accepted.len();

            tracing::info!(
                "🤖 Attempt {}/{}: need {} more, have {}, rejected so far: {}",
                attempts, MAX_ATTEMPTS, need, accepted.len(), rejected_names.len()
            );

            // Первый запрос: каталог целиком (только имена)
            // Последующие: пустой (AI уже знает каталог, промпт короче)
            let empty_set = HashSet::new();
            let slugs_for_prompt = if attempts == 1 { &catalog_slugs } else { &empty_set };

            let prompt = build_suggest_prompt(
                &query,
                slugs_for_prompt,
                &accepted,
                &rejected_names,
                // Просим больше чем нужно — запас для фильтрации
                need + 3,
            );

            let raw = self
                .llm_adapter
                // Flash model: thinking uses ~80% of token budget
                .generate_with_quality(&prompt, 8000, AiQuality::Fast)
                .await?;

            let candidates = match try_parse_suggestions(&raw) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("⚠️ Attempt {} parse error: {}", attempts, e);
                    break;
                }
            };

            tracing::info!(
                "   AI returned {} candidates, filtering...",
                candidates.len()
            );

            // ── 4. Rust фильтрация — deterministic, 0 токенов ──
            for candidate in candidates {
                if candidate.name_en.trim().is_empty() {
                    continue;
                }

                let dup_in_catalog = is_duplicate(&candidate.name_en, &catalog_slugs);
                let dup_in_accepted = is_already_accepted(&candidate.name_en, &accepted);

                if dup_in_catalog || dup_in_accepted {
                    tracing::debug!(
                        "   ❌ Rejected '{}' (catalog={}, session={})",
                        candidate.name_en, dup_in_catalog, dup_in_accepted
                    );
                    rejected_names.push(candidate.name_en.clone());
                } else {
                    tracing::debug!("   ✅ Accepted '{}'", candidate.name_en);
                    accepted.push(candidate);
                    if accepted.len() >= TARGET {
                        break;
                    }
                }
            }
        }

        // ── 5. Результат (partial если не хватило) ──
        if accepted.is_empty() {
            return Err(AppError::internal("AI could not suggest any new products"));
        }

        if accepted.len() < TARGET {
            tracing::warn!(
                "⚠️ Only {}/{} suggestions after {} attempts (partial result accepted)",
                accepted.len(), TARGET, attempts
            );
        } else {
            tracing::info!(
                "✅ Got {}/{} suggestions in {} attempt(s), rejected {}",
                accepted.len(), TARGET, attempts, rejected_names.len()
            );
        }

        // ── 6. Cache ──
        let suggestions = accepted[..accepted.len().min(TARGET)].to_vec();
        if let Ok(val) = serde_json::to_value(&suggestions) {
            let _ = self
                .ai_cache
                .set(&cache_key, val, "gemini", "gemini-3-flash-preview", 1)
                .await;
        }

        Ok(SuggestProductsResponse {
            suggestions,
            query,
            cached: false,
            attempts,
        })
    }
}

// ── Prompt builder ────────────────────────────────────────────────────────────

fn build_suggest_prompt(
    query: &str,
    catalog_slugs: &HashSet<String>,        // только при attempt=1
    accepted: &[ProductSuggestion],         // уже принятые в этой сессии
    rejected: &[String],                    // отклонённые (дубли)
    need: usize,
) -> String {
    // Блок уже принятых
    let accepted_block = if accepted.is_empty() {
        String::new()
    } else {
        let names: Vec<&str> = accepted.iter().map(|a| a.name_en.as_str()).collect();
        format!(
            "\nAlready accepted (DO NOT repeat):\n- {}\n",
            names.join("\n- ")
        )
    };

    // Блок отклонённых (дубли каталога или сессии)
    let rejected_block = if rejected.is_empty() {
        String::new()
    } else {
        format!(
            "\nAlready rejected as duplicates (DO NOT suggest again):\n- {}\n",
            rejected.join("\n- ")
        )
    };

    // Каталог передаём только в первом запросе (дорого при больших каталогах)
    let catalog_block = if !catalog_slugs.is_empty() {
        // Конвертируем slugs обратно в читаемый вид для AI
        let names: Vec<String> = catalog_slugs
            .iter()
            .map(|s| s.replace('-', " "))
            .collect();
        let mut sorted = names;
        sorted.sort();
        format!(
            "\nALREADY IN CATALOG (DO NOT suggest these — they exist):\n{}\n",
            sorted.join(", ")
        )
    } else {
        String::new()
    };

    format!(
        r#"You are a food catalog expert. Suggest NEW ingredients for a food catalog.

Admin query: "{query}"
{catalog_block}{accepted_block}{rejected_block}
Generate exactly {need} unique food ingredients that:
- Match the query
- Are NOT in any of the lists above
- Are real specific ingredients (not dishes)
- Have known nutritional data

Return ONLY a valid JSON array with exactly {need} items:
[
  {{
    "name_en": "Moringa Powder",
    "name_ru": "Порошок моринги",
    "name_pl": "Proszek moringa",
    "emoji": "🌿",
    "product_type": "supplement",
    "why_add": "Суперфуд с высоким содержанием белка, витаминов и антиоксидантов",
    "calories_hint": 205
  }}
]

product_type: vegetable|fruit|grain|legume|meat|poultry|fish|seafood|dairy|egg|fat_oil|nut_seed|herb_spice|mushroom|beverage|sweetener|condiment|bakery|supplement

Return ONLY the JSON array. No markdown, no explanations."#,
        query = query,
        catalog_block = catalog_block,
        accepted_block = accepted_block,
        rejected_block = rejected_block,
        need = need,
    )
}

// ── JSON parser ───────────────────────────────────────────────────────────────

fn try_parse_suggestions(raw: &str) -> Result<Vec<ProductSuggestion>, String> {
    let cleaned = {
        let t = raw.trim();
        let s = if t.starts_with("```json") {
            &t[7..]
        } else if t.starts_with("```") {
            &t[3..]
        } else {
            t
        };
        let s = if s.trim_end().ends_with("```") {
            let trimmed = s.trim_end();
            &trimmed[..trimmed.len() - 3]
        } else {
            s
        };
        s.trim().to_string()
    };

    // 1) Прямой парсинг
    if let Ok(v) = serde_json::from_str::<Vec<ProductSuggestion>>(&cleaned) {
        return Ok(v);
    }

    // 2) Найти [...] блок
    if let Some(start) = cleaned.find('[') {
        if let Some(end) = cleaned.rfind(']') {
            if let Ok(v) = serde_json::from_str::<Vec<ProductSuggestion>>(&cleaned[start..=end]) {
                return Ok(v);
            }
        }

        // 3) Обрезанный JSON — закрыть массив после последнего }
        let body = &cleaned[start..];
        if let Some(last) = body.rfind('}') {
            let partial = format!("{}]", &body[..=last]);
            if let Ok(v) = serde_json::from_str::<Vec<ProductSuggestion>>(&partial) {
                if !v.is_empty() {
                    tracing::warn!("⚠️ JSON truncated, recovered {} items", v.len());
                    return Ok(v);
                }
            }
        }
    }

    Err(format!(
        "Cannot parse JSON from {} chars: {}...",
        raw.len(),
        &raw[..raw.len().min(200)]
    ))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sha256_short(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    let r = h.finalize();
    format!("{:x}", r)[..16].to_string()
}
