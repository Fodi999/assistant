use crate::application::analytics::AnalyticsService;
use crate::shared::AppError;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use serde::Deserialize;

use super::site_context::{resolve_site_id, SiteQuery, KITCHEN_SITE_ID};

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OverviewQuery {
    pub days: Option<u16>,
    pub site_id: Option<uuid::Uuid>,
    pub site: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConnectionRequest {
    pub google_property_id: Option<String>,
}

/// GET /auth/google
pub async fn google_login(
    State(service): State<AnalyticsService>,
) -> Result<impl IntoResponse, AppError> {
    let oauth = service.oauth_url_for_site(KITCHEN_SITE_ID)?;
    Ok(Redirect::temporary(&oauth.url))
}

/// GET /api/admin/analytics/oauth/url
pub async fn oauth_url(
    State(service): State<AnalyticsService>,
    Query(query): Query<SiteQuery>,
) -> Result<impl IntoResponse, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    Ok(Json(service.oauth_url_for_site(site_id)?))
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
    let site_query = SiteQuery {
        site_id: query.site_id,
        site: query.site,
    };
    let site_id = resolve_site_id(&site_query, KITCHEN_SITE_ID);
    Ok(Json(service.overview_for_site(site_id, days).await?))
}

/// GET /api/admin/analytics/realtime
pub async fn realtime(
    State(service): State<AnalyticsService>,
    Query(query): Query<SiteQuery>,
) -> Result<impl IntoResponse, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    Ok(Json(service.realtime_for_site(site_id).await?))
}

/// GET /api/admin/analytics/connection
pub async fn connection(
    State(service): State<AnalyticsService>,
    Query(query): Query<SiteQuery>,
) -> Result<impl IntoResponse, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    Ok(Json(service.connection_status(site_id).await?))
}

/// PATCH /api/admin/analytics/connection
pub async fn update_connection(
    State(service): State<AnalyticsService>,
    Query(query): Query<SiteQuery>,
    Json(req): Json<UpdateConnectionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    Ok(Json(
        service
            .update_connection_property_id(site_id, req.google_property_id)
            .await?,
    ))
}
