// ─── Lab Combos — DDD Module ────────────────────────────────────────────────
//
// Architecture (Domain-Driven Design):
//
//   dish_classifier  — DishType enum + classify_dish() → DishProfile
//   recipe_validator — semantic validation using DishProfile constraints
//   chef_prompt      — Gemini prompt builder with dish type context
//   nutrition        — USDA nutrition calculator (single source of truth)
//   templates        — deterministic SEO text generation (title, h1, intro, FAQ, etc.)
//   enrichment       — AI SEO enrichment + validation + auto-fix pipeline
//   types            — shared data structures (LabComboPage, request/response types)
//   service          — LabComboService (CRUD + generation orchestrator)

pub mod chef_prompt;
pub mod dish_classifier;
pub mod enrichment;
pub mod nutrition;
pub mod recipe_validator;
pub mod service;
pub mod templates;
pub mod types;

// ── Re-exports (public API) ─────────────────────────────────────────────────
pub use service::LabComboService;
pub use types::{
    combo_slug, GenerateComboRequest, LabComboPage, LabComboSitemapEntry, ListCombosQuery,
    PublicComboSlugQuery, RelatedCombo, RelatedCombosQuery, UpdateComboRequest,
};
