//! Validation, healing and diagnostic reports.
pub mod error;
pub mod gap_finder;
pub mod healing;
pub mod report;
pub mod self_intersection;
pub mod tiny_edge_finder;
pub mod validate_body;
pub mod validate_face;
pub mod validate_mesh;
pub mod validate_shell;

pub use error::{Diagnostic, Severity};
pub use report::Report;

