//! `geometry_engine` — собственный процедурный геометрический движок.
//!
//! # Структура
//!   * [`math`]          — Vec2/3, Point2/3, Matrix, Quaternion, Plane, Aabb, BVH
//!   * [`units`]         — grid (i32↔f64), mm ↔ metres, Angle
//!   * [`ids`]           — стабильные ID для топологии и истории
//!   * [`curves`]        — параметрические 3D-кривые (Line, Circle, Arc, NURBS)
//!   * [`surfaces`]      — параметрические поверхности (Plane, Cylinder, NURBS)
//!   * [`nurbs`]         — knot vectors, basis functions, fitting
//!   * [`topology`]      — B-Rep структуры (Vertex/Edge/Face/Shell/Solid/Body)
//!   * [`brep`]          — конкретное хранилище и валидация B-Rep
//!   * [`profile`]       — 2D-профили для extrude/lathe
//!   * [`ops`]           — extrude, lathe, sweep, loft, offset, draft, thicken
//!   * [`boolean`]       — union/subtract/intersect/imprint
//!   * [`fillet`]        — fillet, chamfer, variable-radius
//!   * [`shelling`]      — offset surface, thicken, hollow
//!   * [`tessellation`]  — B-Rep → triangle mesh
//!   * [`mesh`]          — Mesh, GpuMesh, normals, weld, OBJ/STL/glTF
//!   * [`selection`]     — picking (ray-mesh, ray-brep), selection sets
//!   * [`history`]       — параметрическая история, rebuild, undo/redo
//!   * [`diagnostics`]   — validation, healing, отчёты
//!   * [`import_export`] — OBJ, STL, glTF, SVG, DXF, STEP, brep-json
//!   * [`wasm`]          — wasm_bindgen обёртки (feature = "wasm")
//!
//! # Точность
//! Все вычисления — `Real = f64`. GPU-выход — `GpuReal = f32` (только в `GpuMesh`).

#![forbid(unsafe_code)]
#![allow(clippy::module_inception)]

pub mod boolean;
pub mod brep;
pub mod curves;
pub mod diagnostics;
pub mod fillet;
pub mod history;
pub mod ids;
pub mod import_export;
pub mod math;
pub mod mesh;
pub mod nurbs;
pub mod ops;
pub mod profile;
pub mod selection;
pub mod shelling;
/// 2D parametric sketcher — constraints, solver, profile detection, sketch→solid bridge.
/// This is the single source of truth for all 2D CAD operations.
pub mod sketch;
pub mod surfaces;
/// Native CAD tool implementations: rect, circle, copy, edge_extrude, tool FSM.
pub mod tools;
pub mod tessellation;
pub mod topology;
pub mod units;

#[cfg(feature = "wasm")]
pub mod wasm;

// ── Flat re-exports for convenience `use geometry_engine::*` ────────────────
pub use math::{
    Aabb, GpuReal, Line, Matrix3, Matrix4, Plane, Point3, Quaternion, Ray, Real,
    Segment, Tolerance, Transform, Vec2, Vec3, PI, TAU,
};
pub use mesh::{GeometryError, GpuMesh, Material, MaterialGroup, Mesh, MeshPart};
pub use ops::extrude::{extrude_polygon, extrude_polygon_brep, ExtrudeBrepResult, ExtrudeOptions, Point2};
pub use ops::lathe::lathe_profile;
pub use profile::{LathePoint, LatheProfile};
pub use tessellation::{tessellate_body, MeshWithMetadata, TessOptions, TriangleMeta};
pub use selection::{pick_face, pick_face_from_many, FacePickResult, Hit, ray_triangle};
