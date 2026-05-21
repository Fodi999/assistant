//! Planar ellipse with two axis radii.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Plane, Real};

#[derive(Debug, Clone)]
pub struct EllipseCurve { pub plane: Plane, pub radius_major: Real, pub radius_minor: Real }

