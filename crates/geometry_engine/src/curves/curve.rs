//! Trait shared by all parametric 3D curves.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real, Vec3};

pub trait Curve {
    fn domain(&self) -> (Real, Real);
    fn evaluate(&self, t: Real) -> Point3;
    fn derivative(&self, t: Real) -> Vec3;
    fn length(&self) -> Real { todo!() }
}

