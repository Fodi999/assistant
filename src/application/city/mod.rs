/// src/application/city/mod.rs
///
/// City application layer — split into focused modules:
///
///   economy_snapshot    — loads live restaurant data from DB
///   city_generation     — pure deterministic city builder (no DB, no async)
///
/// Public surface: CityEngineService (facade over both)

pub mod economy_snapshot;
pub mod city_generation;

pub use economy_snapshot::EconomySnapshot;
pub use city_generation::CityGenerator;

use crate::domain::city::CityMap;
use crate::shared::{AppResult, TenantId, UserId};
use sqlx::PgPool;

// ─────────────────────────────────────────────────────────────────────────────
// Facade service — used by the HTTP handler
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CityEngineService {
    pool: PgPool,
}

impl CityEngineService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Generate the full CityMap for the authenticated tenant.
    /// 1. Load economy snapshot from DB  
    /// 2. Run pure city generator (no DB, deterministic)
    pub async fn generate_map(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<CityMap> {
        let econ = EconomySnapshot::load(&self.pool, user_id, tenant_id).await?;
        let map = CityGenerator::build(&econ, tenant_id);
        Ok(map)
    }
}
