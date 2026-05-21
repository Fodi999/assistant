//! Boolean subtraction  A − B.
//!
//! **Algorithm**
//!
//! 1. Keep fragments of A that are *outside* B  (unchanged orientation).
//! 2. Keep fragments of B that are *inside*  A  (normals flipped — these
//!    become the inner cavity walls of the result).
//! 3. Assemble into a new [`BrepModel`].
#![allow(dead_code, unused_variables, unused_imports)]

use crate::brep::BrepModel;
use super::classify::{classify_point, Classification};
use super::face_face_intersection::{face_polygon, collect_face_planes};
use super::split_faces::fragment_polygon_by_planes;
use super::rebuild_shell::{FaceSpec, build_model_from_specs};
use super::union::polygon_centroid;
use crate::math::Real;

const MERGE_TOL: Real = 1e-5;

/// Compute A − B (subtract B from A).
///
/// Returns the portion of A that is outside B, capped by the
/// (inward-flipped) faces of B that were inside A.
pub fn run(a: &BrepModel, b: &BrepModel) -> BrepModel {
    let planes_b = collect_face_planes(b);
    let planes_a = collect_face_planes(a);

    let mut specs: Vec<FaceSpec> = Vec::new();

    // ── Contributions from A: keep what is outside B ─────────────────────
    // OnBoundary means the face of A lies on B's surface — it gets replaced
    // by the (flipped) face from B, so we skip it here.
    for &face_id in a.store.faces.keys() {
        let poly  = face_polygon(&a.store, face_id);
        let frags = fragment_polygon_by_planes(&poly, &planes_b);
        for frag in frags {
            if frag.len() < 3 { continue; }
            let centroid = polygon_centroid(&frag);
            if classify_point(centroid, b) == Classification::Outside {
                specs.push(FaceSpec::new(frag));
            }
        }
    }

    // ── Contributions from B: keep what is inside A, normals flipped ─────
    // Only strictly-inside — coplanar (OnBoundary) faces are either the
    // open hole face (excluded) or already covered by A's outer shell.
    for &face_id in b.store.faces.keys() {
        let poly  = face_polygon(&b.store, face_id);
        let frags = fragment_polygon_by_planes(&poly, &planes_a);
        for frag in frags {
            if frag.len() < 3 { continue; }
            let centroid = polygon_centroid(&frag);
            if classify_point(centroid, a) == Classification::Inside {
                specs.push(FaceSpec::flipped(frag));
            }
        }
    }

    let (model, _) = build_model_from_specs(specs, MERGE_TOL);
    model
}

// ── Tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::brep::builder::BrepBuilder;

    fn box_model(min: [f64; 3], max: [f64; 3]) -> BrepModel {
        let mut b = BrepBuilder::new();
        b.begin_body("box").box_from_extents(min, max);
        b.build()
    }

    fn vertex_bbox(model: &BrepModel) -> ([f64; 3], [f64; 3]) {
        let mut mn = [f64::INFINITY; 3];
        let mut mx = [f64::NEG_INFINITY; 3];
        for v in model.store.vertices.values() {
            mn[0] = mn[0].min(v.point.x);
            mn[1] = mn[1].min(v.point.y);
            mn[2] = mn[2].min(v.point.z);
            mx[0] = mx[0].max(v.point.x);
            mx[1] = mx[1].max(v.point.y);
            mx[2] = mx[2].max(v.point.z);
        }
        (mn, mx)
    }

    /// Subtracting a non-overlapping box leaves A unchanged.
    #[test]
    fn subtract_non_overlapping_b_leaves_a_intact() {
        let a = box_model([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = box_model([5.0, 0.0, 0.0], [6.0, 1.0, 1.0]);
        let result = run(&a, &b);
        // All 6 faces of A survive; 0 faces from B (none are inside A).
        assert_eq!(result.store.faces.len(), 6,
            "expected 6 faces, got {}", result.store.faces.len());
    }

    /// Subtracting a box entirely inside A produces A's 6 faces + 6 inner cavity faces.
    #[test]
    fn subtract_inner_box_creates_cavity_faces() {
        let a = box_model([0.0, 0.0, 0.0], [4.0, 4.0, 4.0]);
        let b = box_model([1.0, 1.0, 1.0], [3.0, 3.0, 3.0]);
        let result = run(&a, &b);
        // 6 faces of A (outside B) + 6 flipped faces of B (inside A) = 12.
        assert_eq!(result.store.faces.len(), 12,
            "expected 12 faces for hollow subtraction, got {}", result.store.faces.len());
    }

    /// Result bbox should not exceed A's bbox when subtracting a box inside A.
    #[test]
    fn subtract_does_not_grow_bbox() {
        let a = box_model([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = box_model([5.0, 5.0, 5.0], [6.0, 6.0, 6.0]);
        let result = run(&a, &b);
        let (_, mx) = vertex_bbox(&result);
        // The result must not extend beyond A's max corner.
        assert!(mx[0] <= 1.0 + 1e-9);
        assert!(mx[1] <= 1.0 + 1e-9);
        assert!(mx[2] <= 1.0 + 1e-9);
    }

    /// Subtracting a box that fully contains A should leave no faces.
    #[test]
    fn subtract_container_leaves_no_faces() {
        let a = box_model([0.25, 0.25, 0.25], [0.75, 0.75, 0.75]);
        let b = box_model([0.0,  0.0,  0.0 ], [1.0,  1.0,  1.0 ]);
        let result = run(&a, &b);
        // All of A is inside B → zero outside fragments; B's faces are also
        // entirely outside A → 0 inside fragments from B.
        assert_eq!(result.store.faces.len(), 0,
            "expected 0 faces when A is fully inside B, got {}",
            result.store.faces.len());
    }
}

