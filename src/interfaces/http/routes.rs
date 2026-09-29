use crate::interfaces::http::{
    admin, auth, availability, booking, business, catalog, clients, health, profile, public,
    schedule, state::AppState, team,
};
use axum::{
    http::{header, HeaderName, HeaderValue, Method},
    routing::{delete, get, patch, post, put},
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
        .route(
            "/v1/businesses/:business_id/staff",
            get(business::list_staff),
        )
        .route(
            "/v1/businesses/:business_id/staff/:staff_id",
            patch(business::update_staff),
        )
        .route(
            "/v1/businesses/:business_id/members",
            get(team::list_members).post(team::add_member),
        )
        .route(
            "/v1/businesses/:business_id/members/:membership_id",
            patch(team::update_member),
        )
        .route(
            "/v1/businesses/:business_id/availability",
            get(availability::get_availability),
        )
        .route(
            "/v1/businesses/:business_id/holds",
            post(booking::create_hold),
        )
        .route(
            "/v1/businesses/:business_id/holds/:hold_id",
            get(booking::get_appointment).delete(booking::release_hold),
        )
        .route(
            "/v1/businesses/:business_id/appointments",
            post(booking::create_appointment).get(booking::list_appointments),
        )
        .route(
            "/v1/businesses/:business_id/appointments/:appointment_id",
            get(booking::get_appointment),
        )
        .route(
            "/v1/businesses/:business_id/appointments/:appointment_id/cancel",
            post(booking::cancel_appointment),
        )
        .route(
            "/v1/businesses/:business_id/appointments/:appointment_id/complete",
            post(booking::complete_appointment),
        )
        .route(
            "/v1/businesses/:business_id/appointments/:appointment_id/no-show",
            post(booking::no_show_appointment),
        )
        .route(
            "/v1/businesses/:business_id/appointments/:appointment_id/reschedule",
            post(booking::reschedule_appointment),
        )
        .route(
            "/v1/businesses/:business_id/appointments/:appointment_id/history",
            get(booking::appointment_history),
        )
        .route(
            "/v1/businesses/:business_id/profile",
            get(profile::get_profile).put(profile::update_profile),
        )
        .route(
            "/v1/businesses/:business_id/clients",
            get(clients::list_clients).post(clients::create_client),
        )
        .route(
            "/v1/businesses/:business_id/clients/:client_id",
            get(clients::get_client).patch(clients::update_client),
        )
        .route(
            "/v1/businesses/:business_id/clients/:client_id/appointments",
            get(clients::client_appointments),
        )
        .route("/v1/public/guest", post(public::guest))
        .route("/v1/public/businesses", get(public::catalog))
        .route("/v1/public/businesses/:key", get(public::profile))
        .route(
            "/v1/public/businesses/:key/availability",
            get(public::availability),
        )
        .route(
            "/v1/public/businesses/:key/holds",
            post(public::create_hold),
        )
        .route(
            "/v1/public/businesses/:key/holds/:hold_id",
            delete(public::release_hold),
        )
        .route(
            "/v1/public/businesses/:key/appointments",
            post(public::book).get(public::my_appointments),
        )
        .route(
            "/v1/public/businesses/:key/appointments/:appointment_id",
            get(public::my_appointment),
        )
        .route(
            "/v1/public/businesses/:key/appointments/:appointment_id/cancel",
            post(public::cancel),
        )
        .route(
            "/v1/public/businesses/:key/appointments/:appointment_id/reschedule",
            post(public::reschedule),
        )
        .route("/v1/admin/businesses", get(admin::list))
        .route("/v1/admin/businesses/:business_id", get(admin::detail))
        .route(
            "/v1/admin/businesses/:business_id/approve",
            post(admin::approve),
        )
        .route(
            "/v1/admin/businesses/:business_id/reject",
            post(admin::reject),
        )
        .route(
            "/v1/admin/businesses/:business_id/suspend",
            post(admin::suspend),
        )
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
        .route(
            "/v1/businesses/:business_id/staff/:staff_id/schedule",
            get(schedule::get_schedule),
        )
        .route(
            "/v1/businesses/:business_id/staff/:staff_id/schedule/weekly",
            put(schedule::set_weekly),
        )
        .route(
            "/v1/businesses/:business_id/staff/:staff_id/schedule/breaks",
            put(schedule::set_breaks),
        )
        .route(
            "/v1/businesses/:business_id/staff/:staff_id/schedule/exceptions/:date",
            put(schedule::put_exception).delete(schedule::delete_exception),
        )
        .route(
            "/v1/businesses/:business_id/staff/:staff_id/time-off",
            get(schedule::list_time_off).post(schedule::create_time_off),
        )
        .route(
            "/v1/businesses/:business_id/time-off/:time_off_id",
            delete(schedule::delete_time_off),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
