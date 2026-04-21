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
    /// Optional user ID for personalized responses (from auth token).
    #[serde(default)]
    pub user_id: Option<String>,
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

    let response: ChatResponse = if let Some(ref uid_str) = req.user_id {
        if let Ok(uuid) = uuid::Uuid::parse_str(uid_str) {
            let user_id = crate::shared::UserId::from(uuid);
            engine.handle_chat_with_user(&input, &req.context, Some(user_id)).await
        } else {
            engine.handle_chat_with_context(&input, &req.context).await
        }
    } else {
        engine.handle_chat_with_context(&input, &req.context).await
    };

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
        // Step 3: detect category of THIS user query so the next turn knows
        // what was asked about (used for complementary suggestions).
        crate::application::rulebot::category_filter::detect_category(&input),
    );

    let mut body = serde_json::to_value(&response).unwrap_or_default();
    if let serde_json::Value::Object(ref mut map) = body {
        map.insert("context".to_string(), serde_json::to_value(&updated_ctx).unwrap_or_default());
    }

    (StatusCode::OK, Json(body))
}

// ══════════════════════════════════════════════════════════════════════════════
// POST /public/chat/event — telemetry ingestion (Step 4)
// ══════════════════════════════════════════════════════════════════════════════

use crate::application::chat_events_service::{ChatEvent, ChatEventsService};

#[derive(Debug, Deserialize)]
pub struct ChatEventRequest {
    /// Caller's authenticated user id — optional (anonymous chat allowed).
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(flatten)]
    pub event: ChatEvent,
}

/// POST /public/chat/event
///
/// Fire-and-forget telemetry endpoint. Always returns 202 Accepted on
/// valid payload; DB errors are logged server-side but never surfaced
/// because telemetry must not break the chat flow.
pub async fn chat_event_handler(
    State(events): State<Arc<ChatEventsService>>,
    Json(req): Json<ChatEventRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Whitelist accepted event types — keeps the table clean.
    const ALLOWED: &[&str] = &[
        "query_sent",
        "card_shown",
        "card_dismissed",
        "action_clicked",
        "suggestion_clicked",
    ];
    if !ALLOWED.contains(&req.event.event_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid event_type",
                "allowed": ALLOWED,
            })),
        );
    }

    let user_id = req
        .user_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .map(crate::shared::UserId::from);

    // Fire-and-forget: we swallow errors on purpose so telemetry never
    // becomes a user-visible failure mode.
    if let Err(e) = events.record(user_id, req.event).await {
        tracing::warn!("chat_event insert failed: {e}");
    }

    (StatusCode::ACCEPTED, Json(serde_json::json!({ "ok": true })))
}
