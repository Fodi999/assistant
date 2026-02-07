use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "payload")]
pub enum AssistantCommand {
    StartInventory,
    AddProduct(AddProductPayload),
    FinishInventory,
    CreateRecipe,
    FinishRecipes,
    CreateDish,
    FinishDishes,
    ViewReport,
}

/// Payload for adding product to inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductPayload {
    /// Catalog ingredient ID
    pub catalog_ingredient_id: Uuid,
    /// Price per unit in cents
    pub price_per_unit_cents: i64,
    /// Quantity
    pub quantity: f64,
    /// Optional expiration date
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,
}
