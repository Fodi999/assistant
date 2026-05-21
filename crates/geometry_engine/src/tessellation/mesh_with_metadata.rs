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

    /// Return all triangle indices that belong to `face_id`.
    ///
    /// This is the hot-path for highlight / selection: given the `FaceId`
    /// returned by [`pick_face`][crate::selection::pick_face], you get the
    /// exact set of triangles to re-upload as a highlight draw-call.
    ///
    /// O(n) scan — suitable for interactive use on typical CAD meshes
    /// (thousands of triangles). For very large meshes consider caching the
    /// result of [`triangles_by_face`][Self::triangles_by_face] once.
    pub fn triangle_indices_for_face(&self, face_id: FaceId) -> Vec<usize> {
        self.triangles
            .iter()
            .enumerate()
            .filter_map(|(i, m)| if m.face_id == face_id { Some(i) } else { None })
            .collect()
    }

    /// Collect the actual `[usize; 3]` index triples (into `mesh.vertices`)
    /// for every triangle that belongs to `face_id`.
    ///
    /// Convenience wrapper over [`triangle_indices_for_face`] for callers
    /// that need to re-submit geometry directly to a GPU index buffer.
    pub fn face_triangles(&self, face_id: FaceId) -> Vec<[usize; 3]> {
        self.triangle_indices_for_face(face_id)
            .into_iter()
            .map(|i| self.mesh.faces[i])
            .collect()
    }

    /// Flatten the vertex positions of every triangle belonging to `face_id`
    /// into a `Vec<[f64; 3]>` — useful for bounding-box or normal computation
    /// of the selected face without access to the full mesh.
    pub fn face_vertex_positions(&self, face_id: FaceId) -> Vec<[f64; 3]> {
        self.face_triangles(face_id)
            .into_iter()
            .flat_map(|tri| tri.map(|vi| self.mesh.vertices[vi]))
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::extrude::{extrude_polygon_brep, ExtrudeOptions, Point2};

    fn box_mwm() -> MeshWithMetadata {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        extrude_polygon_brep(&pts, &ExtrudeOptions { depth: 1.0, bevel: 0.0 })
            .unwrap()
            .mesh
    }

    /// A unit box has 6 faces × 2 triangles = 12 triangles total.
    /// Every face_id must appear at least once in triangles_by_face.
    #[test]
    fn triangles_by_face_covers_all_faces() {
        let mwm = box_mwm();
        let by_face = mwm.triangles_by_face();
        assert_eq!(by_face.len(), 6, "expected 6 B-Rep faces");
        let total: usize = by_face.values().map(|v| v.len()).sum();
        assert_eq!(total, 12, "total triangles across all faces");
    }

    /// triangle_indices_for_face must return the same set as triangles_by_face.
    #[test]
    fn triangle_indices_for_face_agrees_with_by_face() {
        let mwm = box_mwm();
        let by_face = mwm.triangles_by_face();
        for (fid, expected) in &by_face {
            let mut got = mwm.triangle_indices_for_face(*fid);
            let mut exp = expected.clone();
            got.sort_unstable();
            exp.sort_unstable();
            assert_eq!(got, exp, "mismatch for face {:?}", fid);
        }
    }

    /// Each face of the unit box contributes exactly 2 triangles.
    #[test]
    fn each_face_has_two_triangles() {
        let mwm = box_mwm();
        for meta in &mwm.triangles {
            let count = mwm.triangle_indices_for_face(meta.face_id).len();
            assert_eq!(count, 2, "face {:?} has {} triangles (expected 2)", meta.face_id, count);
        }
    }

    /// face_triangles returns valid index triples (all within vertex range).
    #[test]
    fn face_triangles_valid_indices() {
        let mwm = box_mwm();
        let vc = mwm.vertex_count();
        for meta in &mwm.triangles {
            for tri in mwm.face_triangles(meta.face_id) {
                for vi in tri {
                    assert!(vi < vc, "vertex index {} out of range (vc={})", vi, vc);
                }
            }
        }
    }

    /// face_vertex_positions returns 3 positions per triangle × 2 triangles = 6.
    #[test]
    fn face_vertex_positions_count() {
        let mwm = box_mwm();
        let fid = mwm.triangles[0].face_id;
        let pos = mwm.face_vertex_positions(fid);
        assert_eq!(pos.len(), 6, "2 triangles × 3 verts = 6 positions");
    }

    /// Querying a face that doesn't exist returns empty vecs.
    #[test]
    fn unknown_face_returns_empty() {
        use crate::topology::FaceId;
        let mwm = box_mwm();
        let fake = FaceId::fresh();
        assert!(mwm.triangle_indices_for_face(fake).is_empty());
        assert!(mwm.face_triangles(fake).is_empty());
        assert!(mwm.face_vertex_positions(fake).is_empty());
    }
}



