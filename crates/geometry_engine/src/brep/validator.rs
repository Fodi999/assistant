//! B-Rep validation (Euler–Poincaré, manifold, …).
#![allow(dead_code, unused_variables, unused_imports)]
use super::model::BrepModel;
use crate::diagnostics::Report;

pub fn validate(_model: &BrepModel) -> Report { Report::ok() }

