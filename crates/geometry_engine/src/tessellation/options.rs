//! Tessellation quality options.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Real;

#[derive(Debug, Clone)]
pub struct TessOptions {
    pub max_chord_deviation: Real,
    pub max_angle_deviation: Real,
    pub max_edge_length: Real,
}

impl Default for TessOptions {
    fn default() -> Self {
        Self { max_chord_deviation: 0.001, max_angle_deviation: 0.1, max_edge_length: 0.05 }
    }
}

