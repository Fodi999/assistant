//! Hard-Surface B-Rep-lite generators.
//!
//! Hard-surface objects require engineering precision:
//! flat planes, sharp edges, chamfers, slots, recesses, sockets.
//!
//! Pipeline for these objects:
//!   GeometricShell (f64 domain) → tessellate() → MeshBuilder → GLB
//! OR for purely prismatic shapes:
//!   rounded_rect_points → extrude_polygon → MeshBuilder → GLB
//!
//! NO food generators live here.

pub mod card;
pub mod dock;
pub mod sci_fi_card;

pub use card::generate_card;
pub use dock::generate_dock;
pub use sci_fi_card::generate_sci_fi_card;
