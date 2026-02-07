use serde::{Serialize, Deserialize};
use crate::shared::{Language, AssistantMessage, AssistantActionLabel, AssistantHint, translate_message, translate_action, translate_hint};
use super::response::{AssistantAction, AssistantResponse};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssistantStep {
    Start,
    InventorySetup,
    RecipeSetup,
    DishSetup,
    Report,
    Completed,
}

impl AssistantStep {
    pub fn progress(self) -> u8 {
        match self {
            Self::Start => 0,
            Self::InventorySetup => 25,
            Self::RecipeSetup => 50,
            Self::DishSetup => 75,
            Self::Report | Self::Completed => 100,
        }
    }
    
    pub fn to_response(self, language: Language) -> AssistantResponse {
        match self {
            Self::Start => AssistantResponse {
                message: translate_message(AssistantMessage::Welcome, language).to_string(),
                hint: None,
                warnings: vec![],
                actions: vec![AssistantAction {
                    id: "start_inventory".to_string(),
                    label: translate_action(AssistantActionLabel::AddProducts, language).to_string(),
                }],
                step: self,
                progress: self.progress(),
                dish_financials: None,
            },
            Self::InventorySetup => AssistantResponse {
                message: translate_message(AssistantMessage::InventorySetup, language).to_string(),
                hint: Some(translate_hint(AssistantHint::InventoryWhy, language).to_string()),
                warnings: vec![],
                actions: vec![
                    AssistantAction {
                        id: "add_products".to_string(),
                        label: translate_action(AssistantActionLabel::AddMoreProducts, language).to_string(),
                    },
                    AssistantAction {
                        id: "proceed_to_recipes".to_string(),
                        label: translate_action(AssistantActionLabel::ProceedToRecipes, language).to_string(),
                    },
                ],
                step: self,
                progress: self.progress(),
                dish_financials: None,
            },
            Self::RecipeSetup => AssistantResponse {
                message: translate_message(AssistantMessage::RecipeSetup, language).to_string(),
                hint: Some(translate_hint(AssistantHint::RecipeWhy, language).to_string()),
                warnings: vec![],
                actions: vec![
                    AssistantAction {
                        id: "add_recipe".to_string(),
                        label: translate_action(AssistantActionLabel::AddRecipe, language).to_string(),
                    },
                    AssistantAction {
                        id: "proceed_to_dishes".to_string(),
                        label: translate_action(AssistantActionLabel::ProceedToDishes, language).to_string(),
                    },
                ],
                step: self,
                progress: self.progress(),
                dish_financials: None,
            },
            Self::DishSetup => AssistantResponse {
                message: translate_message(AssistantMessage::DishSetup, language).to_string(),
                hint: Some(translate_hint(AssistantHint::DishWhy, language).to_string()),
                warnings: vec![],
                actions: vec![
                    AssistantAction {
                        id: "add_dish".to_string(),
                        label: translate_action(AssistantActionLabel::AddDish, language).to_string(),
                    },
                    AssistantAction {
                        id: "generate_report".to_string(),
                        label: translate_action(AssistantActionLabel::GenerateReport, language).to_string(),
                    },
                ],
                step: self,
                progress: self.progress(),
                dish_financials: None,
            },
            Self::Report => AssistantResponse {
                message: translate_message(AssistantMessage::Report, language).to_string(),
                hint: Some(translate_hint(AssistantHint::ReportWhy, language).to_string()),
                warnings: vec![],
                actions: vec![
                    AssistantAction {
                        id: "view_report".to_string(),
                        label: translate_action(AssistantActionLabel::ViewReport, language).to_string(),
                    },
                    AssistantAction {
                        id: "finish".to_string(),
                        label: translate_action(AssistantActionLabel::Finish, language).to_string(),
                    },
                ],
                step: self,
                progress: self.progress(),
                dish_financials: None,
            },
            Self::Completed => AssistantResponse {
                message: translate_message(AssistantMessage::Completed, language).to_string(),
                hint: None,
                warnings: vec![],
                actions: vec![AssistantAction {
                    id: "restart".to_string(),
                    label: translate_action(AssistantActionLabel::Restart, language).to_string(),
                }],
                step: self,
                progress: self.progress(),
                dish_financials: None,
            },
        }
    }
}

