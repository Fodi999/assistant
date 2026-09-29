//! /v1/businesses/:business_id/staff/:staff_id/{schedule,time-off}

use crate::application::schedule::{
    BreakView, ExceptionInput, ExceptionView, ScheduleView, SetBreaksInput, SetWeeklyInput,
    TimeOffInput, TimeOffQuery, TimeOffView, WeeklyView,
};
use crate::interfaces::http::{
    extract::{member_access, AuthUser},
    state::AppState,
};
use crate::shared::AppResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

pub async fn get_schedule(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, staff_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<ScheduleView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.schedule.get_schedule(access, staff_id).await?))
}

pub async fn set_weekly(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, staff_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<SetWeeklyInput>,
) -> AppResult<Json<Vec<WeeklyView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state.schedule.set_weekly(access, staff_id, input).await?,
    ))
}

pub async fn set_breaks(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, staff_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<SetBreaksInput>,
) -> AppResult<Json<Vec<BreakView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state.schedule.set_breaks(access, staff_id, input).await?,
    ))
}

pub async fn put_exception(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, staff_id, date)): Path<(Uuid, Uuid, String)>,
    Json(input): Json<ExceptionInput>,
) -> AppResult<Json<ExceptionView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state
            .schedule
            .put_exception(access, staff_id, &date, input)
            .await?,
    ))
}

pub async fn delete_exception(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, staff_id, date)): Path<(Uuid, Uuid, String)>,
) -> AppResult<StatusCode> {
    let access = member_access(&state, user_id, business_id).await?;
    state
        .schedule
        .delete_exception(access, staff_id, &date)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_time_off(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, staff_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<TimeOffQuery>,
) -> AppResult<Json<Vec<TimeOffView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state
            .schedule
            .list_time_off(access, staff_id, query)
            .await?,
    ))
}

pub async fn create_time_off(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, staff_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<TimeOffInput>,
) -> AppResult<(StatusCode, Json<TimeOffView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let view = state
        .schedule
        .create_time_off(access, staff_id, input)
        .await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn delete_time_off(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, time_off_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let access = member_access(&state, user_id, business_id).await?;
    state.schedule.delete_time_off(access, time_off_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
