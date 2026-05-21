//! Classify a point or face centroid relative to a closed B-Rep solid.
//!
//! Uses a ray-casting (Jordan curve) test: shoot a ray from the query point
//! and count the number of face triangles it pierces.  Odd → Inside,
//! Even → Outside.  The ray direction is chosen to be irrational w.r.t. the
//! axis directions to reduce the probability of hitting an edge or vertex.
#![allow(dead_code, unused_variables, unused_imports)]

use crate::brep::BrepModel;
use crate::brep::store::BrepStore;
use crate::math::{Point3, Real};
use crate::topology::FaceId;
use super::face_face_intersection::{
    face_polygon, face_plane_with_normal, signed_dist, dot, sub3, add3, scale3, cross,
    pt_arr, arr_pt,
};

// ── Public enum ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// Point is strictly inside the closed solid.
    Inside,
    /// Point is strictly outside the closed solid.
    Outside,
    /// Point lies on (or very close to) a face boundary.
    OnBoundary,
    /// Two faces are coplanar (used in face-face comparisons).
    Coplanar,
}

// ── Constants ────────────────────────────────────────────────────────────────

/// Points within this distance from a face plane are considered OnBoundary.
const ON_BOUNDARY_EPS: Real = 1e-7;

/// Fixed irrational ray direction (normalised).  Chosen so it's not
/// axis-aligned and avoids common coplanarity issues.
///
/// direction = (1, √2, √3) normalised  ≈ (0.4264, 0.6030, 0.6738)
const RAY_DIR: [Real; 3] = [
    0.42640143271122083,
    0.60302268593744253,
    0.67380695989935566,
];

// ── Möller–Trumbore helper ───────────────────────────────────────────────────

/// Ray–triangle intersection (double-sided).
/// Returns `Some(t)` if the ray hits the triangle at `t > EPS`.
fn ray_triangle_t(
    orig: [Real; 3],
    dir:  [Real; 3],
    v0:   [Real; 3],
    v1:   [Real; 3],
    v2:   [Real; 3],
) -> Option<Real> {
    const EPS: Real = 1e-10;
    let e1 = sub3(v1, v0);
    let e2 = sub3(v2, v0);
    let h  = cross(dir, e2);
    let a  = dot(e1, h);
    if a.abs() < EPS { return None; }
    let f = 1.0 / a;
    let s = sub3(orig, v0);
    let u = f * dot(s, h);
    if !(0.0..=1.0).contains(&u) { return None; }
    let q = cross(s, e1);
    let v = f * dot(dir, q);
    if v < 0.0 || u + v > 1.0 { return None; }
    let t = f * dot(e2, q);
    if t < EPS { None } else { Some(t) }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Classify `query` against the solid described by `model`.
///
/// The model must be a closed (watertight) manifold for a correct result.
/// For open shells the result is heuristic.
pub fn classify_point(query: Point3, model: &BrepModel) -> Classification {
    let store = &model.store;
    let orig  = pt_arr(query);

    // First check: is the point sitting on any face plane?
    for (&face_id, _) in &store.faces {
        let poly = face_polygon(store, face_id);
        if poly.len() < 3 { continue; }
        if let Some((plane_pt, plane_n)) = face_plane_with_normal(store, face_id) {
            if signed_dist(orig, plane_pt, plane_n).abs() < ON_BOUNDARY_EPS {
                return Classification::OnBoundary;
            }
        }
    }

    // Ray-cast parity test.
    let mut hit_count: usize = 0;
    for (&face_id, _) in &store.faces {
        let poly = face_polygon(store, face_id);
        if poly.len() < 3 { continue; }
        let v0 = pt_arr(poly[0]);
        for i in 1..poly.len().saturating_sub(1) {
            let v1 = pt_arr(poly[i]);
            let v2 = pt_arr(poly[i + 1]);
            if ray_triangle_t(orig, RAY_DIR, v0, v1, v2).is_some() {
                hit_count += 1;
            }
        }
    }

    if hit_count % 2 == 1 { Classification::Inside } else { Classification::Outside }
}

/// Classify a face (by its polygon centroid) against another B-Rep model.
///
/// Returns `Inside` if the face centroid is inside `other`, `Outside`
/// if outside, or `OnBoundary` / `Coplanar` for degenerate cases.
pub fn classify_face(
    store:   &BrepStore,
    face_id: FaceId,
    other:   &BrepModel,
) -> Classification {
    let poly = face_polygon(store, face_id);
    if poly.len() < 3 { return Classification::Outside; }
    let sum     = poly.iter().fold([0.0; 3], |acc, p| add3(acc, pt_arr(*p)));
    let centroid = arr_pt(scale3(sum, 1.0 / poly.len() as Real));
    classify_point(centroid, other)
}

// ── Tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::brep::builder::BrepBuilder;

    fn unit_box() -> BrepModel {
        let mut b = BrepBuilder::new();
        b.begin_body("box").box_from_extents([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        b.build()
    }

    #[test]
    fn centre_of_unit_box_is_inside() {
        let model = unit_box();
        let c = classify_point(Point3::new(0.5, 0.5, 0.5), &model);
        assert_eq!(c, Classification::Inside);
    }

    #[test]
    fn distant_point_is_outside_unit_box() {
        let model = unit_box();
        let c = classify_point(Point3::new(10.0, 10.0, 10.0), &model);
        assert_eq!(c, Classification::Outside);
    }

    #[test]
    fn negative_point_is_outside_unit_box() {
        let model = unit_box();
        let c = classify_point(Point3::new(-1.0, 0.5, 0.5), &model);
        assert_eq!(c, Classification::Outside);
    }

    #[test]
    fn face_centroids_are_on_boundary_of_own_model() {
        let model = unit_box();
        for &fid in model.store.faces.keys() {
            let cls = classify_face(&model.store, fid, &model);
            // Face centroids lie on the surface → OnBoundary
            assert_eq!(cls, Classification::OnBoundary,
                "face centroid should be OnBoundary for its own model");
        }
    }
}

