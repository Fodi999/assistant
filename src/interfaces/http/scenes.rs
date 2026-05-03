//! HTTP handlers for game-like scene snapshots.
//!
//! `GET /api/scenes/inventory` — returns a `SceneState` for the
//! authenticated tenant. The frontend's `VisualSceneRenderer` consumes
//! this directly; no client-side scene assembly required.
//!
//! Future endpoints (PR3+):
//!   • `GET  /api/scenes/recipes`
//!   • `GET  /api/scenes/dishes`
//!   • `GET  /api/scenes/laboratory`
//!   • `POST /api/scenes/inventory/commands`  (SceneCommand)

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::application::scenes::InventorySceneService;
use crate::domain::scene::SceneState;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

#[derive(Debug, Deserialize)]
pub struct InventorySceneQuery {
    /// Optional pre-selected entity id (e.g. "product_<uuid>").
    /// Backend echoes it back so HUD/camera can highlight on first paint.
    #[serde(rename = "selectedEntityId")]
    pub selected_entity_id: Option<String>,
}

/// `GET /api/scenes/inventory`
///
/// Returns the authoritative `SceneState` for the caller's tenant.
/// JSON shape mirrors `blog/components/visual/sceneTypes.ts` (camelCase).
pub async fn get_inventory_scene(
    State(service): State<Arc<InventorySceneService>>,
    auth: AuthUser,
    Query(params): Query<InventorySceneQuery>,
) -> Result<Json<SceneState>, AppError> {
    let scene = service
        .build_scene(
            auth.user_id,
            auth.tenant_id,
            auth.language,
            params.selected_entity_id,
        )
        .await?;
    Ok(Json(scene))
}
