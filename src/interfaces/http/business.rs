//! /v1/businesses

use crate::application::business::{
    BusinessView, CreateBusinessInput, StaffView, UpdateBusinessInput, UpdateStaffInput,
};
use crate::interfaces::http::{extract::AuthUser, state::AppState};
use crate::shared::{AppResult, BusinessId};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

/// POST /v1/businesses — the caller becomes its owner.
pub async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(input): Json<CreateBusinessInput>,
) -> AppResult<(StatusCode, Json<BusinessView>)> {
    let view = state.business.create(user_id, input).await?;
    Ok((StatusCode::CREATED, Json(view)))
}

/// GET /v1/businesses/:business_id — any active member.
pub async fn get(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
) -> AppResult<Json<BusinessView>> {
    let access = state
        .business
        .access(user_id, BusinessId::from_uuid(business_id))
        .await?;
    Ok(Json(state.business.get(access).await?))
}

/// PATCH /v1/businesses/:business_id — owner or manager.
pub async fn update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Json(input): Json<UpdateBusinessInput>,
) -> AppResult<Json<BusinessView>> {
    let access = state
        .business
        .access(user_id, BusinessId::from_uuid(business_id))
        .await?;
    Ok(Json(state.business.update(access, input).await?))
}

/// GET /v1/businesses/:business_id/staff — any active member.
pub async fn list_staff(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
) -> AppResult<Json<Vec<StaffView>>> {
    let access = state
        .business
        .access(user_id, BusinessId::from_uuid(business_id))
        .await?;
    Ok(Json(state.business.list_staff(access).await?))
}

/// PATCH /v1/businesses/:business_id/staff/:staff_id — owner or manager for any
/// card; any other member only for their own.
pub async fn update_staff(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, staff_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateStaffInput>,
) -> AppResult<Json<StaffView>> {
    let access = state
        .business
        .access(user_id, BusinessId::from_uuid(business_id))
        .await?;
    Ok(Json(
        state.business.update_staff(access, staff_id, input).await?,
    ))
}
