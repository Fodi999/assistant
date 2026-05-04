//! ChefOS Geometry Kernel (PR #10).
//!
//! A small CAD-like core for procedural food / product modelling.
//!
//! This module is **purely additive**: it does not touch the existing
//! generators (`sauce_in_bowl`, `bottled_sauce`, `jar_product`, `flat_card`).
//! Subsequent PRs will rewrite each generator on top of `lathe_profile` so
//! all product shapes share the same revolve-of-2D-profile pipeline.
//!
//! Layers:
//!   * [`math`]         — `Vec2` / `Vec3` with the few ops we actually need.
//!   * [`profile`]      — 2D radius/y profile to revolve around the Y axis.
//!   * [`mesh_builder`] — append-only builder over `Mesh` with material groups.
//!   * [`lathe`]        — `revolve` operation: profile + segments → MeshPart.
//!   * [`normals`]      — recompute smooth per-vertex normals on a `Mesh`.
//!   * [`validate`]     — sanity-check a finished mesh before export.
//!
//! Design rules:
//!   * Y-up, metres, centred at origin (same as the existing generators).
//!   * No external dependencies — pure Rust + `serde_json::Value` only at the
//!     edges (we reuse `Mesh`/`Material` from the parent geometry module).
//!   * Every public type is `Debug + Clone`; geometry data is plain `Vec`s
//!     (no GPU types, no allocators, no unsafe).

pub mod lathe;
pub mod math;
pub mod mesh_builder;
pub mod normals;
pub mod profile;
pub mod quality;
pub mod validate;
pub mod disk;
pub mod decal;
pub mod extrude;
pub mod precision;
pub mod rounded;
pub mod csg;

pub use decal::{cylindrical_band, flat_patch};
pub use disk::{disk_fan_down, disk_fan_up};
pub use extrude::{extrude_polygon, ExtrudeOptions, Point2};
pub use lathe::{lathe_profile, MeshPart};
pub use math::{Vec2, Vec3};
pub use mesh_builder::MeshBuilder;
pub use precision::tessellate;
pub use profile::{Profile, ProfilePoint};
pub use quality::GeometryQuality;
pub use rounded::rounded_rect_points;
pub use validate::{validate_mesh, GeometryError};
