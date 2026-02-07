use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantCommand {
    StartInventory,
    AddProduct,
    FinishInventory,
    CreateRecipe,
    FinishRecipes,
    CreateDish,
    FinishDishes,
    ViewReport,
}
