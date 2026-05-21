//! Polygon splitting / clipping for boolean face preparation.
//!
//! All operations work on **ordered planar polygons** represented as
//! `Vec<Point3>`.  The main entry point is [`fragment_polygon_by_planes`]
//! which splits a face polygon by every plane in a list and returns all
//! resulting non-degenerate fragments.  Each fragment is then independently
//! classified as Inside/Outside the other solid.
#![allow(dead_code, unused_variables, unused_imports)]

use crate::math::{Point3, Real};
use super::face_face_intersection::{dot, sub3, add3, scale3, pt_arr, arr_pt};

const EPS: Real = 1e-9;

// ── Sutherland–Hodgman polygon clip ─────────────────────────────────────────

/// Clip a polygon against the half-space `n·(x − p) ≥ 0` (keep positive side).
///
/// Uses the Sutherland–Hodgman algorithm.  Degenerate output polygons
/// (< 3 vertices) are returned as-is; callers should filter them out.
pub fn clip_polygon_by_halfspace(
    polygon: &[Point3],
    plane_p: [Real; 3],
    plane_n: [Real; 3],
) -> Vec<Point3> {
    if polygon.is_empty() { return Vec::new(); }

    let sdist = |p: [Real; 3]| -> Real { dot(sub3(p, plane_p), plane_n) };

    let n   = polygon.len();
    let mut out: Vec<Point3> = Vec::with_capacity(n + 1);

    for i in 0..n {
        let cur = pt_arr(polygon[i]);
        let nxt = pt_arr(polygon[(i + 1) % n]);
        let dc  = sdist(cur);
        let dn  = sdist(nxt);

        if dc >= -EPS {
            out.push(arr_pt(cur));
        }
        // Edge straddles the plane → add intersection point.
        if (dc > EPS && dn < -EPS) || (dc < -EPS && dn > EPS) {
            let t     = dc / (dc - dn);
            let inter = add3(cur, scale3(sub3(nxt, cur), t));
            out.push(arr_pt(inter));
        }
    }
    out
}

/// Split a polygon into two pieces: the part on the *inside* (positive) side
/// and the part on the *outside* (negative) side of the given plane.
///
/// Returns `(inside_fragment, outside_fragment)`.  Either may have < 3 vertices
/// if the polygon is entirely on one side.
pub fn split_polygon(
    polygon: &[Point3],
    plane_p: [Real; 3],
    plane_n: [Real; 3],
) -> (Vec<Point3>, Vec<Point3>) {
    let inside  = clip_polygon_by_halfspace(polygon, plane_p, plane_n);
    let neg_n   = [-plane_n[0], -plane_n[1], -plane_n[2]];
    let outside = clip_polygon_by_halfspace(polygon, plane_p, neg_n);
    (inside, outside)
}

// ── Fragment a polygon by multiple planes ────────────────────────────────────

/// Fragment a polygon by successively splitting it against each plane in `planes`.
///
/// `planes` is a list of `(plane_point, outward_unit_normal)` pairs.  Each
/// plane splits every current fragment into two pieces.  Non-degenerate
/// fragments (≥ 3 vertices) are retained.
///
/// After calling this function, each returned fragment is entirely on one
/// side of every given plane.  The centroid of each fragment can then be
/// classified with [`super::classify::classify_point`].
pub fn fragment_polygon_by_planes(
    polygon: &[Point3],
    planes:  &[([Real; 3], [Real; 3])],
) -> Vec<Vec<Point3>> {
    let mut frags: Vec<Vec<Point3>> = vec![polygon.to_vec()];

    for &(plane_p, plane_n) in planes {
        let new_frags: Vec<Vec<Point3>> = frags.iter().flat_map(|frag| {
            let sdists: Vec<Real> = frag.iter()
                .map(|p| dot(sub3(pt_arr(*p), plane_p), plane_n))
                .collect();
            let has_pos = sdists.iter().any(|&d| d >  EPS);
            let has_neg = sdists.iter().any(|&d| d < -EPS);

            if has_pos && has_neg {
                // Fragment straddles the plane → split.
                let pos = clip_polygon_by_halfspace(frag, plane_p, plane_n);
                let neg = clip_polygon_by_halfspace(frag, plane_p,
                    [-plane_n[0], -plane_n[1], -plane_n[2]]);
                let mut result = Vec::new();
                if pos.len() >= 3 { result.push(pos); }
                if neg.len() >= 3 { result.push(neg); }
                result
            } else {
                vec![frag.clone()]
            }
        }).collect();
        frags = new_frags;
    }

    frags
}

// ── Tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Point3;

    fn unit_square() -> Vec<Point3> {
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ]
    }

    #[test]
    fn clip_unit_square_by_midplane_gives_half_square() {
        let sq = unit_square();
        // Clip by x = 0.5, keep x >= 0.5
        let clipped = clip_polygon_by_halfspace(&sq, [0.5, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(clipped.len(), 4);
        for p in &clipped {
            assert!(p.x >= 0.5 - 1e-9, "x={} should be >= 0.5", p.x);
        }
    }

    #[test]
    fn clip_outside_plane_returns_empty() {
        let sq = unit_square();
        let clipped = clip_polygon_by_halfspace(&sq, [2.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(clipped.len() < 3);
    }

    #[test]
    fn clip_inside_plane_keeps_whole_polygon() {
        let sq = unit_square();
        let clipped = clip_polygon_by_halfspace(&sq, [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(clipped.len(), 4);
    }

    #[test]
    fn fragment_square_by_two_planes_gives_four_pieces() {
        let sq = unit_square();
        let planes = [
            ([0.5, 0.0, 0.0], [1.0, 0.0, 0.0]),
            ([0.0, 0.5, 0.0], [0.0, 1.0, 0.0]),
        ];
        let frags = fragment_polygon_by_planes(&sq, &planes);
        assert_eq!(frags.len(), 4, "expected 4 fragments from 2 cuts");
        for f in &frags { assert!(f.len() >= 3); }
    }

    #[test]
    fn fragment_with_non_intersecting_plane_gives_one_piece() {
        let sq = unit_square();
        let planes = [([5.0, 0.0, 0.0], [1.0, 0.0, 0.0])];
        let frags = fragment_polygon_by_planes(&sq, &planes);
        assert_eq!(frags.len(), 1);
    }
}
