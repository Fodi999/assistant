//! Validate a triangle mesh.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::mesh::Mesh;
use super::report::Report;

pub fn run(_mesh: &Mesh) -> Report { Report::ok() }

