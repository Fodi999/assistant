use serde::Serialize;
use super::step::AssistantStep;

#[derive(Debug, Serialize)]
pub struct AssistantAction {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub actions: Vec<AssistantAction>,
    pub step: AssistantStep,
    pub progress: u8,
}
