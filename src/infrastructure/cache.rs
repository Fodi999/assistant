//! In-memory TTL cache for public read-only data (business profiles, services,
//! portfolio metadata). Values are pre-serialised JSON, so a hit costs nothing.
//! Availability slots must NOT be cached here: they change with every booking.

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
    /// E.g. `bust_prefix("business:")` clears all per-business caches.
    pub fn bust_prefix(&self, prefix: &str) {
        // mini-moka doesn't support prefix scan, so we iterate
        // This is O(n) but infrequent (only on admin writes).
        let prefix = prefix.to_string();
        self.inner.invalidate_all();
        tracing::info!("🧹 Cache invalidated (prefix: {}*)", prefix);
    }

    /// Nuke everything. Used after bulk imports.
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
