//! 2D профили для операций вращения (lathe) и валидации контуров.

pub mod lathe_profile;
pub mod orientation;
pub mod profile2;
pub mod profile3;
pub mod self_intersection;
pub mod triangulate;
pub mod validate;
pub mod validated;

pub use lathe_profile::{LathePoint, LatheProfile};
pub use orientation::{ensure_ccw, is_ccw, signed_area_2d};
pub use profile2::Profile2;
pub use profile3::Profile3;
pub use validate::validate_profile_3d;
pub use validated::ValidatedProfile;
