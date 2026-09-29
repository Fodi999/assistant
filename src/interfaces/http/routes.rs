use crate::interfaces::http::{auth, business, catalog, health, state::AppState};
use axum::{
    http::{header, HeaderName, HeaderValue, Method},
    routing::{get, patch, post, put},
    Router,
};
use std::time::Duration;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

/// Strict CORS: only the configured origins, no wildcard. An empty list means
/// no browser origin is allowed (native apps are not subject to CORS).
fn build_cors(allowed_origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|origin| match HeaderValue::from_str(origin) {
            Ok(value) => Some(value),
            Err(_) => {
                tracing::warn!("Ignoring invalid CORS origin: {}", origin);
                None
            }
        })
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("idempotency-key"),
            header::IF_MATCH,
        ])
        .max_age(Duration::from_secs(3600))
}

pub fn create_router(state: AppState) -> Router {
    let cors = build_cors(&state.config.cors.allowed_origins);

    Router::new()
        .route("/health", get(health::health_check))
        .route("/ready", get(health::ready_check))
        .route("/v1/auth/register", post(auth::register))
        .route("/v1/auth/login", post(auth::login))
        .route("/v1/auth/refresh", post(auth::refresh))
        .route("/v1/auth/logout", post(auth::logout))
        .route("/v1/me", get(auth::me))
        .route("/v1/businesses", post(business::create))
        .route(
            "/v1/businesses/:business_id",
            get(business::get).patch(business::update),
        )
        .route("/v1/businesses/:business_id/staff", get(business::list_staff))
        .route(
            "/v1/businesses/:business_id/categories",
            get(catalog::list_categories).post(catalog::create_category),
        )
        .route(
            "/v1/businesses/:business_id/categories/:category_id",
            patch(catalog::update_category).delete(catalog::delete_category),
        )
        .route(
            "/v1/businesses/:business_id/services",
            get(catalog::list_services).post(catalog::create_service),
        )
        .route(
            "/v1/businesses/:business_id/services/:service_id",
            get(catalog::get_service)
                .patch(catalog::update_service)
                .delete(catalog::delete_service),
        )
        .route(
            "/v1/businesses/:business_id/services/:service_id/variants",
            post(catalog::create_variant),
        )
        .route(
            "/v1/businesses/:business_id/services/:service_id/staff",
            put(catalog::set_service_staff),
        )
        .route(
            "/v1/businesses/:business_id/variants/:variant_id",
            patch(catalog::update_variant).delete(catalog::delete_variant),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
