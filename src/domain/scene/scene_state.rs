//! `SceneState` — the top-level snapshot the frontend renders.
//!
//! Game-like analogy: this is one frame of the simulated world. Tick
//! increases on every rebuild so the renderer can spot stale snapshots.

use serde::{Deserialize, Serialize};

use super::camera::SceneCamera;
use super::entity::SceneEntity;
use super::environment::SceneEnvironment;
use super::mechanics::EntityMechanic;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SceneMode {
    Inventory,
    Recipes,
    Dishes,
    Laboratory,
}

/// Pre-formatted, locale-aware HUD strings. Backend formats currency etc.
/// so the frontend never has to know about i18n number rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneHud {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_value_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiring_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_stock_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneState {
    pub scene_id: String,
    pub mode: SceneMode,
    /// Monotonic tick used by frontend to detect stale snapshots.
    pub tick: i64,
    /// ISO-8601 (RFC3339) timestamp of generation.
    pub generated_at: String,
    pub camera: SceneCamera,
    /// Optional global lighting / fog / background hints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<SceneEnvironment>,
    pub hud: SceneHud,
    pub entities: Vec<SceneEntity>,
    /// Scene-wide mechanics (focusCamera on first risk, etc.).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mechanics: Vec<EntityMechanic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_entity_id: Option<String>,
}

// ── Commands / Events (PR4 — wired by `POST /api/scenes/inventory/commands`) ─

use super::action::EntityAction;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SceneCommand {
    SelectEntity {
        #[serde(rename = "entityId")]
        entity_id: Option<String>,
    },
    RequestAction {
        #[serde(rename = "entityId")]
        entity_id: String,
        action: EntityAction,
    },
    FocusRisks,
    ResetCamera,
    SetViewMode {
        mode: SceneMode,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SceneEvent {
    EntitySelected {
        #[serde(rename = "entityId")]
        entity_id: Option<String>,
    },
    EntityHovered {
        #[serde(rename = "entityId")]
        entity_id: Option<String>,
    },
    CameraChanged,
    RequiresConfirmation {
        #[serde(rename = "planId")]
        plan_id: String,
        #[serde(rename = "planType")]
        plan_type: String,
        message: String,
    },
    SceneInvalidated {
        tick: i64,
    },
}
