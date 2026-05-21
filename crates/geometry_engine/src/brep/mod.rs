//! Concrete B-Rep storage, building and validation.
pub mod builder;
pub mod edge_curve;
pub mod face_surface;
pub mod healer;
pub mod model;
pub mod pcurve;
pub mod store;
pub mod trim;
pub mod validator;

pub use builder::BrepBuilder;
pub use model::BrepModel;
pub use store::BrepStore;

