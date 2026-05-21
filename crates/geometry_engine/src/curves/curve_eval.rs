//! Evaluation helpers (point + tangent + second derivative).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real, Vec3};
use super::curve::Curve;

pub fn point_at<C: Curve>(c: &C, t: Real) -> Point3 { c.evaluate(t) }
pub fn tangent_at<C: Curve>(c: &C, t: Real) -> Vec3 { c.derivative(t) }

