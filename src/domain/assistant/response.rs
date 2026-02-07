use serde::Serialize;
use super::step::AssistantStep;

#[derive(Debug, Serialize)]
pub struct AssistantAction {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantWarning {
    pub level: WarningLevel,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WarningLevel {
    Critical,  // ❌ Expired products
    Warning,   // ⚠️ Expiring today/soon
    Info,      // ℹ️ General info
}

#[derive(Debug, Serialize)]
pub struct AssistantResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub warnings: Vec<AssistantWarning>,
    pub actions: Vec<AssistantAction>,
    pub step: AssistantStep,
    pub progress: u8,
}
