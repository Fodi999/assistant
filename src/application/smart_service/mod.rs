//! SmartService v2 — Culinary Intelligence Aggregator
//!
//! `POST /api/smart/ingredient`   — aggregates data from DB + pure domain engines
//!                                   into a single intelligent response.
//! `GET  /api/smart/autocomplete` — fast search-as-you-type for ingredients.
//!
//! v2 improvements:
//! 1. State-aware: cooking state affects nutrition, flavor vector, suggestions, explain
//! 2. Equivalents: unit equivalents from density (g → ml, cups, tbsp, etc.)
//! 3. Processing state explain: detailed before/after nutrition comparison
//! 4. Autocomplete: lightweight slug/name/image search (<20ms)
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
