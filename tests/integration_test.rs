use restaurant_backend::application::{AdminAuthService, AdminCatalogService, AssistantService, AuthService, CatalogService, DishService, InventoryService, InventoryAlertService, MenuEngineeringService, RecipeService, TenantIngredientService, UserService};
use restaurant_backend::infrastructure::{Config, GroqService, JwtService, PasswordHasher, R2Client, Repositories};
use restaurant_backend::interfaces::http::routes::create_router;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use std::sync::Arc;

#[tokio::test]
async fn test_application_startup_integrity() {
    // 1. Load environment
    dotenvy::dotenv().ok();
    
    // Skip test if DATABASE_URL is not set (e.g. in CI without DB)
    let db_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };

    // 2. Load configuration
    let config = Config::from_env().expect("Failed to load config");

    // 3. Create pool
    let pool = PgPoolOptions::new()
        .max_connections(1) // Minimal for test
        .acquire_timeout(Duration::from_secs(1))
        .connect(&db_url)
        .await;

    // Skip if DB is not reachable
    let pool = match pool {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Skipping startup test: DB not reachable: {}", e);
            return;
        }
    };

    // 4. Initialize Repositories
    let repositories = Repositories::new(pool.clone());

    // 5. Initialize Security
    let password_hasher = PasswordHasher::new();
    let jwt_service = JwtService::new(
        config.jwt.secret.clone(),
        config.jwt.issuer.clone(),
        config.jwt.access_token_ttl_minutes,
        config.jwt.refresh_token_ttl_days,
    );

    // 6. Initialize Services
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
    let inventory_alert_service = InventoryAlertService::new(repositories.pool.clone());
    let catalog_service = CatalogService::new(repositories.pool.clone());
    let recipe_service = RecipeService::new(
        Arc::new(repositories.recipe.clone()),
        Arc::new(repositories.inventory_product.clone()),
        Arc::new(repositories.catalog_ingredient.clone()),
    );
    let dish_service = DishService::new(
        Arc::new(repositories.dish.clone()),
        recipe_service.clone(),
    );
    let menu_engineering_service = MenuEngineeringService::new(
        repositories.pool.clone(),
        Arc::new(repositories.inventory_product.clone()),
        Arc::new(repositories.dish.clone()),
        Arc::new(repositories.recipe.clone()),
    );
    let assistant_service = AssistantService::new(
        repositories.assistant_state.clone(),
        repositories.user.clone(),
        inventory_service.clone(),
        dish_service.clone(),
    );
    let admin_auth_service = AdminAuthService::new(
        config.admin.email.clone(),
        config.admin.password_hash.clone(),
        config.admin.jwt_secret.clone(),
        config.admin.token_ttl_hours,
    );
    
    // Mock R2 and Groq or use real config
    let r2_client = R2Client::new(
        config.r2.account_id.clone(),
        config.r2.access_key_id.clone(),
        config.r2.secret_access_key.clone(),
        config.r2.bucket_name.clone(),
        config.r2.public_url_base.clone(),
    ).await;

    let groq_service = Arc::new(GroqService::new(config.ai.groq_api_key.clone()));
    
    let admin_catalog_service = AdminCatalogService::new(
        repositories.pool.clone(),
        r2_client,
        repositories.dictionary.clone(),
        (*groq_service).clone(),
    );

    let tenant_ingredient_service = TenantIngredientService::new(
        Arc::new(repositories.tenant_ingredient.clone())
    );

    let recipe_translation_service = Arc::new(restaurant_backend::application::recipe_translation_service::RecipeTranslationService::new(
        Arc::new(repositories.recipe_translation.clone()),
        Arc::new(repositories.recipe_v2.clone()),
        groq_service.clone(),
    ));

    let recipe_v2_service = Arc::new(restaurant_backend::application::recipe_v2_service::RecipeV2Service::new(
        Arc::new(repositories.recipe_v2.clone()),
        Arc::new(repositories.recipe_ingredient.clone()),
        Arc::new(repositories.catalog_ingredient.clone()),
        recipe_translation_service,
    ));

    let recipe_ai_insights_service = Arc::new(restaurant_backend::application::RecipeAIInsightsService::new(
        groq_service,
        Arc::new(repositories.recipe_ai_insights.clone()),
        Arc::new(repositories.recipe_v2.clone()),
    ));

    // 7. Create Router
    let _app = create_router(
        auth_service,
        user_service,
        assistant_service,
        catalog_service,
        recipe_service,
        recipe_v2_service,
        recipe_ai_insights_service,
        dish_service,
        menu_engineering_service,
        inventory_service,
        inventory_alert_service,
        tenant_ingredient_service,
        jwt_service,
        repositories.pool.clone(),
        admin_auth_service,
        admin_catalog_service,
        config.cors.allowed_origins.clone(),
    );

    // If we reached here, the wiring is correct!
    println!("✅ Application startup integrity verified - all services and routes initialized correctly.");
}
