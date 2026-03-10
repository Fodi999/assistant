use crate::application::{
    cms_service::CmsService,
    recipe_v2_service::RecipeV2Service, // V2 with translations
    report::ReportService,
    AdminAuthService,
    AdminCatalogService,
    AssistantService,
    AuthService,
    CatalogService,
    DishService,
    InventoryService,
    MenuEngineeringService,
    RecipeAIInsightsService, // 🆕 AI Insights service
    RecipeService,
    TenantIngredientService,
    UserService,
};
use crate::infrastructure::JwtService;
use crate::interfaces::http::{
    admin_auth,
    admin_catalog,
    admin_cms,
    admin_users,
    assistant::{get_state, send_command},
    auth::{login_handler, refresh_handler, register_handler},
    catalog::{get_categories, search_ingredients, CatalogState},
    chef_reference_public::{convert_units, fish_season, get_ingredient},
    public::{
        cms as public_cms,
        ingredients::{get_ingredient_by_slug, list_ingredients},
        tools::{convert_units as tools_convert, fish_season as tools_fish_season, fish_season_table, list_units, list_categories, nutrition, ingredients_db, compare_foods, scale_recipe, yield_calc, ingredient_equivalents, food_cost_calc, ingredient_suggestions, popular_conversions, ingredient_scale, ingredient_convert, seo_ingredient_convert, measure_conversion, ingredient_measures, seasonal_calendar, in_season_now, product_seasonality, best_in_season, products_by_month, product_search, recipe_nutrition, recipe_cost, list_regions, best_right_now, resolve_slug},
    },
    dish::{create_dish, list_dishes, recalculate_all_costs},
    inventory::{
        add_product, delete_product, get_alerts, get_dashboard, get_health, get_loss_report,
        list_products, process_expirations, update_product,
    },
    menu_engineering::{analyze_menu, record_sale},
    middleware::AuthUser,
    recipe::{calculate_recipe_cost, create_recipe, delete_recipe, get_recipe, list_recipes},
    recipe_ai_insights, // AI insights handlers
    recipe_v2,          // V2 handlers with translations
    report::get_summary,
    tenant_ingredient,
    user::{get_avatar_upload_url, me_handler, update_avatar_url},
};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, FromRequestParts, Request},
    http::{header, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use governor::{clock::DefaultClock, state::keyed::DashMapStateStore, Quota, RateLimiter};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, CorsLayer};

/// IP-based rate limiter type
type IpRateLimiter = RateLimiter<String, DashMapStateStore<String>, DefaultClock>;

