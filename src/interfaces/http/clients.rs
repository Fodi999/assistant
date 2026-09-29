//! /v1/businesses/:business_id/clients (owner, manager, reception)

use crate::application::clients::{ClientQuery, ClientView};
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

pub async fn list_clients(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Query(query): Query<ClientQuery>,
) -> AppResult<Json<Vec<ClientView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.clients.list(access, query).await?))
}

pub async fn get_client(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, client_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<ClientView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.clients.get(access, client_id).await?))
}
