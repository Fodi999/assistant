//! Parametric history / undo–redo.
pub mod dependency_graph;
pub mod document;
pub mod operation;
pub mod operation_id;
pub mod rebuild;
pub mod undo_redo;

pub use document::Document;
pub use operation::Operation;
pub use operation_id::OperationId;

