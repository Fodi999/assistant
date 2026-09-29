//! Request extractors.

use crate::interfaces::http::state::AppState;
use crate::shared::{AppError, UserId};
use axum::{async_trait, extract::FromRequestParts, http::header, http::request::Parts};

/// The signed-in user, taken from `Authorization: Bearer <access token>`.
/// Business membership and role are resolved separately per business
/// (see `BusinessService::access`), never trusted from the token.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser(pub UserId);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::authentication("Missing Authorization header"))?;
        let token = value
            .strip_prefix("Bearer ")
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| AppError::authentication("Expected a Bearer token"))?;

        let claims = state.jwt.verify_access_token(token)?;
        Ok(AuthUser(claims.user_id()?))
    }
}
