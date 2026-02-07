use crate::infrastructure::JwtService;
use crate::shared::{AppError, TenantId, UserId};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: UserId,
    pub tenant_id: TenantId,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract JWT service from extensions
        let jwt_service = parts
            .extensions
            .get::<JwtService>()
            .ok_or_else(|| AppError::internal("JWT service not configured"))?
            .clone();

        // Extract Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::authentication("Missing or invalid authorization header"))?;

        // Verify token
        let claims = jwt_service.verify_access_token(bearer.token())?;

        Ok(AuthUser {
            user_id: claims.user_id()?,
            tenant_id: claims.tenant_id()?,
        })
    }
}
