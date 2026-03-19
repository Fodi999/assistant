//! SmartService — public API, owns cache + pool, delegates to pipeline.

use sqlx::PgPool;
use crate::shared::AppResult;

use super::cache::SmartCache;
use super::context::CulinaryContext;
use super::pipeline;
use super::response::SmartResponse;

pub struct SmartService {
    pool:  PgPool,
    cache: SmartCache,
}

impl SmartService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: SmartCache::new(),
        }
    }

    /// Main entry point: get smart ingredient analysis.
    /// Returns cached if available, otherwise runs full pipeline.
    pub async fn get_smart_ingredient(&self, ctx: CulinaryContext) -> AppResult<SmartResponse> {
        let cache_key = ctx.cache_key();

        // 1. Check cache
        if let Some(mut cached) = self.cache.get(&cache_key) {
            cached.meta.cached = true;
            cached.meta.timing_ms = 0; // instant
            return Ok(cached);
        }

        // 2. Run pipeline
        let response = pipeline::execute(&self.pool, &ctx).await?;

        // 3. Store in cache
        self.cache.insert(cache_key, response.clone());

        Ok(response)
    }
}
