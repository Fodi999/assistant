//! RuleBot — the orchestrator + ChefOS chat engine.
//!
//! Two entry points:
//!   - `POST /public/tools/run`  → RuleBot orchestrator (tool dispatch)
//!   - `POST /public/chat`       → ChatEngine (intent-based chat)
//!
//! ```json
//! POST /public/tools/run
//! {
//!   "tool": "convert",
//!   "params": { "value": 100, "from": "g", "to": "oz" }
//! }
//! ```
//!
//! Response is always wrapped in `ToolResponse<T>`.

pub mod orchestrator;
pub mod intent_router;
pub mod chat_engine;
pub mod chat_response;
pub mod session_context;
pub mod response_builder;
pub mod response_templates;
pub mod ai_brain;
