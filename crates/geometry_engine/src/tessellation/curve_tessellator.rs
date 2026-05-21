//! Polyline approximation of a parametric curve.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::{curves::Curve, math::Point3};
use super::options::TessOptions;

pub fn tessellate<C: Curve>(_c: &C, _opts: &TessOptions) -> Vec<Point3> { todo!() }

