//! HTTP handler for POST /public/chat — ChefOS Chat Interface.
//!
//! Request:
//!   { "input": "что полезного поесть" }
//!   { "input": "а сколько в нём калорий?", "context": { "last_product_slug": "salmon", "turn_count": 1 } }
//!
//! Response:
//!   { "text": "...", "card": {...}, "intent": "healthy_product", "intents": [...],
//!     "reason": "protein: 31.0g/100g", "lang": "ru", "timing_ms": 12,
//!     "context": { "last_product_slug": "salmon", "turn_count": 2 } }

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::rulebot::chat_engine::ChatEngine;
use crate::application::rulebot::chat_response::ChatResponse;
use crate::application::rulebot::session_context::SessionContext;
use crate::application::rulebot::intent_router::{detect_language, parse_input_with_context, DialogContext};

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// Free-text user input.
    pub input: String,
    /// Optional session context from the previous turn (client-side storage).
    #[serde(default)]
    pub context: SessionContext,
}

#[derive(Debug, Serialize)]
pub struct ChatApiResponse {
    #[serde(flatten)]
    pub response: ChatResponse,
    /// Updated context — client must store and send back on next turn.
    pub context: SessionContext,
}

/// POST /public/chat
pub async fn chat_handler(
    State(engine): State<Arc<ChatEngine>>,
    Json(req): Json<ChatRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let input = req.input.trim().to_string();

    if input.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "input is required" })),
        );
    }

    if input.len() > 500 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "input too long (max 500 chars)" })),
        );
    }

    let response: ChatResponse = engine.handle_chat_with_context(&input, &req.context).await;

    // Build updated context for next turn
    let lang = detect_language(&input);
    let dialog_ctx = DialogContext {
        last_intent: req.context.last_intent,
        last_modifier: req.context.effective_modifier_opt(),
        turn_count: req.context.turn_count,
    };
    let parsed = parse_input_with_context(&input, &dialog_ctx);
    let modifier = req.context.effective_modifier(parsed.modifier);

    // Extract product slug from first product card in response
    let product_card = response.cards.iter().find_map(|c| {
        if let crate::application::rulebot::chat_response::Card::Product(p) = c { Some(p) } else { None }
    });
    let nutrition_card = response.cards.iter().find_map(|c| {
        if let crate::application::rulebot::chat_response::Card::Nutrition(n) = c { Some(n) } else { None }
    });

    let (product_slug, product_name) = match product_card {
        Some(p) => (Some(p.slug.clone()), Some(p.name.clone())),
        None => match nutrition_card {
            Some(n) => (req.context.last_product_slug.clone(), Some(n.name.clone())),
            None => (req.context.last_product_slug.clone(), req.context.last_product_name.clone()),
        },
    };

    // Collect all product card slugs shown this turn (for "а что ещё?" exclusion)
    let card_slugs: Vec<String> = response.cards.iter().filter_map(|c| {
        if let crate::application::rulebot::chat_response::Card::Product(p) = c { Some(p.slug.clone()) } else { None }
    }).collect();

    let updated_ctx = req.context.advance(
        response.intent,
        response.intents.clone(),
        product_slug,
        product_name,
        lang,
        modifier,
        card_slugs,
    );

    let mut body = serde_json::to_value(&response).unwrap_or_default();
    if let serde_json::Value::Object(ref mut map) = body {
        map.insert("context".to_string(), serde_json::to_value(&updated_ctx).unwrap_or_default());
    }

    (StatusCode::OK, Json(body))
}
