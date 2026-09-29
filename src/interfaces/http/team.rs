//! /v1/businesses/:business_id/members

use crate::application::team::{AddMemberInput, MemberDetail, MemberView, UpdateMemberInput};
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

pub async fn list_members(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
) -> AppResult<Json<Vec<MemberDetail>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.team.list_members(access).await?))
}

pub async fn update_member(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, membership_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateMemberInput>,
) -> AppResult<Json<MemberDetail>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state
            .team
            .update_member(access, membership_id, input)
            .await?,
    ))
}
