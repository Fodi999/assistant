//! SmartService v3 — public API, owns cache + pool + sessions.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use sqlx::PgPool;
use crate::shared::AppResult;

use super::cache::SmartCache;
use super::context::CulinaryContext;
use super::pipeline;
use super::response::SmartResponse;

// ── Session storage ──────────────────────────────────────────────────────────

/// What we remember per session (lightweight, in-memory).
#[derive(Debug, Clone)]
pub struct SessionData {
    /// Last ingredients the user explored (most recent first)
    pub recent_ingredients: Vec<String>,
    /// Last updated timestamp (for TTL eviction)
    pub updated: Instant,
}

const MAX_SESSIONS: usize = 5000;
const SESSION_TTL_SECS: u64 = 1800; // 30 minutes

pub struct SmartService {
    pool:     PgPool,
    cache:    SmartCache,
    sessions: Mutex<HashMap<String, SessionData>>,
}

impl SmartService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: SmartCache::new(),
            sessions: Mutex::new(HashMap::with_capacity(256)),
        }
    }

    /// Main entry point: get smart ingredient analysis.
    /// Returns cached if available, otherwise runs full pipeline.
    /// v3: updates session with the ingredient.
    pub async fn get_smart_ingredient(&self, ctx: CulinaryContext) -> AppResult<SmartResponse> {
        let cache_key = ctx.cache_key();

        // 1. Check cache
        if let Some(mut cached) = self.cache.get(&cache_key) {
            cached.meta.cached = true;
            cached.meta.timing_ms = 0;
            // Update session even on cache hit
            self.update_session(&cached.session_id, &ctx.ingredient);
            return Ok(cached);
        }

        // 2. Run pipeline
        let response = pipeline::execute(&self.pool, &ctx).await?;

        // 3. Store in cache
        self.cache.insert(cache_key, response.clone());

        // 4. Update session
        self.update_session(&response.session_id, &ctx.ingredient);

        Ok(response)
    }

    /// Get recent ingredients for a session.
    pub fn get_session(&self, session_id: &str) -> Option<SessionData> {
        let map = self.sessions.lock().ok()?;
        let data = map.get(session_id)?;
        if data.updated.elapsed().as_secs() > SESSION_TTL_SECS {
            return None;
        }
        Some(data.clone())
    }

    /// Record that a session explored an ingredient.
    fn update_session(&self, session_id: &str, ingredient: &str) {
        if let Ok(mut map) = self.sessions.lock() {
            // Evict expired / overflow
            if map.len() >= MAX_SESSIONS {
                let now = Instant::now();
                map.retain(|_, v| now.duration_since(v.updated).as_secs() < SESSION_TTL_SECS);
                // If still too many, clear oldest half
                if map.len() >= MAX_SESSIONS {
                    let mut entries: Vec<(String, Instant)> = map.iter().map(|(k, v)| (k.clone(), v.updated)).collect();
                    entries.sort_by_key(|(_, t)| *t);
                    let to_remove = entries.len() / 2;
                    for (k, _) in entries.into_iter().take(to_remove) {
                        map.remove(&k);
                    }
                }
            }

            let entry = map.entry(session_id.to_string()).or_insert_with(|| SessionData {
                recent_ingredients: Vec::new(),
                updated: Instant::now(),
            });

            // Remove if already present, then push to front
            entry.recent_ingredients.retain(|s| s != ingredient);
            entry.recent_ingredients.insert(0, ingredient.to_string());
            entry.recent_ingredients.truncate(20); // keep last 20
            entry.updated = Instant::now();
        }
    }
}
