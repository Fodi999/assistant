//! Knot vector for B-Splines.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Real;

#[derive(Debug, Clone)]
pub struct KnotVector { pub knots: Vec<Real>, pub degree: usize }

impl KnotVector {
    pub fn new(knots: Vec<Real>, degree: usize) -> Self { Self { knots, degree } }
    pub fn span(&self, u: Real) -> usize { todo!() }
}

