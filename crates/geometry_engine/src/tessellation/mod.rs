//! Tessellation: B-Rep → triangle mesh.
pub mod adaptive;
pub mod curve_tessellator;
pub mod face_tessellator;
pub mod mesh_with_metadata;
pub mod options;
pub mod tessellator;
pub mod triangulate_polygon;
pub mod triangulate_surface;

pub use options::TessOptions;
pub use tessellator::{tessellate, tessellate_body};
pub use mesh_with_metadata::{MeshWithMetadata, TriangleMeta};


