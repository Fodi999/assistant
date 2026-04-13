//! RuleBot — Culinary Intelligence Platform.
//!
//! Modules:
//!   - `orchestrator`       — routes `POST /public/tools/run` to engines
//!   - `chat_engine`        — routing layer for `POST /public/chat`
//!   - `chat_response`      — ChatResponse / Card / Suggestion types
//!   - `intent_router`      — NLP intent detection + language + health modifier
//!   - `response_builder`   — card assembly + suggestion generation
//!   - `response_templates` — localized human-readable text
//!   - `session_context`    — multi-turn session state
//!   - `chef_coach`         — motivational sous-chef messages
//!   - `meal_builder`       — dynamic meal combo assembler
//!   - `ai_brain`           — Layer 2 LLM fallback with tool calling

pub mod orchestrator;
pub mod chat_engine;
pub mod chat_response;
pub mod intent_router;
pub mod response_builder;
pub mod response_templates;
pub mod session_context;
pub mod chef_coach;
pub mod meal_builder;
pub mod ai_brain;
