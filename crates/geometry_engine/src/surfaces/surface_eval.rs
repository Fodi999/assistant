//! Higher-level surface evaluation helpers.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real, Vec3};
use super::surface::Surface;

pub fn point_at<S: Surface>(s: &S, u: Real, v: Real) -> Point3 { s.evaluate(u, v) }
pub fn normal_at<S: Surface>(s: &S, u: Real, v: Real) -> Vec3 { s.normal(u, v) }

