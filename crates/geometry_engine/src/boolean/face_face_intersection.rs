//! Face↔face intersection producing edge segments.
//!
//! Given two planar faces, computes the line segment where the two faces
//! (treated as infinite planes) intersect, then clips that segment so it
//! lies within both face polygons.
#![allow(dead_code, unused_variables, unused_imports)]

use crate::brep::store::BrepStore;
use crate::math::{Point3, Real};
use crate::topology::FaceId;

// ── Public output type ───────────────────────────────────────────────────────

/// The segment where two planar faces intersect (both endpoints in world space).
#[derive(Debug, Clone)]
pub struct FaceFaceIntersection {
    pub face_a:  FaceId,
    pub face_b:  FaceId,
    /// Both endpoints of the intersection segment.
    pub segment: [Point3; 2],
}

// ── Internal vector helpers (avoid pulling in nalgebra) ──────────────────────

pub(crate) fn sub3(a: [Real; 3], b: [Real; 3]) -> [Real; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn add3(a: [Real; 3], b: [Real; 3]) -> [Real; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(crate) fn scale3(a: [Real; 3], s: Real) -> [Real; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub(crate) fn dot(a: [Real; 3], b: [Real; 3]) -> Real {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn cross(a: [Real; 3], b: [Real; 3]) -> [Real; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn normalize(v: [Real; 3]) -> Option<[Real; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-12 { None } else { Some([v[0] / len, v[1] / len, v[2] / len]) }
}
pub(crate) fn pt_arr(p: Point3) -> [Real; 3] { [p.x, p.y, p.z] }
pub(crate) fn arr_pt(a: [Real; 3]) -> Point3  { Point3::new(a[0], a[1], a[2]) }

// ── Face geometry helpers ────────────────────────────────────────────────────

/// Extract the ordered boundary polygon of `face_id` by walking its outer loop.
pub fn face_polygon(store: &BrepStore, face_id: FaceId) -> Vec<Point3> {
    let Some(face)  = store.get_face(face_id)           else { return Vec::new() };
    let Some(outer) = store.get_loop(face.outer_loop)   else { return Vec::new() };
    let mut pts = Vec::with_capacity(outer.coedges.len());
    for &ce_id in &outer.coedges {
        let Some(ce)   = store.get_coedge(ce_id)       else { continue };
        let Some(edge) = store.get_edge(ce.edge)        else { continue };
        let v_id = if ce.reversed { edge.end } else { edge.start };
        let Some(v)    = store.get_vertex(v_id)         else { continue };
        pts.push(v.point);
    }
    pts
}

/// Newell's method: unnormalised polygon normal (robust for non-convex polygons).
pub(crate) fn newell_normal(poly: &[Point3]) -> [Real; 3] {
    let n = poly.len();
    let (mut nx, mut ny, mut nz) = (0.0_f64, 0.0, 0.0);
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        nx += (a.y - b.y) * (a.z + b.z);
        ny += (a.z - b.z) * (a.x + b.x);
        nz += (a.x - b.x) * (a.y + b.y);
    }
    [nx, ny, nz]
}

/// Compute the plane (centroid, **unit outward normal**) for a face.
/// Incorporates `face.orientation` so the normal points outward.
pub fn face_plane_with_normal(store: &BrepStore, face_id: FaceId) -> Option<([Real; 3], [Real; 3])> {
    let face = store.get_face(face_id)?;
    let poly = face_polygon(store, face_id);
    if poly.len() < 3 { return None; }
    let n_raw = newell_normal(&poly);
    let mut n = normalize(n_raw)?;
    if !face.orientation { n = [-n[0], -n[1], -n[2]]; }
    // Use the polygon centroid as a stable point on the plane.
    let sum = poly.iter().fold([0.0; 3], |acc, p| add3(acc, pt_arr(*p)));
    let c   = scale3(sum, 1.0 / poly.len() as Real);
    Some((c, n))
}

/// Signed distance of `pt` from the plane defined by `(plane_pt, unit_normal)`.
pub(crate) fn signed_dist(pt: [Real; 3], plane_pt: [Real; 3], plane_n: [Real; 3]) -> Real {
    dot(sub3(pt, plane_pt), plane_n)
}

// ── Segment-clipping helpers ─────────────────────────────────────────────────

/// Clip a line segment (a → b) against the half-space `n·(x − p) ≥ 0`.
/// Returns `None` if the entire segment is on the outside (negative) side.
fn clip_segment_by_halfspace(
    a: [Real; 3], b: [Real; 3],
    plane_p: [Real; 3], plane_n: [Real; 3],
) -> Option<([Real; 3], [Real; 3])> {
    const EPS: Real = 1e-9;
    let da = signed_dist(a, plane_p, plane_n);
    let db = signed_dist(b, plane_p, plane_n);
    if da < -EPS && db < -EPS { return None; }          // both outside
    if da >= -EPS && db >= -EPS { return Some((a, b)); } // both inside
    let t   = da / (da - db);
    let mid = add3(a, scale3(sub3(b, a), t));
    if da < -EPS { Some((mid, b)) } else { Some((a, mid)) }
}

/// Clip a segment to the interior of a face polygon.
///
/// The polygon is treated as a (possibly non-convex) planar region.  For each
/// directed edge (eᵢ → eᵢ₊₁) the inward half-space normal is
/// `(eᵢ₊₁ − eᵢ) × poly_n` (pointing inward when the polygon is CCW when
/// viewed from `poly_n`).
pub fn clip_segment_to_polygon(
    mut seg_a: [Real; 3],
    mut seg_b: [Real; 3],
    polygon:   &[Point3],
    poly_n:    [Real; 3],
) -> Option<([Real; 3], [Real; 3])> {
    let n = polygon.len();
    for i in 0..n {
        let ea = pt_arr(polygon[i]);
        let eb = pt_arr(polygon[(i + 1) % n]);
        let edge_dir = sub3(eb, ea);
        let inward   = cross(edge_dir, poly_n);
        match clip_segment_by_halfspace(seg_a, seg_b, ea, inward) {
            None         => return None,
            Some((a, b)) => { seg_a = a; seg_b = b; }
        }
    }
    // Reject degenerate (zero-length) result.
    let d = sub3(seg_b, seg_a);
    if dot(d, d) < 1e-18 { None } else { Some((seg_a, seg_b)) }
}

// ── Plane–plane intersection ─────────────────────────────────────────────────

/// Intersect two planes (each given by a point + unit normal).
/// Returns `(line_point, line_direction)` or `None` if the planes are parallel.
pub fn intersect_two_planes(
    p1: [Real; 3], n1: [Real; 3],
    p2: [Real; 3], n2: [Real; 3],
) -> Option<([Real; 3], [Real; 3])> {
    let dir = cross(n1, n2);
    let dir = normalize(dir)?; // None iff planes are parallel
    let d1 = dot(n1, p1);
    let d2 = dot(n2, p2);
    // Solve the 2×2 system for a point on the intersection line.
    // We try fixing each coordinate to 0 in turn (XY, XZ, YZ).
    for (i, j, k) in [(0usize, 1, 2), (0, 2, 1), (1, 2, 0)] {
        let det = n1[i] * n2[j] - n1[j] * n2[i];
        if det.abs() < 1e-10 { continue; }
        let mut pt = [0.0; 3];
        pt[i] = (d1 * n2[j] - d2 * n1[j]) / det;
        pt[j] = (n1[i] * d2 - n2[i] * d1) / det;
        pt[k] = 0.0;
        return Some((pt, dir));
    }
    None
}

// ── Top-level face×face intersection ────────────────────────────────────────

/// Compute the intersection segment of two planar B-Rep faces.
/// Returns `None` if the faces are parallel, disjoint, or only touch at a point.
pub fn intersect_faces(
    store: &BrepStore,
    fa:    FaceId,
    fb:    FaceId,
) -> Option<FaceFaceIntersection> {
    let poly_a = face_polygon(store, fa);
    let poly_b = face_polygon(store, fb);

    let (ca, na) = face_plane_with_normal(store, fa)?;
    let (cb, nb) = face_plane_with_normal(store, fb)?;

    let (line_pt, line_dir) = intersect_two_planes(ca, na, cb, nb)?;

    // Extend the line far enough to span any reasonable model (~10 km).
    const T_MAX: Real = 1e5;
    let seg_start = add3(line_pt, scale3(line_dir, -T_MAX));
    let seg_end   = add3(line_pt, scale3(line_dir,  T_MAX));

    // Clip first to face A, then to face B.
    let (a1, a2) = clip_segment_to_polygon(seg_start, seg_end, &poly_a, na)?;
    let (b1, b2) = clip_segment_to_polygon(a1, a2,           &poly_b, nb)?;

    Some(FaceFaceIntersection {
        face_a: fa,
        face_b: fb,
        segment: [arr_pt(b1), arr_pt(b2)],
    })
}

// ── Convenience: collect face planes for all faces in a model ────────────────

use crate::brep::BrepModel;

/// Returns (plane_centroid, outward_unit_normal) for every face in the model.
pub fn collect_face_planes(model: &BrepModel) -> Vec<([Real; 3], [Real; 3])> {
    model.store.faces.keys()
        .filter_map(|&fid| face_plane_with_normal(&model.store, fid))
        .collect()
}

// ── Tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::brep::builder::BrepBuilder;
    use crate::math::Point3;

    fn unit_box() -> crate::brep::BrepModel {
        let mut b = BrepBuilder::new();
        b.begin_body("box").box_from_extents([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        b.build()
    }

    #[test]
    fn two_adjacent_faces_produce_intersection_segment() {
        let model = unit_box();
        let face_ids: Vec<FaceId> = model.store.faces.keys().copied().collect();
        // At least one pair of adjacent faces must intersect.
        let mut found = false;
        'outer: for i in 0..face_ids.len() {
            for j in (i + 1)..face_ids.len() {
                if intersect_faces(&model.store, face_ids[i], face_ids[j]).is_some() {
                    found = true;
                    break 'outer;
                }
            }
        }
        assert!(found, "expected at least one face–face intersection in unit box");
    }

    #[test]
    fn parallel_faces_have_no_intersection() {
        // A unit box has 3 pairs of parallel faces (top/bottom, front/back, left/right).
        let model = unit_box();
        let face_ids: Vec<FaceId> = model.store.faces.keys().copied().collect();
        let mut parallel_no_isect = 0usize;
        for i in 0..face_ids.len() {
            for j in (i + 1)..face_ids.len() {
                let (_, na) = face_plane_with_normal(&model.store, face_ids[i]).unwrap();
                let (_, nb) = face_plane_with_normal(&model.store, face_ids[j]).unwrap();
                let cos = dot(na, nb).abs();
                if cos > 0.999 {
                    // Parallel faces
                    let result = intersect_faces(&model.store, face_ids[i], face_ids[j]);
                    if result.is_none() { parallel_no_isect += 1; }
                }
            }
        }
        assert_eq!(parallel_no_isect, 3, "unit box should have exactly 3 parallel face pairs");
    }
}

