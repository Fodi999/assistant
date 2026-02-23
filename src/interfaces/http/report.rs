use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;

use crate::application::report::ReportService;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    #[serde(default = "default_period")]
    pub period_days: u32,
}

fn default_period() -> u32 {
    30
}

/// GET /api/reports/summary?period_days=30
/// Executive summary for the restaurant owner — "one glance" KPI dashboard.
pub async fn get_summary(
    State(service): State<ReportService>,
    auth: AuthUser,
    axum::extract::Query(query): axum::extract::Query<ReportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let period_days = query.period_days.max(1).min(365);

    let summary = service
        .get_summary(auth.user_id, auth.tenant_id, auth.language, period_days)
        .await?;

    Ok(Json(summary))
}
