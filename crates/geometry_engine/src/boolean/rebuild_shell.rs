//! Rebuild a closed shell from a collection of face polygon specifications.
//!
//! [`build_model_from_specs`] takes a list of [`FaceSpec`] (ordered polygon
//! + flip flag) and assembles a fresh [`BrepModel`] using the same
//! `FaceId::fresh()` / `add_face_in_current_shell` pattern as
//! [`crate::brep::builder::BrepBuilder::polyline_face`].
//!
//! Vertices that are within `merge_tol` of each other are merged into a
//! single [`Vertex`].  Edges are similarly de-duplicated so the result is
//! as close to manifold as the input polygons allow.
#![allow(dead_code, unused_variables, unused_imports)]

use crate::brep::{BrepModel, store::BrepStore};
use crate::math::{Point3, Real};
use crate::topology::*;

// ── Public types ─────────────────────────────────────────────────────────────

/// A face described by its ordered polygon vertices.
///
/// * `polygon` — at least 3 points in CCW order when viewed from outside.
/// * `flip`    — if `true`, the polygon winding is reversed so the outward
///               normal points in the opposite direction (used for subtracted
///               faces that need their normal flipped).
#[derive(Debug, Clone)]
pub struct FaceSpec {
    pub polygon: Vec<Point3>,
    pub flip:    bool,
}

impl FaceSpec {
    pub fn new(polygon: Vec<Point3>) -> Self {
        Self { polygon, flip: false }
    }
    pub fn flipped(polygon: Vec<Point3>) -> Self {
        Self { polygon, flip: true }
    }
}

// ── Builder ──────────────────────────────────────────────────────────────────

/// Build a fresh [`BrepModel`] from a list of face polygon specs.
///
/// * Merges vertices within `merge_tol` (use `1e-9` for millimetre-scale
///   geometry, or `1e-6` for metre-scale).
/// * Re-uses edges whose endpoints match already-created vertices.
/// * Returns `(model, body_id)`.
pub fn build_model_from_specs(
    specs:     Vec<FaceSpec>,
    merge_tol: Real,
) -> (BrepModel, BodyId) {
    let mut model = BrepModel::default();
    let store     = &mut model.store;

    // ── Hierarchy skeleton ───────────────────────────────────────────────
    let body_id  = store.add_body(Body::new());
    let shell_id = store.add_shell(Shell::new());
    let solid_id = store.add_solid(Solid::new(body_id, shell_id));
    store.get_body_mut(body_id) .unwrap().add_solid(solid_id);
    store.get_shell_mut(shell_id).unwrap().solid = Some(solid_id);

    // ── Vertex pool (deduplication) ──────────────────────────────────────
    let merge_sq = merge_tol * merge_tol;
    // We store (position, VertexId) and do a linear scan.  For typical
    // boolean results (< 1 000 vertices) this is fast enough.
    let mut vpool: Vec<(Point3, VertexId)> = Vec::new();

    // ── Face construction ────────────────────────────────────────────────
    for spec in specs {
        let poly = if spec.flip {
            // Reverse winding to flip the outward normal.
            let mut p = spec.polygon.clone();
            p.reverse();
            p
        } else {
            spec.polygon
        };

        if poly.len() < 3 { continue; }
        let n = poly.len();

        // 1. Vertices (with merge).
        let vids: Vec<VertexId> = poly.iter().map(|&pt| {
            for (ep, id) in &vpool {
                let dx = pt.x - ep.x;
                let dy = pt.y - ep.y;
                let dz = pt.z - ep.z;
                if dx*dx + dy*dy + dz*dz <= merge_sq {
                    return *id;
                }
            }
            let id = store.add_vertex(Vertex::new(pt));
            vpool.push((pt, id));
            id
        }).collect();

        // 2. Edges (with de-duplication).
        let eids: Vec<(EdgeId, bool)> = (0..n).map(|i| {
            let a = vids[i];
            let b = vids[(i + 1) % n];
            // Look for an existing edge connecting the same pair.
            if let Some((&eid, edge)) = store.edges.iter().find(|(_, e)| {
                (e.start == a && e.end == b) || (e.start == b && e.end == a)
            }) {
                let reversed = edge.start == b;
                (eid, reversed)
            } else {
                // Compute length from vertex positions.
                let len = {
                    let pa = store.get_vertex(a).map(|v| v.point).unwrap_or(Point3::ORIGIN);
                    let pb = store.get_vertex(b).map(|v| v.point).unwrap_or(Point3::ORIGIN);
                    pa.distance(pb)
                };
                let mut e = Edge::new(a, b);
                e.length = len;
                (store.add_edge(e), false)
            }
        }).collect();

        // 3. Loop (sentinel FaceId, patched after the face is created).
        let sentinel = FaceId::fresh();
        let loop_id  = store.add_loop(Loop::outer(sentinel));

        // 4. Co-edges with cyclic prev/next links.
        let coedge_ids: Vec<CoEdgeId> = eids.iter().map(|&(eid, rev)| {
            store.add_coedge(CoEdge::new(eid, loop_id, rev))
        }).collect();
        for i in 0..n {
            let cur = coedge_ids[i];
            let nxt = coedge_ids[(i + 1) % n];
            if let Some(c) = store.get_coedge_mut(cur) { c.next = Some(nxt); }
            if let Some(c) = store.get_coedge_mut(nxt) { c.prev = Some(cur); }
        }
        if let Some(lp) = store.get_loop_mut(loop_id) {
            lp.coedges = coedge_ids;
        }

        // 5. Face (links back into the shell and patches the loop's face pointer).
        let face_id = store.add_face(Face::new(shell_id, loop_id));
        store.get_shell_mut(shell_id).unwrap().add_face(face_id);
        if let Some(lp) = store.get_loop_mut(loop_id) { lp.face = face_id; }
    }

    store.get_shell_mut(shell_id).unwrap().mark_closed();

    (model, body_id)
}

// ── Tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Point3;

    #[test]
    fn single_triangle_produces_one_face() {
        let tri = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let (model, _) = build_model_from_specs(vec![FaceSpec::new(tri)], 1e-9);
        assert_eq!(model.store.faces.len(), 1);
        assert_eq!(model.store.vertices.len(), 3);
        assert_eq!(model.store.edges.len(), 3);
    }

    #[test]
    fn shared_edge_is_reused_by_two_adjacent_triangles() {
        // Two triangles sharing the edge (1,0,0)→(0,1,0).
        let t1 = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let t2 = vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let specs = vec![FaceSpec::new(t1), FaceSpec::new(t2)];
        let (model, _) = build_model_from_specs(specs, 1e-9);
        assert_eq!(model.store.faces.len(), 2);
        // 4 unique vertices, 5 unique edges (3 outer + 1 shared + 1 outer)
        assert_eq!(model.store.vertices.len(), 4);
        assert_eq!(model.store.edges.len(), 5);
    }

    #[test]
    fn flipped_spec_reverses_polygon_winding() {
        let tri = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let (model, _) = build_model_from_specs(
            vec![FaceSpec::flipped(tri)],
            1e-9,
        );
        // Should still produce a valid face.
        assert_eq!(model.store.faces.len(), 1);
    }

    #[test]
    fn degenerate_specs_are_skipped() {
        let line = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)];
        let (model, _) = build_model_from_specs(vec![FaceSpec::new(line)], 1e-9);
        assert_eq!(model.store.faces.len(), 0);
    }
}
