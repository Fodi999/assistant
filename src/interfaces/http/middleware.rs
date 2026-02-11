use crate::application::AdminAuthService;
use crate::domain::AdminClaims;
use crate::infrastructure::JwtService;
use crate::shared::{AppError, Language, TenantId, UserId};
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
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub language: Language,  // 🎯 ДОБАВЛЕНО: источник языка = backend!
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
        let user_id = claims.user_id()?;
        let tenant_id = claims.tenant_id()?;

        // Get user's language from database (source of truth!)
        let pool = parts
            .extensions
            .get::<PgPool>()
            .ok_or_else(|| AppError::internal("Database pool not configured"))?
            .clone();

        let language = sqlx::query_scalar::<_, String>(
            "SELECT language FROM users WHERE id = $1"
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&pool)
        .await?
        .and_then(|lang| Language::from_str(&lang).ok())
        .unwrap_or(Language::En);  // Fallback to English

        Ok(AuthUser {
            user_id,
            tenant_id,
            language,  // 🎯 Backend = source of truth!
        })
    }
}

/// AdminClaims extractor - validates Super Admin JWT token
#[async_trait]
impl<S> FromRequestParts<S> for AdminClaims
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract AdminAuthService from extensions
        let admin_service = parts
            .extensions
            .get::<AdminAuthService>()
            .ok_or_else(|| AppError::internal("Admin auth service not configured"))?
            .clone();

        // Extract Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::authentication("Missing or invalid authorization header"))?;

        // Verify admin token
        let claims = admin_service.verify_token(bearer.token())?;

        Ok(claims)
    }
}
