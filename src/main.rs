mod application;
mod domain;
mod infrastructure;
mod interfaces;
mod shared;

use application::{AdminAuthService, AssistantService, AuthService, CatalogService, DishService, InventoryService, MenuEngineeringService, RecipeService, UserService};
use infrastructure::{Config, JwtService, PasswordHasher, Repositories};
use interfaces::http::routes::create_router;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing BEFORE anything else
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    tracing::info!("Starting Restaurant Backend...");
    tracing::info!("Environment: DATABASE_URL present = {}", std::env::var("DATABASE_URL").is_ok());
    tracing::info!("Environment: JWT_SECRET present = {}", std::env::var("JWT_SECRET").is_ok());
    tracing::info!("Environment: PORT = {}", std::env::var("PORT").unwrap_or_else(|_| "not set".to_string()));

    // Load configuration
    let config = match Config::from_env() {
        Ok(c) => {
            tracing::info!("Configuration loaded successfully");
            c
        }
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            return Err(e);
        }
    };

    tracing::info!("Server will bind to: {}", config.server_address());

    // Create database connection pool
    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database.url)
        .await
        .map_err(|e| {
            tracing::error!("Database connection failed: {}", e);
            e
        })?;
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

    let catalog_service = CatalogService::new(repositories.pool.clone());

    // Create RecipeService with all dependencies
    let recipe_service = RecipeService::new(
        Arc::new(repositories.recipe.clone()),
        Arc::new(repositories.inventory_product.clone()),
        Arc::new(repositories.catalog_ingredient.clone()),
    );

    // Create DishService
    let dish_service = DishService::new(
        Arc::new(repositories.dish.clone()),
        recipe_service.clone(),
    );

    // Create MenuEngineeringService
    let menu_engineering_service = MenuEngineeringService::new(repositories.pool.clone());

    // Create AssistantService with all services
    let assistant_service = AssistantService::new(
        repositories.assistant_state.clone(),
        repositories.user.clone(),
        inventory_service.clone(),
        recipe_service.clone(),
        dish_service.clone(),
    );

    // Create AdminAuthService (Super Admin)
    let admin_auth_service = AdminAuthService::new(
        config.admin.email.clone(),
        config.admin.password_hash.clone(),
        config.admin.jwt_secret.clone(),
        config.admin.token_ttl_hours,
    );
    tracing::info!("Super Admin configured: {}", config.admin.email);

    // Clone CORS origins before moving config
    let cors_origins = config.cors.allowed_origins.clone();

    // Create router
    let app = create_router(
        auth_service,
        user_service,
        assistant_service,
        catalog_service,
        recipe_service,
        dish_service,
        menu_engineering_service,
        inventory_service,
        jwt_service,
        repositories.pool.clone(),  // 🎯 pool для AuthUser middleware
        admin_auth_service,         // 🆕 Super Admin auth
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
