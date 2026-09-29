//! /v1/businesses/:business_id/members

use crate::application::team::{AddMemberInput, MemberView};
use crate::interfaces::http::{
    extract::{member_access, AuthUser},
    state::AppState,
};
use crate::shared::AppResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

pub async fn add_member(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Json(input): Json<AddMemberInput>,
) -> AppResult<(StatusCode, Json<MemberView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let view = state.team.add_member(access, input).await?;
    Ok((StatusCode::CREATED, Json(view)))
}
