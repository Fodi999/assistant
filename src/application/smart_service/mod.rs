//! SmartService — Culinary Intelligence Aggregator
//!
//! `POST /api/smart/ingredient` — aggregates data from DB + pure domain engines
//! into a single intelligent response for any ingredient + cooking state.
//!
//! Design:
//! - Deterministic only (no LLM, no external APIs)
//! - Composes FlavorGraph + SuggestionEngine + RuleEngine + Nutrition
//! - In-memory TTL cache (key = "slug|state|extras")
//! - Target: <100ms without cache, <1ms with cache
//! - Clean Architecture: isolated from admin / Telegram logic

pub mod context;
pub mod response;
pub mod pipeline;
pub mod cache;
pub mod service;

pub use context::CulinaryContext;
pub use response::SmartResponse;
pub use service::SmartService;
