//! glTF 2.0 export (binary GLB or JSON+bin).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::mesh::{Mesh, GeometryError};

pub fn write_glb(_mesh: &Mesh) -> Result<Vec<u8>, GeometryError> { todo!() }

