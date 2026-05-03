//! Gameplay descriptor — what the user can do with an entity, plus the
//! reverse-link to the underlying domain row (so the shell can open
//! detail panels / Copilot context).

use serde::{Deserialize, Serialize};

use super::action::EntityAction;

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DomainKind {
    Inventory,
    Recipes,
    Dishes,
    Laboratory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDataRef {
    pub domain: DomainKind,
    pub entity_id: String,
}
