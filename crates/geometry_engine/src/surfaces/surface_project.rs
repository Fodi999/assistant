//! Project a 3D point onto a surface (closest uv).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real};
use super::surface::Surface;

pub fn project<S: Surface>(s: &S, p: Point3) -> (Real, Real) { todo!() }

