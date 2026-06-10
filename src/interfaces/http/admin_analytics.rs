use crate::application::analytics::AnalyticsService;
use crate::shared::AppError;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OverviewQuery {
    pub days: Option<u16>,
}

/// GET /auth/google
pub async fn google_login(
    State(service): State<AnalyticsService>,
) -> Result<impl IntoResponse, AppError> {
    let oauth = service.oauth_url()?;
    Ok(Redirect::temporary(&oauth.url))
}

/// GET /api/admin/analytics/oauth/url
pub async fn oauth_url(
    State(service): State<AnalyticsService>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(service.oauth_url()?))
}

/// GET /api/admin/analytics/oauth/callback?code=...&state=...
///
/// Returns the refresh_token once. Copy it into GA4_REFRESH_TOKEN on backend env.
pub async fn oauth_callback(
    State(service): State<AnalyticsService>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    let token = service
        .exchange_code(&query.code, query.state.as_deref())
        .await?;
    Ok(Json(token))
}

/// GET /api/admin/analytics/overview?days=30
pub async fn overview(
    State(service): State<AnalyticsService>,
    Query(query): Query<OverviewQuery>,
) -> Result<impl IntoResponse, AppError> {
    let days = query.days.unwrap_or(30);
    Ok(Json(service.overview(days).await?))
}
