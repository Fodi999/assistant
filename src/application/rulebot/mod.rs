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
