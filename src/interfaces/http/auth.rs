//! /v1/auth/* and /v1/me

use crate::application::auth::{AuthResponse, LoginInput, MeResponse, RegisterInput, TokenPair};
use crate::interfaces::http::{extract::AuthUser, state::AppState};
use crate::shared::AppResult;
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RefreshInput {
    pub refresh_token: String,
}

/// POST /v1/auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    let response = state.auth.register(input).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> AppResult<Json<AuthResponse>> {
    Ok(Json(state.auth.login(input).await?))
}

/// POST /v1/auth/refresh
pub async fn refresh(
    State(state): State<AppState>,
    Json(input): Json<RefreshInput>,
) -> AppResult<Json<TokenPair>> {
    Ok(Json(state.auth.refresh(&input.refresh_token).await?))
}

/// POST /v1/auth/logout — always 204, whether or not the token was valid.
pub async fn logout(
    State(state): State<AppState>,
    Json(input): Json<RefreshInput>,
) -> AppResult<StatusCode> {
    state.auth.logout(&input.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /v1/me
pub async fn me(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<MeResponse>> {
    Ok(Json(state.auth.me(user_id).await?))
}
