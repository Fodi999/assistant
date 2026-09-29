//! Request extractors.

use crate::application::BusinessAccess;
use crate::interfaces::http::state::AppState;
use crate::shared::{AppError, AppResult, BusinessId, UserId};
use axum::{async_trait, extract::FromRequestParts, http::header, http::request::Parts};
use uuid::Uuid;

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

/// Resolves the caller's active membership in the business named by the URL.
/// Non-members get 404 (the business is not revealed to them).
pub async fn member_access(
    state: &AppState,
    user_id: UserId,
    business_id: Uuid,
) -> AppResult<BusinessAccess> {
    state
        .business
        .access(user_id, BusinessId::from_uuid(business_id))
        .await
}
