//! Inventory mechanics — pulse / glow effects per severity.
//!
//! These are *visual* mechanics: they instruct the renderer to animate
//! emissive intensity. Domain mechanics (FIFO ordering, moveToRiskZone)
//! are already baked into the layout/material decisions.

use crate::domain::scene::{EntityMechanic, MaterialTheme};

/// Decide which mechanics a product card should run, based on its severity.
pub fn mechanics_for_theme(theme: MaterialTheme) -> Vec<EntityMechanic> {
    match theme {
        MaterialTheme::Expired => vec![EntityMechanic::pulse(0.9, 1.6)],
        MaterialTheme::Critical => vec![EntityMechanic::pulse(0.7, 1.2)],
        MaterialTheme::Warning => vec![EntityMechanic::pulse(0.4, 0.8)],
        _ => vec![],
    }
}

/// Decide which mechanics a zone should run.
pub fn zone_mechanics(theme: MaterialTheme) -> Vec<EntityMechanic> {
    match theme {
        MaterialTheme::Risk => vec![EntityMechanic::glow(0.5)],
        _ => vec![],
    }
}
