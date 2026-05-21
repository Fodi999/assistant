//! Curve↔Curve intersection.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Point3;
use super::curve::Curve;

pub struct CurveIntersection { pub point: Point3, pub t_a: f64, pub t_b: f64 }

pub fn intersect<A: Curve, B: Curve>(a: &A, b: &B) -> Vec<CurveIntersection> { todo!() }

