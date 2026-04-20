//! HTTP handler for POST /api/cook/suggestions — Smart recipe suggestions from inventory.
//!
//! Request: (no body needed — uses auth user's inventory)
//!
//! Response:
//!   {
//!     "can_cook": [...],
//!     "almost": [...],
//!     "strategic": [...]
//!   }

use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::application::cook_suggestions::{CookSuggestionService, CookSuggestionsResponse};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

/// POST /api/cook/suggestions
pub async fn cook_suggestions(
    State(service): State<Arc<CookSuggestionService>>,
    auth: AuthUser,
) -> Result<Json<CookSuggestionsResponse>, AppError> {
    let result = service
        .suggest(auth.user_id, auth.tenant_id, auth.language)
        .await?;
    Ok(Json(result))
}
