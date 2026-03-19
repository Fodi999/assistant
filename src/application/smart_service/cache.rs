//! In-memory TTL cache for SmartService responses.
//!
//! Simple HashMap + timestamps, cleaned on access. No external crate needed.
//! Thread-safe via Arc<Mutex<...>> (low contention: cache is checked once per request).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::application::smart_service::response::SmartResponse;

const DEFAULT_TTL: Duration = Duration::from_secs(300); // 5 minutes
const MAX_ENTRIES: usize = 2000; // evict oldest if exceeded

struct CacheEntry {
    response: SmartResponse,
    created:  Instant,
}

pub struct SmartCache {
    inner: Arc<Mutex<HashMap<String, CacheEntry>>>,
    ttl:   Duration,
}

impl SmartCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::with_capacity(256))),
            ttl:   DEFAULT_TTL,
        }
    }

    /// Get a cached response if it exists and hasn't expired.
    pub fn get(&self, key: &str) -> Option<SmartResponse> {
        let mut map = self.inner.lock().ok()?;
        if let Some(entry) = map.get(key) {
            if entry.created.elapsed() < self.ttl {
                return Some(entry.response.clone());
            }
            // Expired — remove
            map.remove(key);
        }
        None
    }

    /// Insert a response into the cache.
    pub fn insert(&self, key: String, response: SmartResponse) {
        if let Ok(mut map) = self.inner.lock() {
            // Evict if too many entries (simple: clear oldest half)
            if map.len() >= MAX_ENTRIES {
                let mut entries: Vec<(String, Instant)> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.created))
                    .collect();
                entries.sort_by_key(|(_, t)| *t);
                let to_remove = entries.len() / 2;
                for (k, _) in entries.into_iter().take(to_remove) {
                    map.remove(&k);
                }
            }
            map.insert(key, CacheEntry { response, created: Instant::now() });
        }
    }
}
