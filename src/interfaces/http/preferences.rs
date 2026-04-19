use crate::application::preferences_service::PreferencesService;
use crate::domain::user_preferences::UserPreferences;
use crate::interfaces::http::middleware::AuthUser;
use axum::{extract::State, http::StatusCode, Json};

/// GET /api/preferences
pub async fn get_preferences(
    auth_user: AuthUser,
    State(svc): State<PreferencesService>,
) -> Result<Json<UserPreferences>, crate::shared::AppError> {
    let prefs = svc.get(auth_user.user_id).await?;
    Ok(Json(prefs))
}

/// PUT /api/preferences
pub async fn save_preferences(
    auth_user: AuthUser,
    State(svc): State<PreferencesService>,
    Json(prefs): Json<UserPreferences>,
) -> Result<StatusCode, crate::shared::AppError> {
    svc.save(auth_user.user_id, &prefs).await?;
    Ok(StatusCode::OK)
}
