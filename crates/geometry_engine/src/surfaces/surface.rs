//! Trait for all parametric 3D surfaces.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real, Vec3};

pub trait Surface {
    fn domain_u(&self) -> (Real, Real);
    fn domain_v(&self) -> (Real, Real);
    fn evaluate(&self, u: Real, v: Real) -> Point3;
    fn normal(&self, u: Real, v: Real) -> Vec3;
}

