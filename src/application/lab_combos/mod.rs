// ─── Lab Combos — DDD Module (v2) ───────────────────────────────────────────
//
// Architecture (Domain-Driven Design):
//
// ┌─────────────────────────────────────────────────────────────┐
// │  DOMAIN LAYER                                                │
// │  recipe.rs          — Recipe aggregate + invariants           │
// │  dish_classifier.rs — DishType enum + classify_dish()        │
// │  nutrition.rs       — USDA calculator (single source of truth)│
// ├─────────────────────────────────────────────────────────────┤
// │  APPLICATION LAYER                                           │
// │  service.rs         — thin facade (public API)               │
// │  generator.rs       — AI pipeline (classify→prompt→validate) │
// │  repository.rs      — pure DB CRUD                           │
// ├─────────────────────────────────────────────────────────────┤
// │  INFRASTRUCTURE                                              │
// │  chef_prompt.rs     — Gemini prompt builder                  │
// │  recipe_validator.rs— fix prompt builder (legacy compat)     │
// │  seo/               — decomposed SEO generators              │
// │  metrics.rs         — observability (counters + structured log)│
// │  types.rs           — shared DTOs                            │
// └─────────────────────────────────────────────────────────────┘

// ── Domain ──────────────────────────────────────────────────────────────────
pub mod dish_classifier;
pub mod nutrition;
pub mod recipe;

// ── Application ─────────────────────────────────────────────────────────────
pub mod generator;
pub mod repository;
#[path = "service_new.rs"]
pub mod service;

// ── Infrastructure ──────────────────────────────────────────────────────────
pub mod chef_prompt;
pub mod metrics;
pub mod recipe_validator;
pub mod seo;
pub mod types;

// ── Legacy compatibility (enrichment.rs + templates.rs kept for reference) ──
// These are superseded by generator.rs and seo/ respectively,
// but kept to avoid breaking any stray imports during transition.
#[allow(dead_code)]
mod enrichment;
#[allow(dead_code)]
mod templates;

// ── Re-exports (public API) ─────────────────────────────────────────────────
pub use service::LabComboService;
pub use types::{
    combo_slug, GenerateComboRequest, LabComboPage, LabComboSitemapEntry, ListCombosQuery,
    PublicComboSlugQuery, RelatedCombo, RelatedCombosQuery, UpdateComboRequest,
};
