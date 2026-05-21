//! Boolean modelling (union, subtract, intersect, imprint).
pub mod classify;
pub mod curve_curve_intersection;
pub mod face_face_intersection;
pub mod imprint;
pub mod intersect;
pub mod operation;
pub mod rebuild_shell;
pub mod split_edges;
pub mod split_faces;
pub mod subtract;
pub mod union;
pub mod validation;

pub use operation::BooleanOp;
pub use classify::Classification;
pub use face_face_intersection::FaceFaceIntersection;
pub use rebuild_shell::FaceSpec;

/// Run a boolean operation between two [`crate::brep::BrepModel`]s.
pub use union::run as boolean_union;
pub use subtract::run as boolean_subtract;
pub use intersect::run as boolean_intersect;
