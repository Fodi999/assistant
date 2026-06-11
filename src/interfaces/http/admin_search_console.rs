use crate::application::analytics::AnalyticsService;
use crate::shared::AppError;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchConsoleQuery {
    pub site_url: Option<String>,
    pub days: Option<u16>,
    pub limit: Option<u16>,
}

/// GET /api/admin/search-console/sites
pub async fn sites(State(service): State<AnalyticsService>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(service.search_console_sites().await?))
}

/// GET /api/admin/search-console/overview?site_url=...&days=30
pub async fn overview(
    State(service): State<AnalyticsService>,
    Query(query): Query<SearchConsoleQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        service
            .search_console_overview(query.site_url, query.days.unwrap_or(30))
            .await?,
    ))
}

/// GET /api/admin/search-console/queries?site_url=...&days=30&limit=25
pub async fn queries(
    State(service): State<AnalyticsService>,
    Query(query): Query<SearchConsoleQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        service
            .search_console_queries(
                query.site_url,
                query.days.unwrap_or(30),
                query.limit.unwrap_or(25),
            )
            .await?,
    ))
}

/// GET /api/admin/search-console/pages?site_url=...&days=30&limit=25
pub async fn pages(
    State(service): State<AnalyticsService>,
    Query(query): Query<SearchConsoleQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        service
            .search_console_pages(
                query.site_url,
                query.days.unwrap_or(30),
                query.limit.unwrap_or(25),
            )
            .await?,
    ))
}

/// GET /api/admin/search-console/daily?site_url=...&days=30&limit=90
pub async fn daily(
    State(service): State<AnalyticsService>,
    Query(query): Query<SearchConsoleQuery>,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(
        service
            .search_console_daily(
                query.site_url,
                query.days.unwrap_or(30),
                query.limit.unwrap_or(90),
            )
            .await?,
    ))
}
