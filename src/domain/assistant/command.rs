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
    CreateDish(CreateDishPayload),
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
    /// Product receipt/purchase date (дата поступления)
    #[serde(with = "time::serde::rfc3339")]
    pub received_at: OffsetDateTime,
    /// Expiration date (дата просрочки)
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

/// Payload for creating dish
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDishPayload {
    /// Recipe ID
    pub recipe_id: Uuid,
    /// Dish name
    pub name: String,
    /// Selling price in cents
    pub selling_price_cents: i32,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
