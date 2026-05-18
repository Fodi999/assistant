//! # profiles
//!
//! Profile detection (closed-loop finding) and profile repair / analysis.

pub mod detect;
pub mod repair;

pub use detect::detect_profiles;
pub use repair::{analyze_profile, repair_profile, ProfileAnalyzeRequest, ProfileAnalyzeResponse,
                 ProfileRepairRequest, ProfileRepairResponse};
