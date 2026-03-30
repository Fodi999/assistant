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

pub mod types;
pub mod goal;
pub mod strategy;
pub mod resolver;
pub mod gemini;
pub mod service;

// Re-exports for external consumers
pub use types::{MealPlan, MealVariant, MealIngredient, PlanRequest};
pub use goal::normalize_for_cache;
pub use service::SousChefPlannerService;
