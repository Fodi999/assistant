//! /v1/businesses/:business_id/{holds,appointments}

use crate::application::booking::{
    AppointmentQuery, AppointmentView, CancelInput, CreateAppointmentInput, CreateHoldInput,
    EventView, RescheduleInput,
};
use crate::interfaces::http::{
    extract::{member_access, AuthUser},
    state::AppState,
};
use crate::shared::{AppError, AppResult};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use uuid::Uuid;

pub(crate) fn idempotency_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
}

pub(crate) fn created_or_replayed(created: bool) -> StatusCode {
    if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    }
}

/// POST /holds — needs an `Idempotency-Key` header. 201 for a new hold, 200
/// when the same key and request were already processed.
pub async fn create_hold(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CreateHoldInput>,
) -> AppResult<(StatusCode, Json<AppointmentView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let key = idempotency_key(&headers)
        .ok_or_else(|| AppError::validation("Idempotency-Key header is required"))?;
    let outcome = state.booking.create_hold(access, key, input).await?;
    Ok((
        created_or_replayed(outcome.created),
        Json(outcome.appointment),
    ))
}

pub async fn release_hold(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, hold_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let access = member_access(&state, user_id, business_id).await?;
    state.booking.release_hold(access, hold_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /appointments — confirms a hold (`hold_id`) or books directly (then an
/// `Idempotency-Key` header is required).
pub async fn create_appointment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CreateAppointmentInput>,
) -> AppResult<(StatusCode, Json<AppointmentView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let outcome = state
        .booking
        .create_appointment(access, idempotency_key(&headers), input)
        .await?;
    Ok((
        created_or_replayed(outcome.created),
        Json(outcome.appointment),
    ))
}

/// GET /appointments?from=&to=&staff_id=&status=
pub async fn list_appointments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Query(query): Query<AppointmentQuery>,
) -> AppResult<Json<Vec<AppointmentView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.booking.list_appointments(access, query).await?))
}

/// GET /appointments/:id and GET /holds/:id (a hold is an appointment row).
pub async fn get_appointment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<AppointmentView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.booking.get_appointment(access, id).await?))
}

/// POST /appointments/:id/cancel — the body (`{"reason": "..."}`) is optional.
pub async fn cancel_appointment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, id)): Path<(Uuid, Uuid)>,
    body: Option<Json<CancelInput>>,
) -> AppResult<Json<AppointmentView>> {
    let access = member_access(&state, user_id, business_id).await?;
    let input = body
        .map(|Json(input)| input)
        .unwrap_or(CancelInput { reason: None });
    Ok(Json(state.booking.cancel(access, id, input).await?))
}

pub async fn reschedule_appointment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<RescheduleInput>,
) -> AppResult<Json<AppointmentView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.booking.reschedule(access, id, input).await?))
}

pub async fn appointment_history(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Vec<EventView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.booking.history(access, id).await?))
}
