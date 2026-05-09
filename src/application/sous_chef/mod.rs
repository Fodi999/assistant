//! Sous-Chef Planner — DDD module.
//!
//! Architecture: DB → IngredientCache (startup) → Planner (pure, 0 SQL) → Gemini (text only) → Cache
//!
//! Files:
//! - types.rs     — request/response DTOs
//! - goal.rs      — goal detection & cache key
//! - strategy.rs  — meal strategies with ingredient picks
//! - resolver.rs  — resolves strategies against IngredientCache
//! - gemini.rs    — LLM prompt builder & response parser
//! - service.rs   — orchestrator

pub mod gemini;
pub mod goal;
pub mod resolver;
pub mod service;
pub mod strategy;
pub mod types;

// Re-exports for external consumers
pub use goal::normalize_for_cache;
pub use service::SousChefPlannerService;
pub use types::{MealIngredient, MealPlan, MealVariant, PlanRequest};
