use super::{step::AssistantStep, command::AssistantCommand};

pub fn next_step(
    current: AssistantStep,
    command: &AssistantCommand,
) -> AssistantStep {
    match (current, command) {
        (AssistantStep::Start, AssistantCommand::StartInventory)
            => AssistantStep::InventorySetup,

        (AssistantStep::InventorySetup, AssistantCommand::FinishInventory)
            => AssistantStep::RecipeSetup,

        (AssistantStep::RecipeSetup, AssistantCommand::FinishRecipes)
            => AssistantStep::DishSetup,

        (AssistantStep::DishSetup, AssistantCommand::FinishDishes)
            => AssistantStep::Report,

        (AssistantStep::Report, AssistantCommand::ViewReport)
            => AssistantStep::Completed,

        // Невозможные переходы просто игнорируются
        _ => current,
    }
}
