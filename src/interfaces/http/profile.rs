//! /v1/businesses/:business_id/profile

use crate::application::profile::{ProfileView, UpdateProfileInput};
use crate::interfaces::http::{
    extract::{member_access, AuthUser},
    state::AppState,
};
use crate::shared::AppResult;
use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

/// GET — every member.
pub async fn get_profile(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
) -> AppResult<Json<ProfileView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.profile.get(access).await?))
}

/// PUT — owner or manager. Absent fields stay; an empty string clears one.
pub async fn update_profile(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Json(input): Json<UpdateProfileInput>,
) -> AppResult<Json<ProfileView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.profile.update(access, input).await?))
}
