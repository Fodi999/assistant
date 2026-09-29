//! /v1/admin/*: platform moderation (platform admins only).

use crate::application::admin::{AdminBusinessView, AdminListQuery, DecisionInput};
use crate::interfaces::http::{extract::AuthUser, state::AppState};
use crate::shared::AppResult;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

/// GET /v1/admin/businesses?status=pending
pub async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(query): Query<AdminListQuery>,
) -> AppResult<Json<Vec<AdminBusinessView>>> {
    Ok(Json(state.admin.list(user_id, query).await?))
}

async fn decide(
    state: AppState,
    admin: crate::shared::UserId,
    business_id: Uuid,
    target: &'static str,
    input: Option<Json<DecisionInput>>,
) -> AppResult<Json<AdminBusinessView>> {
    let input = input.map(|Json(input)| input).unwrap_or_default();
    Ok(Json(
        state
            .admin
            .decide(admin, business_id, target, input)
            .await?,
    ))
}

/// POST /v1/admin/businesses/:id/approve
pub async fn approve(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    input: Option<Json<DecisionInput>>,
) -> AppResult<Json<AdminBusinessView>> {
    decide(state, user_id, business_id, "approved", input).await
}

/// POST /v1/admin/businesses/:id/reject — a pending business; note required.
pub async fn reject(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    input: Option<Json<DecisionInput>>,
) -> AppResult<Json<AdminBusinessView>> {
    decide(state, user_id, business_id, "rejected", input).await
}

/// POST /v1/admin/businesses/:id/suspend — an approved business; note required.
pub async fn suspend(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    input: Option<Json<DecisionInput>>,
) -> AppResult<Json<AdminBusinessView>> {
    decide(state, user_id, business_id, "suspended", input).await
}
