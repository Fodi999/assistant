//! /v1/public/*: the customer-facing API.
//!
//! No sign-in: guest session, catalog, profile, availability.
//! Signed in (registered user or guest session): holds, bookings and the
//! customer's own appointments.

use crate::application::auth::{AuthResponse, GuestInput};
use crate::application::availability::{AvailabilityQuery, AvailabilityView};
use crate::application::booking::{AppointmentView, CancelInput, CreateHoldInput, RescheduleInput};
use crate::application::public::{
    CatalogItem, CatalogQuery, CustomerBookInput, MyAppointmentsQuery, PublicProfile,
};
use crate::interfaces::http::booking::{created_or_replayed, idempotency_key};
use crate::interfaces::http::{extract::AuthUser, state::AppState};
use crate::shared::{AppError, AppResult};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use uuid::Uuid;

/// POST /v1/public/guest — an anonymous customer session (needs accepted terms).
pub async fn guest(
    State(state): State<AppState>,
    Json(input): Json<GuestInput>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    Ok((StatusCode::CREATED, Json(state.auth.guest(input).await?)))
}

/// GET /v1/public/businesses?city=&q=&limit=&offset=
pub async fn catalog(
    State(state): State<AppState>,
    Query(query): Query<CatalogQuery>,
) -> AppResult<Json<Vec<CatalogItem>>> {
    Ok(Json(state.public.catalog(query).await?))
}

/// GET /v1/public/businesses/:key — `key` is the business id or its slug.
pub async fn profile(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> AppResult<Json<PublicProfile>> {
    Ok(Json(state.public.profile(&key).await?))
}

/// GET /v1/public/businesses/:key/availability
pub async fn availability(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<AvailabilityQuery>,
) -> AppResult<Json<AvailabilityView>> {
    Ok(Json(state.public.availability(&key, query).await?))
}

/// POST /v1/public/businesses/:key/holds — needs an `Idempotency-Key` header.
pub async fn create_hold(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(key): Path<String>,
    headers: HeaderMap,
    Json(input): Json<CreateHoldInput>,
) -> AppResult<(StatusCode, Json<AppointmentView>)> {
    let idem = idempotency_key(&headers)
        .ok_or_else(|| AppError::validation("Idempotency-Key header is required"))?;
    let outcome = state.public.hold(user_id, &key, idem, input).await?;
    Ok((
        created_or_replayed(outcome.created),
        Json(outcome.appointment),
    ))
}

/// DELETE /v1/public/businesses/:key/holds/:id
pub async fn release_hold(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((key, hold_id)): Path<(String, Uuid)>,
) -> AppResult<StatusCode> {
    state.public.release_hold(user_id, &key, hold_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /v1/public/businesses/:key/appointments — confirms a hold or books
/// directly (then `Idempotency-Key` is required).
pub async fn book(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(key): Path<String>,
    headers: HeaderMap,
    Json(input): Json<CustomerBookInput>,
) -> AppResult<(StatusCode, Json<AppointmentView>)> {
    let outcome = state
        .public
        .book(user_id, &key, idempotency_key(&headers), input)
        .await?;
    Ok((
        created_or_replayed(outcome.created),
        Json(outcome.appointment),
    ))
}

/// GET /v1/public/businesses/:key/appointments — the caller's own only.
pub async fn my_appointments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(key): Path<String>,
    Query(query): Query<MyAppointmentsQuery>,
) -> AppResult<Json<Vec<AppointmentView>>> {
    Ok(Json(
        state.public.my_appointments(user_id, &key, query).await?,
    ))
}

pub async fn my_appointment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((key, id)): Path<(String, Uuid)>,
) -> AppResult<Json<AppointmentView>> {
    Ok(Json(state.public.my_appointment(user_id, &key, id).await?))
}

pub async fn cancel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((key, id)): Path<(String, Uuid)>,
    input: Option<Json<CancelInput>>,
) -> AppResult<Json<AppointmentView>> {
    let input = input
        .map(|Json(input)| input)
        .unwrap_or(CancelInput { reason: None });
    Ok(Json(state.public.cancel(user_id, &key, id, input).await?))
}

pub async fn reschedule(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((key, id)): Path<(String, Uuid)>,
    Json(input): Json<RescheduleInput>,
) -> AppResult<Json<AppointmentView>> {
    Ok(Json(
        state.public.reschedule(user_id, &key, id, input).await?,
    ))
}
