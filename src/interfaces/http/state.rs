use crate::application::{AuthService, BusinessService};
use crate::infrastructure::{AppCache, Config, JwtService};
use sqlx::PgPool;
use std::sync::Arc;

/// Shared, cheaply clonable application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub cache: AppCache,
    pub jwt: JwtService,
    pub auth: AuthService,
    pub business: BusinessService,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Self {
        let jwt = JwtService::new(
            &config.jwt.secret,
            config.jwt.issuer.clone(),
            config.jwt.audience.clone(),
            config.jwt.access_token_ttl_minutes,
            config.jwt.refresh_token_ttl_days,
        );
        Self {
            auth: AuthService::new(pool.clone(), jwt.clone()),
            business: BusinessService::new(pool.clone()),
            jwt,
            config: Arc::new(config),
            pool,
            cache: AppCache::default_production(),
        }
    }
}
