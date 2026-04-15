//! Dish Schema — Gemini LLM call + JSON parsing.
//!
//! Responsibilities:
//!   - Build prompt for Gemini (minimal: dish name + ingredient slugs)
//!   - Parse JSON response, strip markdown fences
//!   - Return `DishSchema { dish, dish_local, items }`
//!
//! Zero business logic — just IO + parsing.

use serde::{Deserialize, Serialize};

use crate::infrastructure::llm_adapter::LlmAdapter;
use super::intent_router::ChatLang;
use super::response_builder::HealthGoal;

// ── Types ────────────────────────────────────────────────────────────────────

/// Minimal schema from Gemini — just dish name + ingredient slugs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DishSchema {
    pub dish: String,
    #[serde(default)]
    pub dish_local: Option<String>,
    pub items: Vec<String>,
}

// ── Gemini call (minimal — 50-100 tokens) ────────────────────────────────────

/// Ask Gemini for ONLY the dish name + ingredient list. Nothing else.
pub async fn ask_gemini_dish_schema(
    llm: &LlmAdapter,
    user_input: &str,
    lang: ChatLang,
    goal: HealthGoal,
) -> Result<DishSchema, String> {
    let lang_label = match lang {
        ChatLang::Ru => "Russian",
        ChatLang::En => "English",
        ChatLang::Pl => "Polish",
        ChatLang::Uk => "Ukrainian",
    };

    let goal_hint = match goal {
        HealthGoal::LowCalorie  => "\nThis is a LOW-CALORIE / DIET version. Pick lean ingredients: vegetables, lean fish/poultry, skip heavy sauces and fatty items. No cherry, no cream, no sugar.",
        HealthGoal::HighProtein => "\nThis is a HIGH-PROTEIN version. Prefer protein-rich ingredients: chicken breast, beef, eggs, legumes.",
        HealthGoal::Balanced    => "",
    };

    let prompt = format!(
        r#"Identify the dish. Return ONLY JSON, no other text.
dish = English name. dish_local = name in {lang}. items = ingredient slugs (English, max 8).
Use only realistic, classic ingredients for this dish. No exotic or random items.
NEVER mix dessert ingredients (ice-cream, chocolate, candy, jam) with savory dishes (soup, stew, grill, pasta).{goal_hint}
If unknown: {{"dish":"unknown","items":[]}}

User: "{input}"

Example: {{"dish":"borscht","dish_local":"Борщ","items":["beet","cabbage","potato","carrot","onion","beef","garlic","tomato-paste"]}}"#,
        input = user_input,
        lang = lang_label,
        goal_hint = goal_hint,
    );

    let raw = llm
        .groq_raw_request_with_model(&prompt, 4000, "gemini-3-flash-preview")
        .await
        .map_err(|e| format!("Gemini error: {e}"))?;

    parse_dish_schema(&raw)
}

// ── Parsing ──────────────────────────────────────────────────────────────────

pub fn parse_dish_schema(raw: &str) -> Result<DishSchema, String> {
    let json_str = extract_json(raw)
        .ok_or_else(|| format!("No JSON found in: {}", &raw[..raw.len().min(100)]))?;

    let schema: DishSchema = serde_json::from_str(json_str)
        .map_err(|e| format!("JSON parse error: {e} — raw: {}", &raw[..raw.len().min(150)]))?;

    if schema.dish == "unknown" || schema.items.is_empty() {
        return Err("Gemini couldn't recognize this dish".into());
    }

    Ok(schema)
}

/// Extract first {...} from raw text (strips markdown fences etc.)
pub fn extract_json(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end >= start { Some(&raw[start..=end]) } else { None }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_schema() {
        let json = r#"{"dish":"borscht","dish_local":"Борщ","items":["beet","cabbage","potato","beef"]}"#;
        let s = parse_dish_schema(json).unwrap();
        assert_eq!(s.dish, "borscht");
        assert_eq!(s.items.len(), 4);
        assert_eq!(s.items[0], "beet");
    }

    #[test]
    fn parse_markdown_wrapped() {
        let raw = "```json\n{\"dish\":\"test\",\"items\":[\"a\",\"b\"]}\n```";
        let s = parse_dish_schema(raw).unwrap();
        assert_eq!(s.dish, "test");
        assert_eq!(s.items.len(), 2);
    }

    #[test]
    fn parse_unknown_dish_errors() {
        let json = r#"{"dish":"unknown","items":[]}"#;
        assert!(parse_dish_schema(json).is_err());
    }

    #[test]
    fn extract_json_from_markdown() {
        let raw = "Sure!\n```json\n{\"dish\":\"x\",\"items\":[]}\n```\nDone.";
        let j = extract_json(raw).unwrap();
        assert!(j.starts_with('{') && j.ends_with('}'));
    }
}
