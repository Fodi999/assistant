//! B-Rep topology: vertex, edge, coedge, loop, face, shell, solid, body.
pub mod adjacency;
pub mod body;
pub mod coedge;
pub mod edge;
pub mod face;
pub mod ids;
#[path = "loop.rs"]
pub mod r#loop;
pub mod orientation;
pub mod shell;
pub mod solid;
pub mod validation;
pub mod vertex;

pub use body::Body;
pub use coedge::CoEdge;
pub use edge::Edge;
pub use face::Face;
pub use ids::*;
pub use r#loop::Loop;
pub use shell::Shell;
pub use solid::Solid;
pub use vertex::Vertex;

