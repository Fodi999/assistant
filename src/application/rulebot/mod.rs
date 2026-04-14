//! RuleBot — Culinary Intelligence Platform.
//!
//! DDD Architecture:
//!   - `intent_keywords`    → keyword data tables (WHAT words mean)
//!   - `goal_modifier`      → health modifier detection (WHICH goal)
//!   - `intent_router`      → intent scoring + routing (the BRAIN)
//!   - `chat_engine`        → routes `POST /public/chat` to handlers
//!   - `chat_response`      → ChatResponse / Card / Suggestion types
//!   - `response_builder`   → card assembly + suggestion generation
//!   - `response_templates` → localized human-readable text
//!   - `session_context`    → multi-turn session state
//!   - `chef_coach`         → motivational sous-chef messages
//!   - `meal_builder`       → dynamic meal combo assembler
//!   - `cooking_rules`      → DDD dish rules as data
//!   - `recipe_engine`      → recipe resolution engine
//!   - `ai_brain`           → Layer 2 LLM fallback with tool calling

pub mod orchestrator;
pub mod chat_engine;
pub mod chat_response;
pub mod intent_keywords;
pub mod goal_modifier;
pub mod intent_router;
pub mod response_builder;
pub mod response_templates;
pub mod session_context;
pub mod chef_coach;
pub mod meal_builder;
pub mod cooking_rules;
pub mod recipe_engine;
pub mod ai_brain;
