//! RuleBot — the orchestrator.
//!
//! Routes `POST /public/tools/run` requests to the correct engine.
//! Single entry point for the Culinary Intelligence Platform.
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
pub mod intent_router;        // 🧠 Score-based intent detection + multi-intent + modifiers
pub mod chat_response;        // 📦 Unified ChatResponse + Card types + reason/intents
pub mod response_templates;   // 📝 Text generation — "как это звучит"
pub mod response_builder;     // 🏗️  Response assembly — "как собрать ответ"
pub mod chat_engine;          // 🔥 ChefOS Chat — routing only: "что ответить"
pub mod session_context;      // 🗂️  Per-session memory (client-side, stateless server)
pub mod ai_brain;             // 🧠 Layer 2 — AI Brain with LLM tool-calling for complex queries
