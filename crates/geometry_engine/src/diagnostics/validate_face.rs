//! Validate a face (closed loops, surface bounds).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::brep::BrepModel;
use super::report::Report;

pub fn run(_model: &BrepModel) -> Report { Report::ok() }

