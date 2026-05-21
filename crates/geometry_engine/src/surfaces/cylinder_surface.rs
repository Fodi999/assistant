//! Cylinder surface (axis + radius + height).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real, Vec3};
use super::surface::Surface;

#[derive(Debug, Clone)]
pub struct CylinderSurface { pub origin: Point3, pub axis: Vec3, pub radius: Real, pub height: Real }

impl Surface for CylinderSurface {
    fn domain_u(&self) -> (Real, Real) { (0.0, std::f64::consts::TAU) }
    fn domain_v(&self) -> (Real, Real) { (0.0, self.height) }
    fn evaluate(&self, u: Real, v: Real) -> Point3 { todo!() }
    fn normal(&self, u: Real, v: Real) -> Vec3 { todo!() }
}

