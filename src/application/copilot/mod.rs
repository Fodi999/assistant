//! Copilot — главный LLM Brain над всеми 8 ботами.
//!
//! Architecture:
//!   POST /api/copilot/message
//!     → CopilotEngine::handle_message(context, message)
//!       → billing check (UsageService)
//!       → Planner (LLM) → ToolPlan
//!       → SafetyLayer::validate(plan)
//!       → ToolExecutor::run_read_tools(plan)
//!       → build ActionPlan (write tools → requires_confirmation)
//!       → AuditLog::record(pending)
//!       → CopilotResponse { answer, action_plan, actions_cost, ... }
//!
//!   POST /api/copilot/actions/{action_id}/confirm
//!     → AuditLog::get(action_id) → ActionPlan
//!     → SafetyLayer::validate_write(plan)
//!     → ToolExecutor::run_write_tool(plan)
//!     → AuditLog::mark_executed(action_id)
//!
//! Modules:
//!   context        — CopilotContext, CopilotScreen
//!   tools          — CopilotTool enum (read/write split)
//!   planner        — LLM prompt → ToolPlan (JSON structured output)
//!   tool_executor  — dispatches tools → real backend services
//!   actions        — ActionPlan, ActionChange, ConfirmResult
//!   safety         — validates permissions, write guards
//!   billing        — AiFeature costs, integration with UsageService
//!   audit          — copilot_action_log table CRUD
//!   engine         — orchestrates all of the above

pub mod actions;
pub mod audit;
pub mod billing;
pub mod context;
pub mod engine;
pub mod planner;
pub mod safety;
pub mod tool_executor;
pub mod tools;

pub use actions::{ActionPlan, ConfirmResult, CopilotResponse};
pub use audit::CopilotAuditService;
pub use billing::AiFeature;
pub use context::{CopilotContext, CopilotScreen};
pub use engine::CopilotEngine;
