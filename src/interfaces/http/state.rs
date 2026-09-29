use crate::infrastructure::{AppCache, Config};
use sqlx::PgPool;
use std::sync::Arc;

/// Shared, cheaply clonable application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub cache: AppCache,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Self {
        Self {
            config: Arc::new(config),
            pool,
            cache: AppCache::default_production(),
        }
    }
}