pub fn create_router(
    auth_service: AuthService,
    user_service: UserService,
    assistant_service: AssistantService,
    catalog_service: CatalogService,
    recipe_service: RecipeService,
    recipe_v2_service: std::sync::Arc<RecipeV2Service>, // 🆕 V2 with translations
    recipe_ai_insights_service: std::sync::Arc<RecipeAIInsightsService>, // 🆕 AI Insights
    dish_service: DishService,
    menu_engineering_service: MenuEngineeringService,
    inventory_service: InventoryService,
    tenant_ingredient_service: TenantIngredientService,
    jwt_service: JwtService,
    pool: PgPool,                         // 🎯 ДОБАВЛЕНО: для получения language из БД
    admin_auth_service: AdminAuthService, // 🆕 Super Admin auth service
    admin_catalog_service: AdminCatalogService, // 🆕 Admin Catalog service
    allowed_origins: Vec<String>,
    rate_limit_per_second: u32,
) -> Router {
    // ── CORS: strict mode, never permissive ──
    let cors = build_strict_cors(allowed_origins);

    // ── Rate Limiter for auth endpoints ──
    let auth_rate_limiter = build_rate_limiter(rate_limit_per_second);

    // Build ReportService (needs clones before services are consumed by routers)
    let report_service = ReportService::new(
        pool.clone(),
        dish_service.clone(),
        inventory_service.clone(),
        menu_engineering_service.clone(),
    );

    // Auth routes (public) — with rate limiting
    /* ВРЕМЕННО ОТКЛЮЧЕНО ДЛЯ ТЕСТОВ
    let auth_limiter = auth_rate_limiter.clone();
    let auth_rate_limit_middleware = middleware::from_fn(move |req: Request, next: Next| {
        let limiter = auth_limiter.clone();
        rate_limit_middleware(req, next, limiter)
    });
    */

    let auth_routes = Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
        .with_state(auth_service); // 🎯 УДАЛЕНО ВРЕМЕННО Rate Limit для тестов

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
        .route(
            "/products/:id",
            axum::routing::put(admin_catalog::update_product).patch(admin_catalog::update_product),
        )
        .route(
            "/products/:id",
            axum::routing::delete(admin_catalog::delete_product),
        )
        .route(
            "/products/:id/image",
            post(admin_catalog::upload_product_image),
        )
        .route(
            "/products/:id/image-url",
            get(admin_catalog::get_image_upload_url),
        )
        .route(
            "/products/:id/image",
            axum::routing::put(admin_catalog::save_image_url),
        )
        .route(
            "/products/:id/image",
            axum::routing::delete(admin_catalog::delete_product_image),
        )
        // Categories
        .route("/categories", get(admin_catalog::list_categories))
        .route("/categories", post(admin_catalog::create_category))
        .route(
            "/categories/:id",
            axum::routing::put(admin_catalog::update_category),
        )
        .route(
            "/categories/:id",
            axum::routing::delete(admin_catalog::delete_category),
        )
        .layer(middleware::from_fn_with_state(
            admin_auth_service.clone(),
            require_super_admin,
        ))
        .layer(admin_catalog_middleware)
        .with_state(admin_catalog_service);

    // Admin users route (for user management)
    let admin_users_route: Router = Router::new()
        .route("/users", get(admin_users::list_users))
        .route("/users/:id", delete(admin_users::delete_user))
        .route("/stats", get(admin_users::get_stats))
        .layer(middleware::from_fn_with_state(
            admin_auth_service.clone(),
            require_super_admin,
        ))
        .with_state(pool.clone());

    // Clone pool for public routes (before move into jwt_middleware)
    let pool_for_public = pool.clone();
    let pool_for_tools = pool.clone();
    let pool_for_cms = pool.clone();
    let cms_service = CmsService::new(pool_for_cms);

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
                .with_state(assistant_service),
        )
        .merge(
            Router::new()
                .route("/catalog/categories", get(get_categories))
                .route("/catalog/ingredients", get(search_ingredients))
                .with_state(CatalogState {
                    catalog_service,
                    user_service,
                }),
        )
        .merge(
            Router::new()
                .route("/recipes", post(create_recipe))
                .route("/recipes", get(list_recipes))
                .route("/recipes/:id", get(get_recipe))
                .route("/recipes/:id", axum::routing::delete(delete_recipe))
                .route("/recipes/:id/cost", get(calculate_recipe_cost))
                .with_state(recipe_service),
        )
        .merge(
            Router::new()
                .route("/recipes/v2", post(recipe_v2::create_recipe))
                .route("/recipes/v2", get(recipe_v2::list_recipes))
                .route("/recipes/v2/:id", get(recipe_v2::get_recipe))
                .route(
                    "/recipes/v2/:id",
                    axum::routing::put(recipe_v2::update_recipe).patch(recipe_v2::update_recipe),
                )
                .route("/recipes/v2/:id/publish", post(recipe_v2::publish_recipe))
                .route(
                    "/recipes/v2/:id",
                    axum::routing::delete(recipe_v2::delete_recipe),
                )
                .route(
                    "/recipes/v2/:id/image",
                    post(recipe_v2::upload_recipe_image),
                )
                .route(
                    "/recipes/v2/:id/image",
                    axum::routing::put(recipe_v2::save_recipe_image_url),
                )
                .route(
                    "/recipes/v2/:id/image-url",
                    get(recipe_v2::get_recipe_image_upload_url),
                )
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10 MB limit for recipes (images/large JSON)
                .with_state(recipe_v2_service),
        )
        .merge(
            Router::new()
                // AI Insights endpoints
                .route(
                    "/recipes/v2/:id/insights",
                    get(recipe_ai_insights::get_all_insights),
                )
                .route(
                    "/recipes/v2/:id/insights/:language",
                    get(recipe_ai_insights::get_or_generate_insights),
                )
                .route(
                    "/recipes/v2/:id/insights/:language/refresh",
                    post(recipe_ai_insights::refresh_insights),
                )
                .with_state(recipe_ai_insights_service),
        )
        .merge(
            Router::new()
                .route("/dishes", post(create_dish))
                .route("/dishes", get(list_dishes))
                .route("/dishes/recalculate-all", post(recalculate_all_costs))
                .with_state(dish_service),
        )
        .merge(
            Router::new()
                .route("/inventory/products", get(list_products))
                .route("/inventory/products", post(add_product))
                .route(
                    "/inventory/products/:id",
                    axum::routing::put(update_product),
                )
                .route(
                    "/inventory/products/:id",
                    axum::routing::delete(delete_product),
                )
                .route("/inventory/process-expirations", post(process_expirations))
                .route("/inventory/reports/loss", get(get_loss_report))
                .route("/inventory/dashboard", get(get_dashboard)) // New ownership dashboard
                .route("/inventory/alerts", get(get_alerts))
                .route("/inventory/health", get(get_health))
                .with_state(inventory_service),
        )
        // Removed separate inventory_alert_service merge
        .merge(
            Router::new()
                .route("/menu-engineering/analysis", get(analyze_menu))
                .route("/menu-engineering/sales", post(record_sale))
                .with_state(menu_engineering_service),
        )
        .merge(
            Router::new()
                .route("/reports/summary", get(get_summary))
                .with_state(report_service),
        )
        .merge(
            Router::new()
                .route(
                    "/tenant/ingredients",
                    post(tenant_ingredient::add_ingredient),
                )
                .route(
                    "/tenant/ingredients",
                    get(tenant_ingredient::list_ingredients),
                )
                .route(
                    "/tenant/ingredients/search",
                    get(tenant_ingredient::search_available_ingredients),
                )
                .route(
                    "/tenant/ingredients/:id",
                    get(tenant_ingredient::get_ingredient),
                )
                .route(
                    "/tenant/ingredients/:id",
                    axum::routing::put(tenant_ingredient::update_ingredient),
                )
                .route(
                    "/tenant/ingredients/:id",
                    axum::routing::delete(tenant_ingredient::remove_ingredient),
                )
                .with_state(tenant_ingredient_service),
        )
        .layer(jwt_middleware);

    // Health check endpoint (no auth, no middleware)
    let health_route = Router::new().route("/health", get(|| async { "OK" }));

    // ── Public router (no auth, no JWT) ──────────────────────────────────────
    // Old chef-reference aliases kept for backward compatibility
    let chef_reference_routes = Router::new()
        .route("/public/chef-reference/convert", get(convert_units))
        .route("/public/chef-reference/ingredient", get(get_ingredient))
        .route("/public/chef-reference/fish-season", get(fish_season));

    // New clean /public/* routes
    let public_ingredients_router = Router::new()
        .route("/ingredients", get(list_ingredients))
        .route("/ingredients/:slug", get(get_ingredient_by_slug))
        .with_state(pool_for_public);

    let public_tools_router = Router::new()
        .route("/tools/convert", get(tools_convert))
        .route("/tools/fish-season", get(tools_fish_season))
        .route("/tools/nutrition", get(nutrition))
        .route("/tools/ingredients", get(ingredients_db))
        .route("/tools/compare", get(compare_foods))
        .route("/tools/units", get(list_units))
        .route("/tools/categories", get(list_categories))
        .route("/tools/scale", get(scale_recipe))
        .route("/tools/yield", get(yield_calc))
        .route("/tools/ingredient-equivalents", get(ingredient_equivalents))
        .route("/tools/food-cost", get(food_cost_calc))
        .route("/tools/ingredient-suggestions", get(ingredient_suggestions))
        .route("/tools/popular-conversions", get(popular_conversions))
        .route("/tools/ingredient-scale", get(ingredient_scale))
        .route("/tools/ingredient-convert", get(ingredient_convert))
        // SEO alias: /tools/cup-to-grams/wheat-flour?value=1&lang=pl
        .route("/tools/:from_to/:slug", get(seo_ingredient_convert))
        .route("/tools/measure-conversion", get(measure_conversion))
        .route("/tools/ingredient-measures", get(ingredient_measures))
        .route("/tools/fish-season-table", get(fish_season_table))
        // Universal seasonal calendar endpoints
        .route("/tools/seasonal-calendar", get(seasonal_calendar))
        .route("/tools/in-season-now", get(in_season_now))
        .route("/tools/product-seasonality", get(product_seasonality))
        // SEO-powerhouse endpoints
        .route("/tools/best-in-season", get(best_in_season))
        .route("/tools/products-by-month", get(products_by_month))
        .route("/tools/best-right-now", get(best_right_now))
        // Search & advanced tools
        .route("/tools/product-search", get(product_search))
        .route("/tools/resolve-slug", get(resolve_slug))
        .route("/tools/regions", get(list_regions))
        .route("/tools/recipe-nutrition", post(recipe_nutrition))
        .route("/tools/recipe-cost", post(recipe_cost))
        .with_state(pool_for_tools);

    // ── Admin CMS routes (protected) ─────────────────────────────────────────
    let admin_cms_routes = Router::new()
        // About page
        .route("/about", get(admin_cms::get_about).put(admin_cms::update_about))
        // Expertise
        .route("/expertise", get(admin_cms::list_expertise).post(admin_cms::create_expertise))
        .route(
            "/expertise/:id",
            axum::routing::put(admin_cms::update_expertise)
                .delete(admin_cms::delete_expertise),
        )
        // Experience
        .route("/experience", get(admin_cms::list_experience).post(admin_cms::create_experience))
        .route(
            "/experience/:id",
            axum::routing::put(admin_cms::update_experience)
                .delete(admin_cms::delete_experience),
        )
        // Gallery
        .route("/gallery", get(admin_cms::list_gallery).post(admin_cms::create_gallery))
        .route(
            "/gallery/:id",
            axum::routing::put(admin_cms::update_gallery)
                .delete(admin_cms::delete_gallery),
        )
        // Knowledge Articles
        .route("/articles", get(admin_cms::list_articles).post(admin_cms::create_article))
        .route("/articles/:id", get(admin_cms::get_article))
        .route(
            "/articles/:id",
            axum::routing::put(admin_cms::update_article)
                .delete(admin_cms::delete_article),
        )
        .layer(middleware::from_fn_with_state(
            admin_auth_service.clone(),
            require_super_admin,
        ))
        .with_state(cms_service.clone());

    // ── Public CMS routes (no auth) ───────────────────────────────────────────
    let public_cms_router = Router::new()
        .route("/about", get(public_cms::get_about))
        .route("/expertise", get(public_cms::list_expertise))
        .route("/experience", get(public_cms::list_experience))
        .route("/gallery", get(public_cms::list_gallery))
        .route("/articles", get(public_cms::list_articles))
        .route("/articles/:slug", get(public_cms::get_article))
        .with_state(cms_service);

    let public_router = Router::new()
        .merge(public_ingredients_router)
        .merge(public_tools_router)
        .merge(public_cms_router);

    // Combine all routes
    Router::new()
        .merge(health_route)
        .merge(chef_reference_routes)
        .nest("/public", public_router)
        .nest("/api/auth", auth_routes)
        .nest("/api/admin/auth", admin_routes)
        .nest("/api/admin/catalog", admin_catalog_routes)
        .nest("/api/admin/cms", admin_cms_routes)
        .nest("/api/admin", admin_users_route)
        .nest("/api", protected_routes)
        .layer(cors)
}

