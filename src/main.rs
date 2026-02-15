mod application;
mod domain;
mod infrastructure;
mod interfaces;
mod shared;

use application::{AdminAuthService, AdminCatalogService, AssistantService, AuthService, CatalogService, DishService, InventoryService, MenuEngineeringService, RecipeService, UserService};
use infrastructure::{Config, GroqService, JwtService, PasswordHasher, R2Client, Repositories};
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

    // Create R2Client for image storage (Cloudflare R2)
    tracing::info!("Initializing R2 Client with bucket: {}", config.r2.bucket_name);
    let r2_client = R2Client::new(
        config.r2.account_id.clone(),
        config.r2.access_key_id.clone(),
        config.r2.secret_access_key.clone(),
        config.r2.bucket_name.clone(),
        config.r2.public_url_base.clone(),
    ).await;
    tracing::info!("✅ R2 Client initialized successfully");

    // Create GroqService for AI translations (for AdminCatalogService)
    let groq_service = GroqService::new(config.ai.groq_api_key.clone());
    if config.ai.groq_api_key.is_empty() {
        tracing::warn!("⚠️ GROQ_API_KEY not set - auto-translation will not work");
    } else {
        tracing::info!("✅ Groq Service initialized for auto-translations");
    }

    // Create AdminCatalogService with Groq + Dictionary support
    let admin_catalog_service = AdminCatalogService::new(
        repositories.pool.clone(),
        r2_client,
        repositories.dictionary.clone(),
        groq_service,
    );
    tracing::info!("Admin Catalog Service initialized (with hybrid translation cache)");

    // Create TenantIngredientService
    let tenant_ingredient_service = crate::application::TenantIngredientService::new(
        repositories.pool.clone()
    );
    tracing::info!("Tenant Ingredient Service initialized");

    // Create Recipe V2 Services (with AI translations)
    // Create separate GroqService instance for Recipe V2
    let groq_service_v2 = Arc::new(GroqService::new(config.ai.groq_api_key.clone()));
    
    let recipe_v2_repo = Arc::new(crate::infrastructure::persistence::RecipeRepositoryV2::new(
        repositories.pool.clone()
    ));
    let recipe_ingredient_repo = Arc::new(crate::infrastructure::persistence::RecipeIngredientRepository::new(
        repositories.pool.clone()
    ));
    let recipe_translation_repo = Arc::new(crate::infrastructure::persistence::RecipeTranslationRepository::new(
        repositories.pool.clone()
    ));
    
    let recipe_translation_service = Arc::new(crate::application::recipe_translation_service::RecipeTranslationService::new(
        recipe_translation_repo,
        recipe_v2_repo.clone(),
        groq_service_v2,
    ));
    
    let recipe_v2_service = Arc::new(crate::application::recipe_v2_service::RecipeV2Service::new(
        recipe_v2_repo,
        recipe_ingredient_repo,
        Arc::new(repositories.catalog_ingredient.clone()),
        recipe_translation_service,
    ));
    tracing::info!("✅ Recipe V2 Service initialized (with auto-translations)");

    // Clone CORS origins before moving config
    let cors_origins = config.cors.allowed_origins.clone();

    // Create router
    let app = create_router(
        auth_service,
        user_service,
        assistant_service,
        catalog_service,
        recipe_service,
        recipe_v2_service,            // 🆕 V2 with auto-translations
        dish_service,
        menu_engineering_service,
        inventory_service,
        tenant_ingredient_service,  // 🆕 Tenant-specific ingredients
        jwt_service,
        repositories.pool.clone(),  // 🎯 pool для AuthUser middleware
        admin_auth_service,         // 🆕 Super Admin auth
        admin_catalog_service,      // 🆕 Admin Catalog service
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
