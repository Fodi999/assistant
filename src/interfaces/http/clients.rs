//! /v1/businesses/:business_id/clients. Owner, manager and reception see and edit
//! all clients; an employee reads only those of their own appointments.

use crate::application::booking::{AppointmentView, HistoryQuery};
use crate::application::clients::{ClientQuery, ClientView, CreateClientInput, UpdateClientInput};
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

/// POST /clients — a card without a visit.
pub async fn create_client(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Json(input): Json<CreateClientInput>,
) -> AppResult<(StatusCode, Json<ClientView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let created = state.clients.create(access, input).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// PATCH /clients/:id
pub async fn update_client(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, client_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateClientInput>,
) -> AppResult<Json<ClientView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.clients.update(access, client_id, input).await?))
}

/// GET /clients/:id/appointments?limit=&offset=
pub async fn client_appointments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, client_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<HistoryQuery>,
) -> AppResult<Json<Vec<AppointmentView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state
            .booking
            .client_history(access, client_id, query)
            .await?,
    ))
}