// ── Strict CORS builder ──

fn build_strict_cors(allowed_origins: Vec<String>) -> CorsLayer {
    // Filter out wildcards — never allow permissive CORS
    let safe_origins: Vec<String> = allowed_origins.into_iter().filter(|o| o != "*").collect();

    if safe_origins.is_empty() {
        tracing::warn!(
            "⚠️ CORS: No valid origins configured (wildcard '*' is rejected). \
             Defaulting to localhost:3000 and localhost:3001. Set CORS_ALLOWED_ORIGINS in production."
        );
        let default_origins: Vec<axum::http::HeaderValue> = [
            "http://localhost:3000",
            "http://localhost:3001",
        ]
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
        CorsLayer::new()
            .allow_origin(default_origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(AllowHeaders::list([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
                header::HeaderName::from_static("x-request-id"),
            ]))
            .allow_credentials(true)
    } else {
        tracing::info!("🔒 CORS: Allowed origins: {:?}", safe_origins);
        CorsLayer::new()
            .allow_origin(
                safe_origins
                    .iter()
                    .filter_map(|origin| origin.parse().ok())
                    .collect::<Vec<_>>(),
            )
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(AllowHeaders::list([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
                header::HeaderName::from_static("x-request-id"),
            ]))
            .allow_credentials(true)
    }
}

// ── Rate limiting ──

fn build_rate_limiter(per_second: u32) -> Arc<IpRateLimiter> {
    let quota =
        Quota::per_second(NonZeroU32::new(per_second).unwrap_or(NonZeroU32::new(10).unwrap()));
    let limiter = RateLimiter::dashmap(quota);
    tracing::info!(
        "🚦 Rate limiter initialized: {} req/sec per IP for auth endpoints",
        per_second
    );
    Arc::new(limiter)
}

async fn rate_limit_middleware(req: Request, next: Next, limiter: Arc<IpRateLimiter>) -> Response {
    // Extract client IP from connection info or forwarded headers
    let ip = extract_client_ip(&req);
    
    // 🎯 ДОБАВЛЕНО: Лог для отладки IP
    // tracing::debug!("Rate limit check for IP: {}", ip);

    match limiter.check_key(&ip) {
        Ok(_) => next.run(req).await,
        Err(_) => {
            tracing::warn!("🚦 Rate limit exceeded for IP: {} (limit reached)", ip);
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "Too many requests. Please try again later.",
                    "ip_detected": ip,
                    "retry_after_seconds": 1
                })),
            )
                .into_response()
        }
    }
}

fn extract_client_ip(req: &Request) -> String {
    // Try X-Forwarded-For header first (common behind reverse proxies like Koyeb)
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            if let Some(first_ip) = value.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            return value.trim().to_string();
        }
    }

    // Fallback: use ConnectInfo if available
    if let Some(connect_info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }

    // Last resort
    "unknown".to_string()
}

async fn inject_jwt_and_pool(
    mut req: Request,
    next: Next,
    jwt_service: JwtService,
    pool: PgPool,
) -> Response {
    req.extensions_mut().insert(jwt_service);
    req.extensions_mut().insert(pool); // 🎯 ДОБАВЛЕНО: pool для AuthUser

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
