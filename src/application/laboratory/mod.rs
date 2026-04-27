//! Laboratory module — public API types and use-case service.
//!
//! Layout follows Clean Architecture:
//!
//!   application/laboratory/
//!     ├── mod.rs                       (this file)
//!     ├── types.rs                     (DTOs: requests & responses)
//!     └── service.rs                   (LaboratoryService — use cases)
//!
//!   infrastructure/persistence/
//!     └── laboratory_repository.rs     (Postgres I/O)
//!
//!   interfaces/http/
//!     └── laboratory.rs                (Axum handlers)
//!
//! The service layer never speaks SQL directly — it goes through
//! `LaboratoryRepository`. Engines (process / shelf-life / flavor / nutrition)
//! and the catalog profile adapter will land in Step 3.

pub mod catalog_profile_adapter;
pub mod copilot_engine;
pub mod flavor_engine;
pub mod process_engine;
pub mod service;
pub mod shelf_life_engine;
pub mod types;

pub use catalog_profile_adapter::{
    CatalogProfileAdapter, LaboratoryCulinaryBehavior, LaboratoryIngredientProfile,
};
pub use service::LaboratoryService;
pub use types::*;
