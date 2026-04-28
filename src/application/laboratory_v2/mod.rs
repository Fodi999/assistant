//! Laboratory **v2** — Photo → 3D Model pipeline.
//!
//! This module replaces the food-tech analysis pipeline (project / ingredients
//! / steps / analyze / scenes) with a much simpler product flow:
//!
//!   1. user uploads a photo            → `laboratory_images`
//!   2. backend analyses it (Gemini Vision) → `Product3DSpec`
//!   3. backend generates an OBJ/GLB    → `laboratory_3d_assets`
//!   4. frontend renders the model in a 3D viewer
//!
//! Layout (Clean Architecture):
//!
//!   application/laboratory_v2/
//!     ├── mod.rs         (this file — re-exports)
//!     ├── models.rs      (LaboratoryImage, Laboratory3DAsset, Product3DSpec, …)
//!     └── service.rs     (LaboratoryV2Service — use cases)
//!
//!   infrastructure/persistence/
//!     └── laboratory_v2_repository.rs  (Postgres I/O)
//!
//!   interfaces/http/
//!     └── laboratory_v2.rs             (Axum handlers)
//!
//! Old `application::laboratory` keeps running side-by-side until v2 is
//! stable in production. No old code is touched in PR #1.

pub mod models;
pub mod service;

pub use models::*;
pub use service::LaboratoryV2Service;
