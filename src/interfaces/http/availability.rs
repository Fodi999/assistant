//! GET /v1/businesses/:business_id/availability

use crate::application::availability::{AvailabilityQuery, AvailabilityView};
use crate::interfaces::http::{
    extract::{member_access, AuthUser},
    state::AppState,
};
use crate::shared::AppResult;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

pub async fn get_availability(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Query(query): Query<AvailabilityQuery>,
) -> AppResult<Json<AvailabilityView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.availability.availability(access, query).await?))
}
