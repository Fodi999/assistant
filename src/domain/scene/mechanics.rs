//! Per-entity visual mechanics — pulse, glow, shake, fade, etc.
//!
//! These are *visual* only: they don't change domain state. The frontend
//! plays them as animations. Business mechanics (moveToRiskZone, fifo, …)
//! are computed at scene-build time and reflected as different positions /
//! themes / actions in the resulting entity.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MechanicKind {
    /// Slow rhythmic emissive pulse — used for risk / expired items.
    Pulse,
    /// Constant strong emissive — used for highlighted zones.
    Glow,
    /// Brief horizontal shake — used to flag a problem on first appearance.
    Shake,
    /// Fade entity out (e.g. after write-off).
    FadeOut,
    /// Animate position to a target zone (rebalancing).
    MoveToZone,
    /// Snap to a layout slot with spring easing.
    SnapToSlot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MechanicTrigger {
    /// Always running while the entity exists.
    Always,
    /// Plays once when entity enters the scene.
    OnAppear,
    /// Plays once when entity leaves.
    OnDisappear,
    /// Plays on hover.
    OnHover,
    /// Plays on selection.
    OnSelect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityMechanic {
    pub kind: MechanicKind,
    pub trigger: MechanicTrigger,
    /// 0..1 amplitude / strength of the effect.
    pub intensity: f32,
    /// Hz / playback rate hint. `None` = prefab default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    /// Total duration in milliseconds. `None` = continuous.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,
}

impl EntityMechanic {
    pub fn pulse(intensity: f32, speed: f32) -> Self {
        Self {
            kind: MechanicKind::Pulse,
            trigger: MechanicTrigger::Always,
            intensity,
            speed: Some(speed),
            duration_ms: None,
        }
    }

    pub fn glow(intensity: f32) -> Self {
        Self {
            kind: MechanicKind::Glow,
            trigger: MechanicTrigger::Always,
            intensity,
            speed: None,
            duration_ms: None,
        }
    }
}
