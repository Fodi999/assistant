use crate::application::{
    AdminService, AuthService, AvailabilityService, BookingService, BusinessService,
    CatalogService, ClientService, ProfileService, PublicApi, ScheduleService, TeamService,
};
use crate::infrastructure::{AppCache, Config, JwtService};
use crate::shared::Clock;
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
    pub catalog: CatalogService,
    pub schedule: ScheduleService,
    pub team: TeamService,
    pub availability: AvailabilityService,
    pub booking: BookingService,
    pub profile: ProfileService,
    pub clients: ClientService,
    pub public: PublicApi,
    pub admin: AdminService,
}

impl AppState {
    pub fn new(config: Config, pool: PgPool) -> Self {
        Self::with_clock(config, pool, Clock::system())
    }

    /// Same as [`AppState::new`] with an injected clock (tests).
    pub fn with_clock(config: Config, pool: PgPool, clock: Clock) -> Self {
        let jwt = JwtService::new(
            &config.jwt.secret,
            config.jwt.issuer.clone(),
            config.jwt.audience.clone(),
            config.jwt.access_token_ttl_minutes,
            config.jwt.refresh_token_ttl_days,
        );
        let availability = AvailabilityService::new(pool.clone(), clock.clone());
        let booking = BookingService::new(pool.clone(), clock);
        Self {
            auth: AuthService::new(pool.clone(), jwt.clone()),
            business: BusinessService::new(pool.clone()),
            catalog: CatalogService::new(pool.clone()),
            schedule: ScheduleService::new(pool.clone()),
            team: TeamService::new(pool.clone()),
            profile: ProfileService::new(pool.clone()),
            clients: ClientService::new(pool.clone()),
            admin: AdminService::new(pool.clone()),
            public: PublicApi::new(pool.clone(), booking.clone(), availability.clone()),
            availability,
            booking,
            jwt,
            config: Arc::new(config),
            pool,
            cache: AppCache::default_production(),
        }
    }
}
