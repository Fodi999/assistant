use crate::application::{AssistantService, AuthService, CatalogService, RecipeService, UserService};
use crate::infrastructure::JwtService;
use crate::interfaces::http::{
    assistant::{get_state, send_command},
    auth::{login_handler, refresh_handler, register_handler},
    catalog::{get_categories, search_ingredients, CatalogState},
    middleware::AuthUser,
    recipe::{create_recipe, get_recipe, list_recipes, delete_recipe, calculate_recipe_cost},
    user::me_handler,
};
use axum::{
    extract::{FromRequestParts, Request},
    http::{Method, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

pub fn create_router(
    auth_service: AuthService,
    user_service: UserService,
    assistant_service: AssistantService,
    catalog_service: CatalogService,
    recipe_service: RecipeService,
    jwt_service: JwtService,
    allowed_origins: Vec<String>,
) -> Router {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .iter()
                .filter_map(|origin| origin.parse().ok())
                .collect::<Vec<_>>(),
        )
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);

    // Auth routes (public)
    let auth_routes = Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
        .with_state(auth_service);

    // Protected routes
    let jwt_middleware = middleware::from_fn(move |req: Request, next: Next| {
        let jwt_service = jwt_service.clone();
        async move { inject_jwt_service(req, next, jwt_service).await }
    });

    let protected_routes = Router::new()
        .route("/me", get(me_handler))
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
        .layer(jwt_middleware);

    // Combine all routes
    Router::new()
        .nest("/api/auth", auth_routes)
        .nest("/api", protected_routes)
        .layer(cors)
}

async fn inject_jwt_service(
    mut req: Request,
    next: Next,
    jwt_service: JwtService,
) -> Response {
    req.extensions_mut().insert(jwt_service);
    
    // Попытка извлечь AuthUser через экстрактор
    // Если успешно - добавляем в extensions
    let mut parts = req.into_parts();
    if let Ok(auth_user) = AuthUser::from_request_parts(&mut parts.0, &()).await {
        parts.0.extensions.insert(auth_user);
    }
    let req = Request::from_parts(parts.0, parts.1);
    
    next.run(req).await
}
