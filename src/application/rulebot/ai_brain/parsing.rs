//! JSON parsing — extract AiAction from raw LLM output.
//!
//! Handles markdown-wrapped JSON, partial responses, and other LLM quirks.

use super::tool_types::AiAction;
use super::response_helpers::truncate;

/// Parse AiAction from raw LLM JSON (tolerant of markdown wrappers).
pub(crate) fn parse_ai_action(raw: &str) -> Result<AiAction, String> {
    // Try direct parse
    if let Ok(action) = serde_json::from_str::<AiAction>(raw) {
        return Ok(action);
    }

    // Try extracting JSON from markdown/surrounding text
    let cleaned = raw.trim();
    let json_str = if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            &cleaned[start..=end]
        } else {
            cleaned
        }
    } else {
        cleaned
    };

    serde_json::from_str::<AiAction>(json_str)
        .map_err(|e| format!("Failed to parse AI action: {} | raw: {}", e, truncate(raw, 200)))
}
