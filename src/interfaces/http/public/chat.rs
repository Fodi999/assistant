//! HTTP handler for POST /api/chat — ChefOS authenticated chat.
//!
//! Pay-per-action billing:
//!   • Each chat turn debits 1 daily-free `AiChat` action OR `cost_chat`
//!     purchased actions (server-truth via `UsageService::perform_action`).
//!   • `user_id` is derived from the JWT (`AuthUser`) — never from the body.
//!     This eliminates spoofing and prevents anonymous abuse of LLM costs.
//!   • On quota exceeded → HTTP 402 Payment Required + usage snapshot
//!     so the client can trigger a paywall / IAP flow.
//!
//! Request:
//!   { "input": "что полезного поесть" }
//!   { "input": "а сколько в нём калорий?", "context": { "last_product_slug": "salmon", "turn_count": 1 } }
//!
//! Response 200:
//!   { "text": "...", "cards": [...], "intent": "...", "context": {...},
//!     "usage": { "chats_left": 4, "purchased_actions": 18, ... } }
//!
//! Response 402:
//!   { "error": "quota_exceeded", "reason": "DailyLimitReached",
//!     "message": "...", "usage": { "chats_left": 0, "purchased_actions": 0, ... } }

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::rulebot::chat_engine::ChatEngine;
use crate::application::rulebot::chat_response::ChatResponse;
use crate::application::rulebot::session_context::SessionContext;
use crate::application::rulebot::intent_router::{detect_language, parse_input_with_context, DialogContext};
use crate::application::usage_service::UsageService;
use crate::domain::usage::{ActionSource, ActionType};
use crate::interfaces::http::middleware::AuthUser;

/// Combined state for the authenticated chat endpoint:
/// engine for the LLM/rule pipeline + usage service for token billing.
#[derive(Clone)]
pub struct AuthChatState {
    pub engine: Arc<ChatEngine>,
    pub usage: UsageService,
}

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

/// POST /api/chat — authenticated, billed.
pub async fn chat_handler(
    State(state): State<AuthChatState>,
    auth: AuthUser,
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

    // ── Token / action billing — server-truth ─────────────────────────────
    // Try to debit one AiChat action BEFORE running the (expensive) engine.
    // Free tier first; falls back to purchased balance; otherwise 402.
    let billing = match state.usage.perform_action(auth.user_id.clone(), ActionType::AiChat).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("usage.perform_action(AiChat) failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "usage_service_unavailable" })),
            );
        }
    };

    if !billing.allowed {
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(serde_json::json!({
                "error": "quota_exceeded",
                "reason": billing.reason.map(|r| format!("{:?}", r)),
                "message": billing.message,
                "usage": billing.usage,
            })),
        );
    }

    // Run the chat pipeline with the authenticated user id.
    let response: ChatResponse = state
        .engine
        .handle_chat_with_user(&input, &req.context, Some(auth.user_id.clone()))
        .await;

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
        // Surface remaining quota so the UI can render counters / nudges
        // without an extra round-trip to /api/usage/today.
        map.insert("usage".to_string(), serde_json::to_value(&billing.usage).unwrap_or_default());
        map.insert(
            "billing_source".to_string(),
            serde_json::Value::String(match billing.source {
                ActionSource::FreeTier => "free_tier".to_string(),
                ActionSource::Purchased => "purchased".to_string(),
                ActionSource::Denied => "denied".to_string(),
            }),
        );
    }

    (StatusCode::OK, Json(body))
}

// ══════════════════════════════════════════════════════════════════════════════
// POST /api/chat/event — telemetry ingestion (authenticated)
// ══════════════════════════════════════════════════════════════════════════════

use crate::application::chat_events_service::{ChatEvent, ChatEventsService};

#[derive(Debug, Deserialize)]
pub struct ChatEventRequest {
    #[serde(flatten)]
    pub event: ChatEvent,
}

/// POST /api/chat/event
///
/// Fire-and-forget telemetry endpoint. `user_id` is derived from the JWT,
/// never from the body. Always returns 202 Accepted on valid payload;
/// DB errors are logged server-side but never surfaced because telemetry
/// must not break the chat flow.
pub async fn chat_event_handler(
    State(events): State<Arc<ChatEventsService>>,
    auth: AuthUser,
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

    // Fire-and-forget: we swallow errors on purpose so telemetry never
    // becomes a user-visible failure mode.
    if let Err(e) = events.record(Some(auth.user_id), req.event).await {
        tracing::warn!("chat_event insert failed: {e}");
    }

    (StatusCode::ACCEPTED, Json(serde_json::json!({ "ok": true })))
}
