use axum::{extract::State, response::IntoResponse, Json};

use crate::application::city_engine::CityEngineService;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

/// GET /api/city/map
/// Returns the full CityMap JSON for the authenticated tenant.
/// Frontend renders this — no geometry is hardcoded on the client.
pub async fn get_city_map(
    State(service): State<CityEngineService>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let map = service
        .generate_map(auth.user_id, auth.tenant_id)
        .await?;

    Ok(Json(map))
}
