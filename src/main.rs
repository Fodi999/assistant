mod application;
mod domain;
mod infrastructure;
mod interfaces;
mod shared;

use application::{AssistantService, AuthService, CatalogService, InventoryService, UserService};
use infrastructure::{Config, JwtService, PasswordHasher, Repositories};
use interfaces::http::routes::create_router;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    tracing::info!("Starting Restaurant Backend...");

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database.url)
        .await?;
    tracing::info!("Database connection pool established");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;
    tracing::info!("Database migrations completed");

    // Initialize repositories
    let repositories = Repositories::new(pool);

    // Initialize security services
    let password_hasher = PasswordHasher::new();
    let jwt_service = JwtService::new(
        config.jwt.secret.clone(),
        config.jwt.issuer.clone(),
        config.jwt.access_token_ttl_minutes,
        config.jwt.refresh_token_ttl_days,
    );

    // Initialize application services
    let auth_service = AuthService::new(
        repositories.user.clone(),
        repositories.tenant.clone(),
        repositories.refresh_token.clone(),
        password_hasher,
        jwt_service.clone(),
    );

    let user_service = UserService::new(
        repositories.user.clone(),
        repositories.tenant.clone(),
    );

    let inventory_service = InventoryService::new(repositories.pool.clone());

    let assistant_service = AssistantService::new(
        repositories.assistant_state.clone(),
        repositories.user.clone(),
        inventory_service,
    );

    let catalog_service = CatalogService::new(repositories.pool.clone());

    // Clone CORS origins before moving config
    let cors_origins = config.cors.allowed_origins.clone();

    // Create router
    let app = create_router(
        auth_service,
        user_service,
        assistant_service,
        catalog_service,
        jwt_service,
        cors_origins,
    );

    // Start server
    let addr = config.server_address();
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    axum::serve(listener, app)
        .await?;

    Ok(())
}
