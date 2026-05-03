//! SceneState contract — mirror of `blog/components/visual/sceneTypes.ts`.
//!
//! Keep field naming and the discriminated unions in sync. When you add a
//! new variant on either side, add it on the other.

use serde::{Deserialize, Serialize};

// ── Primitives ───────────────────────────────────────────────────────────────

pub type Vec3 = [f32; 3];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Transform {
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

// ── Entity classification ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    StorageZone,
    InventoryProduct,
    Label,
    Effect,
    Marker,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GeometryKind {
    StorageRoom,
    ProductCard,
    ZoneLabel,
    RiskMarker,
    Container,
    Tray,
    Bottle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MaterialTheme {
    Cold,
    Dry,
    Freezer,
    Risk,
    Ok,
    Warning,
    Critical,
    Expired,
    Neutral,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityAction {
    WriteOff,
    UseToday,
    OpenDetails,
    Restock,
    Inspect,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SceneMode {
    Inventory,
    Recipes,
    Dishes,
    Laboratory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CameraPreset {
    Overview,
    Risk,
    Zone,
    Focused,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DomainKind {
    Inventory,
    Recipes,
    Dishes,
    Laboratory,
}

// ── Composite shapes ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityGeometry {
    pub kind: GeometryKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityMaterial {
    pub theme: MaterialTheme,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub emissive: f32,
    pub opacity: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Stable asset key resolved against frontend `assetRegistry`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_icon: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub badges: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityGameplay {
    pub selectable: bool,
    pub hoverable: bool,
    #[serde(default)]
    pub actions: Vec<EntityAction>,
    /// Foreign key into the underlying domain row (inventory_batch.id, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDataRef {
    pub domain: DomainKind,
    pub entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneEntity {
    pub id: String,
    pub entity_type: EntityType,
    pub transform: Transform,
    pub geometry: EntityGeometry,
    pub material: EntityMaterial,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<EntityContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gameplay: Option<EntityGameplay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<EntityDataRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneCamera {
    pub preset: CameraPreset,
    pub position: Vec3,
    pub target: Vec3,
    pub fov: f32,
}

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
    pub hud: SceneHud,
    pub entities: Vec<SceneEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_entity_id: Option<String>,
}

// ── Commands / Events (reserved for PR4 — not yet wired) ────────────────────

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
