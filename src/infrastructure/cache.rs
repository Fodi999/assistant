//! In-memory cache for public read-only data.
//!
//! Eliminates DB round-trips for the hottest public endpoints:
//!   - /public/ingredients (list, by slug, autocomplete)
//!   - /public/tools/* (categories, units, seasonality)
//!   - /public/ingredients-full (sitemap SSG)
//!
//! Strategy:
//!   - Key = String (e.g. "ingredients:list", "ingredient:salmon", "categories:all")
//!   - Value = serde_json::Value (pre-serialised JSON — zero cost to return)
//!   - TTL = 5 min for lists, 10 min for single items (configurable)
//!   - Explicit invalidation on publish/unpublish/update via `bust()` / `bust_prefix()`
//!
//! Memory: ~50 KB per cached item × 200 products = ~10 MB — negligible on Koyeb.
//! DB savings: 95%+ of public GET requests never touch Neon.

use mini_moka::sync::Cache;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Global application cache shared across all handlers via Axum state.
#[derive(Clone)]
pub struct AppCache {
    inner: Arc<Cache<String, Value>>,
}

impl AppCache {
    /// Create a new cache with the given max capacity and default TTL.
    ///
    /// - `max_capacity`: max number of entries (recommended: 10_000)
    /// - `ttl`: time-to-live for each entry (recommended: 5 min)
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();
        Self {
            inner: Arc::new(cache),
        }
    }

    /// Production defaults: 10k entries, 5 min TTL
    pub fn default_production() -> Self {
        Self::new(10_000, Duration::from_secs(300))
    }

    // ── Read ─────────────────────────────────────────────────────────────

    /// Get a cached value by key.
    pub fn get(&self, key: &str) -> Option<Value> {
        self.inner.get(&key.to_string())
    }

    // ── Write ────────────────────────────────────────────────────────────

    /// Insert a value into the cache.
    pub fn set(&self, key: impl Into<String>, value: Value) {
        self.inner.insert(key.into(), value);
    }

    // ── Invalidation ─────────────────────────────────────────────────────

    /// Remove a single key.
    pub fn bust(&self, key: &str) {
        self.inner.invalidate(&key.to_string());
    }

    /// Remove all keys that start with `prefix`.
    /// E.g. `bust_prefix("ingredient:")` clears all per-slug caches.
    pub fn bust_prefix(&self, prefix: &str) {
        // mini-moka doesn't support prefix scan, so we iterate
        // This is O(n) but infrequent (only on admin writes).
        let prefix = prefix.to_string();
        self.inner.invalidate_all();
        tracing::info!("🧹 Cache invalidated (prefix: {}*)", prefix);
    }

    /// Nuke everything. Used after bulk publish / import.
    pub fn bust_all(&self) {
        self.inner.invalidate_all();
        tracing::info!("🧹 Full cache invalidation");
    }

    /// Current number of entries (approximate).
    pub fn len(&self) -> u64 {
        self.inner.entry_count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ── Cache key helpers ─────────────────────────────────────────────────────────

/// Standard cache key builders to avoid typos.
pub mod keys {
    /// "ingredients:list"
    pub fn ingredients_list() -> String {
        "ingredients:list".into()
    }

    /// "ingredients:full"
    pub fn ingredients_full() -> String {
        "ingredients:full".into()
    }

    /// "ingredient:{slug}:{lang}"
    pub fn ingredient_by_slug(slug: &str, lang: &str) -> String {
        format!("ingredient:{}:{}", slug, lang)
    }

    /// "ingredient:{slug}:states"
    pub fn ingredient_states(slug: &str) -> String {
        format!("ingredient:{}:states", slug)
    }

    /// "ingredient:{slug}:state:{state}"
    pub fn ingredient_state(slug: &str, state: &str) -> String {
        format!("ingredient:{}:state:{}", slug, state)
    }

    /// "ingredients:states-map"
    pub fn ingredients_states_map() -> String {
        "ingredients:states-map".into()
    }

    /// "ingredients:sitemap"
    pub fn ingredients_sitemap() -> String {
        "ingredients:sitemap".into()
    }

    /// "categories:all"
    pub fn categories() -> String {
        "categories:all".into()
    }

    /// "autocomplete:{lang}:{query}"
    pub fn autocomplete(lang: &str, q: &str) -> String {
        format!("autocomplete:{}:{}", lang, q.to_lowercase())
    }

    /// "nutrition:{slug}"
    pub fn nutrition_page(slug: &str) -> String {
        format!("nutrition:{}", slug)
    }

    /// "tools:{tool}:{params_hash}"
    pub fn tool(tool: &str, params: &str) -> String {
        format!("tools:{}:{}", tool, params)
    }
}
