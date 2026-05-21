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

