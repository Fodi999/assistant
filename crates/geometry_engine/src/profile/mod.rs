//! 2D профили для операций вращения (lathe) и валидации контуров.

pub mod lathe_profile;
pub mod orientation;
pub mod self_intersection;
pub mod validate;

pub use lathe_profile::{LathePoint, LatheProfile};
pub use orientation::{ensure_ccw, is_ccw, signed_area_2d};
pub use validate::validate_profile_3d;
