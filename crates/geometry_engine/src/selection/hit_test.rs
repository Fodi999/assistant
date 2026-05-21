//! Ray hit testing against meshes / B-Rep.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Ray};

#[derive(Debug, Clone)]
pub struct Hit { pub point: Point3, pub t: f64, pub triangle: usize }

pub fn hit_mesh(_mesh: &crate::mesh::Mesh, _ray: &Ray) -> Option<Hit> { todo!() }

