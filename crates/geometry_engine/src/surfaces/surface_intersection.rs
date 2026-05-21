//! Surface↔Surface intersection (typically yields curves).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::curves::Curve;
use super::surface::Surface;

pub fn intersect<A: Surface, B: Surface>(a: &A, b: &B) -> Vec<Box<dyn Curve>> { todo!() }

