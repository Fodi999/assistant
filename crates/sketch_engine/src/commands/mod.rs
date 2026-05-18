//! # commands
//!
//! Pure command functions: every function takes a request and returns a result
//! with the updated SketchGraph. Nothing is mutated in place.

pub mod add_edge;
pub mod add_point;
pub mod move_point;
mod result;

pub use add_edge::{apply_add_edge, AddEdgeRequest};
pub use add_point::{apply_add_point, AddPointRequest};
pub use move_point::{apply_move_point, MovePointRequest};
pub use result::{PointRefOrGrid, SketchCommandResult};
