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
pub mod intent_router;    // 🧠 Score-based intent detection + multi-intent + modifiers
pub mod chat_response;    // 📦 Unified ChatResponse + Card types + reason/intents
pub mod chat_engine;      // 🔥 ChefOS Chat — handle_chat() logic
pub mod session_context;  // 🗂️  Per-session memory (client-side, stateless server)
