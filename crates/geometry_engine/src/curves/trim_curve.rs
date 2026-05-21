//! Trimmed sub-range of an underlying curve.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Real;

pub struct TrimCurve<C> { pub inner: C, pub t_min: Real, pub t_max: Real }

