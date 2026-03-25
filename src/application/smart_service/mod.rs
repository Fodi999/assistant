//! SmartService v3 — Culinary Intelligence Decision Engine
//!
//! `POST /api/smart/ingredient`   — aggregates data from DB + pure domain engines
//!                                   into a single intelligent response.
//! `GET  /api/smart/autocomplete` — fast search-as-you-type for ingredients.
//!
//! v3 improvements:
//! 1. Goal Engine: typed Goal enum affects suggestion scoring, diagnostics, explain
//! 2. Feedback Loop: diagnostics issues → synthetic suggestion candidates
//! 3. Confidence System: data-completeness scores (overall, nutrition, pairings, flavor)
//! 4. Next Actions: actionable [{type, ingredient, reason, priority}] from issues + goal
//! 5. Session: session_id for continuity, stores recent ingredients in-memory
//!
//! Design:
//! - Deterministic only (no LLM, no external APIs)
//! - Composes FlavorGraph + SuggestionEngine + RuleEngine + Nutrition
//! - In-memory TTL cache (key = "slug|state|extras|goal|lang")
//! - In-memory session store (30min TTL, 5000 max)
//! - Target: <100ms without cache, <1ms with cache
//! - Clean Architecture: isolated from admin / Telegram logic

pub mod context;
pub mod response;
pub mod pipeline;
pub mod cache;
pub mod service;
pub mod recipe_builder;
pub mod culinary_rules;

pub use context::CulinaryContext;
pub use context::{MealType, Diet};
pub use response::SmartResponse;
pub use service::SmartService;
