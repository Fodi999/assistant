use crate::application::{
    AdminAuthService, AdminCatalogService, AssistantService, AuthService, CatalogService, 
    DishService, InventoryService, MenuEngineeringService, RecipeService, TenantIngredientService, UserService,
    recipe_v2_service::RecipeV2Service,  // V2 with translations
    RecipeAIInsightsService,  // 🆕 AI Insights service
};
use crate::infrastructure::JwtService;
use crate::interfaces::http::{
    admin_auth,
    admin_catalog,
    admin_users,
    assistant::{get_state, send_command},
    auth::{login_handler, refresh_handler, register_handler},
    catalog::{get_categories, search_ingredients, CatalogState},
    dish::create_dish,
    inventory::{add_product, delete_product, get_health, list_products, update_product, get_alerts, process_expirations, get_loss_report, get_dashboard},
    menu_engineering::{analyze_menu, record_sale},
    middleware::AuthUser,
    recipe::{create_recipe, get_recipe, list_recipes, delete_recipe, calculate_recipe_cost},
    recipe_v2,  // V2 handlers with translations
    recipe_ai_insights,  // AI insights handlers
    tenant_ingredient,
    user::{me_handler, get_avatar_upload_url, update_avatar_url},
};
use axum::{
    extract::{FromRequestParts, Request},
    http::{Method, header},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Router,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

pub fn create_router(
    auth_service: AuthService,
    user_service: UserService,
    assistant_service: AssistantService,
    catalog_service: CatalogService,
    recipe_service: RecipeService,
    recipe_v2_service: std::sync::Arc<RecipeV2Service>,  // 🆕 V2 with translations
    recipe_ai_insights_service: std::sync::Arc<RecipeAIInsightsService>,  // 🆕 AI Insights
    dish_service: DishService,
    menu_engineering_service: MenuEngineeringService,
    inventory_service: InventoryService,
    tenant_ingredient_service: TenantIngredientService,
    jwt_service: JwtService,
    pool: PgPool,  // 🎯 ДОБАВЛЕНО: для получения language из БД
    admin_auth_service: AdminAuthService,  // 🆕 Super Admin auth service
    admin_catalog_service: AdminCatalogService,  // 🆕 Admin Catalog service
    allowed_origins: Vec<String>,
) -> Router {
    // Configure CORS
    let cors = if allowed_origins.iter().any(|o| o == "*") {
        // Use permissive CORS for wildcard
        CorsLayer::permissive()
    } else {
        // Use specific origins
        CorsLayer::new()
            .allow_origin(
                allowed_origins
                    .iter()
                    .filter_map(|origin| origin.parse().ok())
                    .collect::<Vec<_>>(),
            )
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
            .allow_credentials(true)
    };

    // Auth routes (public)
    let auth_routes = Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
        .with_state(auth_service);

    // Admin auth routes
    let admin_service_for_middleware = admin_auth_service.clone();
    let admin_middleware = middleware::from_fn(move |req: Request, next: Next| {
        let admin_auth_service = admin_service_for_middleware.clone();
        async move { inject_admin_service(req, next, admin_auth_service).await }
    });

    let admin_routes = Router::new()
        .route("/login", post(admin_auth::login))
        .route("/verify", get(admin_auth::verify))
        .with_state(admin_auth_service.clone())
        .layer(admin_middleware.clone());

    // Admin catalog routes (protected with admin JWT)
    use crate::interfaces::http::middleware::require_super_admin;
    
    // Create middleware to inject AdminAuthService
    let admin_auth_for_catalog = admin_auth_service.clone();
    let admin_catalog_middleware = middleware::from_fn(move |mut req: Request, next: Next| {
        let admin_auth_service = admin_auth_for_catalog.clone();
        async move {
            req.extensions_mut().insert(admin_auth_service);
            next.run(req).await
        }
    });
    
    let admin_catalog_routes = Router::new()
        // Products
        .route("/products", get(admin_catalog::list_products))
        .route("/products/:id", get(admin_catalog::get_product))
        .route("/products", post(admin_catalog::create_product))
        .route("/products/:id", axum::routing::put(admin_catalog::update_product).patch(admin_catalog::update_product))
        .route("/products/:id", axum::routing::delete(admin_catalog::delete_product))
        .route("/products/:id/image", post(admin_catalog::upload_product_image))
        .route("/products/:id/image-url", get(admin_catalog::get_image_upload_url))
        .route("/products/:id/image", axum::routing::put(admin_catalog::save_image_url))
        .route("/products/:id/image", axum::routing::delete(admin_catalog::delete_product_image))
        // Categories
        .route("/categories", get(admin_catalog::list_categories))
        .route("/categories", post(admin_catalog::create_category))
        .route("/categories/:id", axum::routing::put(admin_catalog::update_category))
        .route("/categories/:id", axum::routing::delete(admin_catalog::delete_category))
        .layer(middleware::from_fn_with_state(admin_auth_service.clone(), require_super_admin))
        .layer(admin_catalog_middleware)
        .with_state(admin_catalog_service);
    
    // Admin users route (for user management)
    let admin_users_route: Router = Router::new()
        .route("/users", get(admin_users::list_users))
        .route("/users/:id", delete(admin_users::delete_user))
        .route("/stats", get(admin_users::get_stats))
        .layer(middleware::from_fn_with_state(admin_auth_service.clone(), require_super_admin))
        .with_state(pool.clone());

    // Protected routes
    let jwt_middleware = middleware::from_fn(move |req: Request, next: Next| {
        let jwt_service = jwt_service.clone();
        let pool = pool.clone();
        async move { inject_jwt_and_pool(req, next, jwt_service, pool).await }
    });

    let protected_routes = Router::new()
        .route("/me", get(me_handler))
        .route("/profile/avatar/upload-url", post(get_avatar_upload_url))
        .route("/profile/avatar", axum::routing::put(update_avatar_url))
        .with_state(user_service.clone())
        .merge(
            Router::new()
                .route("/assistant/state", get(get_state))
                .route("/assistant/command", post(send_command))
                .with_state(assistant_service)
        )
        .merge(
            Router::new()
                .route("/catalog/categories", get(get_categories))
                .route("/catalog/ingredients", get(search_ingredients))
                .with_state(CatalogState {
                    catalog_service,
                    user_service,
                })
        )
        .merge(
            Router::new()
                .route("/recipes", post(create_recipe))
                .route("/recipes", get(list_recipes))
                .route("/recipes/:id", get(get_recipe))
                .route("/recipes/:id", axum::routing::delete(delete_recipe))
                .route("/recipes/:id/cost", get(calculate_recipe_cost))
                .with_state(recipe_service)
        )
        .merge(
            Router::new()
                .route("/recipes/v2", post(recipe_v2::create_recipe))
                .route("/recipes/v2", get(recipe_v2::list_recipes))
                .route("/recipes/v2/:id", get(recipe_v2::get_recipe))
                .route("/recipes/v2/:id/publish", post(recipe_v2::publish_recipe))
                .route("/recipes/v2/:id", axum::routing::delete(recipe_v2::delete_recipe))
                .with_state(recipe_v2_service)
        )
        .merge(
            Router::new()
                // AI Insights endpoints
                .route("/recipes/v2/:id/insights", get(recipe_ai_insights::get_all_insights))
                .route("/recipes/v2/:id/insights/:language", get(recipe_ai_insights::get_or_generate_insights))
                .route("/recipes/v2/:id/insights/:language/refresh", post(recipe_ai_insights::refresh_insights))
                .with_state(recipe_ai_insights_service)
        )
        .merge(
            Router::new()
                .route("/dishes", post(create_dish))
                .with_state(dish_service)
        )
        .merge(
            Router::new()
                .route("/inventory/products", get(list_products))
                .route("/inventory/products", post(add_product))
                .route("/inventory/products/:id", axum::routing::put(update_product))
                .route("/inventory/products/:id", axum::routing::delete(delete_product))
                .route("/inventory/process-expirations", post(process_expirations))
                .route("/inventory/reports/loss", get(get_loss_report))
                .route("/inventory/dashboard", get(get_dashboard)) // New ownership dashboard
                .route("/inventory/alerts", get(get_alerts))
                .route("/inventory/health", get(get_health))
                .with_state(inventory_service)
        )
        // Removed separate inventory_alert_service merge
        .merge(
            Router::new()
                .route("/menu-engineering/analysis", get(analyze_menu))
                .route("/menu-engineering/sales", post(record_sale))
                .with_state(menu_engineering_service)
        )
        .merge(
            Router::new()
                .route("/tenant/ingredients", post(tenant_ingredient::add_ingredient))
                .route("/tenant/ingredients", get(tenant_ingredient::list_ingredients))
                .route("/tenant/ingredients/search", get(tenant_ingredient::search_available_ingredients))
                .route("/tenant/ingredients/:id", get(tenant_ingredient::get_ingredient))
                .route("/tenant/ingredients/:id", axum::routing::put(tenant_ingredient::update_ingredient))
                .route("/tenant/ingredients/:id", axum::routing::delete(tenant_ingredient::remove_ingredient))
                .with_state(tenant_ingredient_service)
        )
        .layer(jwt_middleware);

    // Health check endpoint (no auth, no middleware)
    let health_route = Router::new()
        .route("/health", get(|| async { "OK" }));

    // Combine all routes
    Router::new()
        .merge(health_route)
        .nest("/api/auth", auth_routes)
        .nest("/api/admin/auth", admin_routes)
        .nest("/api/admin/catalog", admin_catalog_routes)
        .nest("/api/admin", admin_users_route)
        .nest("/api", protected_routes)
        .layer(cors)
}

async fn inject_jwt_and_pool(
    mut req: Request,
    next: Next,
    jwt_service: JwtService,
    pool: PgPool,
) -> Response {
    req.extensions_mut().insert(jwt_service);
    req.extensions_mut().insert(pool);  // 🎯 ДОБАВЛЕНО: pool для AuthUser
    
    // Попытка извлечь AuthUser через экстрактор
    // Если успешно - добавляем в extensions
    let mut parts = req.into_parts();
    if let Ok(auth_user) = AuthUser::from_request_parts(&mut parts.0, &()).await {
        parts.0.extensions.insert(auth_user);
    }
    let req = Request::from_parts(parts.0, parts.1);
    
    next.run(req).await
}

async fn inject_admin_service(
    mut req: Request,
    next: Next,
    admin_auth_service: AdminAuthService,
) -> Response {
    req.extensions_mut().insert(admin_auth_service);
    next.run(req).await
}
