//! /v1/businesses/:business_id/holds

use crate::application::booking::{CreateHoldInput, HoldView};
use crate::interfaces::http::{
    extract::{member_access, AuthUser},
    state::AppState,
};
use crate::shared::{AppError, AppResult};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use uuid::Uuid;

/// POST /holds — needs an `Idempotency-Key` header. 201 for a new hold, 200
/// when the same key and request were already processed.
pub async fn create_hold(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CreateHoldInput>,
) -> AppResult<(StatusCode, Json<HoldView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::validation("Idempotency-Key header is required"))?;
    let outcome = state.booking.create_hold(access, key, input).await?;
    let status = if outcome.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(outcome.hold)))
}

pub async fn get_hold(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, hold_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<HoldView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.booking.get_hold(access, hold_id).await?))
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
