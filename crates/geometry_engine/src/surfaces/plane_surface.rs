//! Infinite plane as a parametric surface.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Plane, Point3, Real, Vec3};
use super::surface::Surface;

#[derive(Debug, Clone)]
pub struct PlaneSurface { pub plane: Plane }

impl Surface for PlaneSurface {
    fn domain_u(&self) -> (Real, Real) { (Real::NEG_INFINITY, Real::INFINITY) }
    fn domain_v(&self) -> (Real, Real) { (Real::NEG_INFINITY, Real::INFINITY) }
    fn evaluate(&self, u: Real, v: Real) -> Point3 { todo!() }
    fn normal(&self, u: Real, v: Real) -> Vec3 { todo!() }
}

