//! Gemini-related infrastructure adapters.
//!
//! The legacy `gemini_service.rs` (one big file) stays where it is for now;
//! new vertical-slice adapters live under this module.

pub mod vision_3d;

pub use vision_3d::GeminiVision3D;
