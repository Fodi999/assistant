use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sqlx::PgPool;

use crate::shared::{AppError, Language};

use super::matcher;
use super::parser;
use super::response::{IngredientShort, MatchType, ParseMeta, SmartParseResponse};

const MAX_TOKENS: usize = 15;
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes
const MAX_CACHE_ENTRIES: usize = 2000;

// ── In-memory cache ──────────────────────────────────────────────────────────

struct CacheEntry {
    response: SmartParseResponse,
    created: Instant,
}

/// SmartParse service — pool + in-memory response cache.
#[derive(Clone)]
pub struct SmartParseService {
    pool: PgPool,
    cache: std::sync::Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl SmartParseService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: std::sync::Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Cache key = "text_lowercase|lang_code"
    fn cache_key(text: &str, lang: Language) -> String {
        format!("{}|{}", text.to_lowercase().trim(), lang.code())
    }

    fn get_cached(&self, key: &str) -> Option<SmartParseResponse> {
        let guard = self.cache.lock().ok()?;
        let entry = guard.get(key)?;
        if entry.created.elapsed() < CACHE_TTL {
            Some(entry.response.clone())
        } else {
            None
        }
    }

    fn put_cache(&self, key: String, response: SmartParseResponse) {
        if let Ok(mut guard) = self.cache.lock() {
            // Evict expired entries if cache is getting large
            if guard.len() >= MAX_CACHE_ENTRIES {
                guard.retain(|_, v| v.created.elapsed() < CACHE_TTL);
            }
            // If still too large after eviction, just clear (rare edge case)
            if guard.len() >= MAX_CACHE_ENTRIES {
                guard.clear();
            }
            guard.insert(key, CacheEntry {
                response,
                created: Instant::now(),
            });
        }
    }

    // ── Main API ─────────────────────────────────────────────────────────────

    /// Main entry point: text → SmartParseResponse.
    ///
    /// Pipeline: cache check → tokenize → normalize → batch-match → dedup → cache store.
    /// Total DB round-trips: 2 (dictionary UNION + batch LATERAL JOIN), or 0 if cached.
    pub async fn parse(
        &self,
        text: &str,
        lang: Language,
    ) -> Result<SmartParseResponse, AppError> {
        let t0 = Instant::now();

        // 0. Cache check
        let key = Self::cache_key(text, lang);
        if let Some(mut cached) = self.get_cached(&key) {
            cached.meta.cache = true;
            cached.meta.timing_ms = t0.elapsed().as_millis();
            return Ok(cached);
        }

        // 1. Tokenize (stop-words removed, numbers stripped, multi-word merged)
        let tokens = parser::tokenize(text, MAX_TOKENS);
        let total_tokens = tokens.len();

        if total_tokens == 0 {
            let resp = SmartParseResponse {
                found: vec![],
                unknown: vec![],
                meta: ParseMeta {
                    tokens: 0,
                    matched: 0,
                    unmatched: 0,
                    timing_ms: t0.elapsed().as_millis(),
                    cache: false,
                },
            };
            return Ok(resp);
        }

        // 2. Normalize: dictionary lookup (local name → EN slug)
        let dict = self.load_dictionary(lang).await.unwrap_or_default();

        let mut token_candidates: Vec<Vec<String>> = Vec::with_capacity(tokens.len());
        let mut all_candidates: Vec<String> = Vec::new();
        let mut candidate_set: HashSet<String> = HashSet::new();

        for token in &tokens {
            let mut cands = Vec::new();

            // Dictionary: local_name → slug
            if let Some(en_name) = dict.get(&token.to_lowercase()) {
                let slug = slugify(en_name);
                if slug != *token {
                    cands.push(slug.clone());
                    if candidate_set.insert(slug.clone()) {
                        all_candidates.push(slug);
                    }
                }
            }

            cands.push(token.clone());
            if candidate_set.insert(token.clone()) {
                all_candidates.push(token.clone());
            }

            token_candidates.push(cands);
        }

        // 3. Batch-match ALL candidates in ONE DB round-trip
        let match_map = matcher::batch_match(&self.pool, &all_candidates, lang).await?;

        // 4. Resolve per-token, dedup by slug, build response with confidence
        let mut found: Vec<IngredientShort> = Vec::new();
        let mut unknown: Vec<String> = Vec::new();
        let mut seen_slugs: HashSet<String> = HashSet::new();

        for (i, token) in tokens.iter().enumerate() {
            let mut matched = false;
            for cand in &token_candidates[i] {
                if let Some(row) = match_map.get(cand) {
                    if seen_slugs.insert(row.slug.clone()) {
                        let mt = match row.match_type() {
                            "exact" => MatchType::Exact,
                            "name"  => MatchType::Name,
                            "ilike" => MatchType::Ilike,
                            _       => MatchType::Fuzzy,
                        };
                        found.push(IngredientShort {
                            slug: row.slug.clone(),
                            name: row.name.clone(),
                            confidence: row.confidence(),
                            match_type: mt,
                        });
                    }
                    matched = true;
                    break;
                }
            }
            if !matched {
                unknown.push(token.clone());
            }
        }

        let timing_ms = t0.elapsed().as_millis();
        let matched_count = found.len();
        let unmatched_count = unknown.len();

        let resp = SmartParseResponse {
            found,
            unknown,
            meta: ParseMeta {
                tokens: total_tokens,
                matched: matched_count,
                unmatched: unmatched_count,
                timing_ms,
                cache: false,
            },
        };

        // 5. Store in cache
        self.put_cache(key, resp.clone());

        Ok(resp)
    }

    /// Load dictionary mapping: lowercase local_name → english_slug.
    /// Single DB query (UNION of ingredient_dictionary + catalog_ingredients).
    async fn load_dictionary(
        &self,
        lang: Language,
    ) -> Result<HashMap<String, String>, AppError> {
        let name_col = match lang {
            Language::En => return Ok(HashMap::new()),
            Language::Ru => "name_ru",
            Language::Pl => "name_pl",
            Language::Uk => "name_uk",
        };

        let sql = format!(
            r#"
            SELECT LOWER(TRIM({col})) AS local_name, TRIM(name_en) AS en_name
            FROM ingredient_dictionary WHERE status = 'active'
            UNION ALL
            SELECT LOWER(TRIM({col})) AS local_name, slug AS en_name
            FROM catalog_ingredients WHERE COALESCE(is_active, true) = true
            "#,
            col = name_col,
        );

        #[derive(sqlx::FromRow)]
        struct DictRow {
            local_name: String,
            en_name: String,
        }

        let rows: Vec<DictRow> = sqlx::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let mut map = HashMap::with_capacity(rows.len());
        for r in rows {
            map.entry(r.local_name).or_insert(r.en_name);
        }
        Ok(map)
    }
}

/// Convert "Pasteurized Milk" → "pasteurized-milk".
fn slugify(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Olive Oil"), "olive-oil");
        assert_eq!(slugify("  Salmon  "), "salmon");
    }
}
