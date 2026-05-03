//! Allowed user actions per entity.
//!
//! Backend decides which actions are allowed (e.g. a fresh product can
//! be `useToday`, an expired one only `writeOff`). Frontend renders
//! buttons; clicks dispatch through `POST /api/scenes/inventory/commands`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EntityAction {
    WriteOff,
    UseToday,
    OpenDetails,
    Restock,
    Inspect,
}
