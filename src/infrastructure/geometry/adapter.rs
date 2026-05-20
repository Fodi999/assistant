//! Adapter — bridges `geometry_engine` crate into the infrastructure layer.
//!
//! Все модули `src/infrastructure/geometry/` используют этот файл как
//! единую точку импорта вместо прямых ссылок на внутренние пути крейта.
//!
//! Использование внутри `src/`:
//! ```rust
//! use crate::infrastructure::geometry::adapter::*;
//! ```

pub use geometry_engine::math::{Aabb, Plane, Tolerance, Vec2, Vec3};
pub use geometry_engine::units::{grid_to_f64, mm_to_m, m_to_mm, GRID_SIZE};
pub use geometry_engine::mesh::{
    validate_mesh, GeometryError, Material, MaterialGroup, Mesh, MeshPart,
    hex_to_rgb, export_obj, ObjExport, recalculate_smooth_normals, weld_vertices,
};
pub use geometry_engine::profile::{
    LatheProfile, LathePoint,
    signed_area_2d, is_ccw, ensure_ccw,
    validate_profile_3d,
};
pub use geometry_engine::ops::{
    extrude::{extrude_polygon, ExtrudeOptions, Point2},
    lathe::lathe_profile,
    bevel::rounded_rect_points,
    csg::{Aabb as CsgAabb, subtract_box, subtract_cylinder},
};

/// Bridge: GeometryError → AppError для использования с `?` в application layer.
impl From<GeometryError> for crate::shared::AppError {
    fn from(e: GeometryError) -> Self {
        crate::shared::AppError::internal(format!("geometry kernel: {e}"))
    }
}
