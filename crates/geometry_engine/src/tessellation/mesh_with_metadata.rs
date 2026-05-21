//! Tessellated mesh annotated with originating B-Rep IDs.
//!
//! Each triangle in `mesh.faces` has a corresponding `TriangleMeta` at the
//! same index in `triangles`, carrying the `FaceId` of the B-Rep face that
//! produced it. This enables face-level picking, per-face material
//! assignment, and round-tripping back to topology after rendering.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::{mesh::Mesh, topology::FaceId};

/// One entry per triangle in [`MeshWithMetadata::mesh.faces`].
#[derive(Debug, Clone, Copy)]
pub struct TriangleMeta {
    /// The B-Rep face this triangle was tessellated from.
    pub face_id: FaceId,
}

#[derive(Debug, Clone)]
pub struct MeshWithMetadata {
    pub mesh: Mesh,
    /// Length == `mesh.faces.len()`.
    pub triangles: Vec<TriangleMeta>,
}

impl MeshWithMetadata {
    pub fn triangle_count(&self) -> usize { self.mesh.faces.len() }
    pub fn vertex_count(&self) -> usize   { self.mesh.vertices.len() }

    /// Group triangle indices by FaceId — useful for assigning per-face
    /// materials or for picking.
    pub fn triangles_by_face(&self) -> std::collections::HashMap<FaceId, Vec<usize>> {
        let mut out: std::collections::HashMap<FaceId, Vec<usize>> = Default::default();
        for (i, m) in self.triangles.iter().enumerate() {
            out.entry(m.face_id).or_default().push(i);
        }
        out
    }
}


