//! High-level face picking: cast a ray against a tessellated B-Rep and return
//! the closest [`FaceId`] and the full [`Hit`] record.
//!
//! # Usage
//! ```ignore
//! let ray = Ray::from_ndc(nx, ny, eye, target, up, fov, aspect);
//! if let Some(result) = pick_face(&mwm, &ray) {
//!     // result.face_id  → B-Rep topology handle
//!     // result.hit.t    → distance from camera
//!     // result.hit.point → world-space impact point
//! }
//! ```
#![allow(dead_code, unused_imports)]

use crate::math::Ray;
use crate::tessellation::MeshWithMetadata;
use crate::topology::FaceId;
use super::hit_test::{hit_mesh_with_metadata, Hit};

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a successful face-pick operation.
#[derive(Debug, Clone)]
pub struct FacePickResult {
    /// The B-Rep face that was hit.
    pub face_id: FaceId,
    /// Full hit record (world point, distance, triangle index, barycentrics).
    pub hit: Hit,
}

// ─────────────────────────────────────────────────────────────────────────────
// API
// ─────────────────────────────────────────────────────────────────────────────

/// Cast `ray` against the tessellated mesh in `mwm` and return the closest
/// B-Rep face hit, if any.
///
/// The function delegates triangle intersection to
/// [`hit_mesh_with_metadata`] (Möller–Trumbore, double-sided) and then
/// resolves the hit triangle to its originating [`FaceId`] via
/// [`TriangleMeta`][crate::tessellation::TriangleMeta].
pub fn pick_face(mwm: &MeshWithMetadata, ray: &Ray) -> Option<FacePickResult> {
    let hit = hit_mesh_with_metadata(mwm, ray)?;
    let face_id = hit.face_id?;
    Some(FacePickResult { face_id, hit })
}

/// Convenience: pick against multiple bodies / meshes and return the globally
/// closest hit. Useful for scenes with several objects.
pub fn pick_face_from_many<'a>(
    meshes: impl Iterator<Item = &'a MeshWithMetadata>,
    ray: &Ray,
) -> Option<FacePickResult> {
    meshes
        .filter_map(|mwm| pick_face(mwm, ray))
        .min_by(|a, b| a.hit.t.partial_cmp(&b.hit.t).unwrap_or(std::cmp::Ordering::Equal))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Point3, Vec3};
    use crate::ops::extrude::{extrude_polygon_brep, ExtrudeOptions, Point2};

    fn make_box_mwm() -> MeshWithMetadata {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let r = extrude_polygon_brep(&pts, &ExtrudeOptions { depth: 1.0, bevel: 0.0 })
            .expect("extrude ok");
        r.mesh
    }

    /// A ray shot straight down through the centre of the top face (z > 1)
    /// toward -Z should hit the box and return a valid FaceId.
    #[test]
    fn pick_top_face_of_box() {
        let mwm = make_box_mwm();
        // Fire from z = 5 downward.
        let ray = Ray::new(
            Point3::new(0.5, 0.5, 5.0),
            Vec3::new(0.0, 0.0, -1.0),
        );
        let result = pick_face(&mwm, &ray).expect("expected a hit");
        // Make sure the returned face_id actually exists in the metadata.
        let any_meta_with_id = mwm.triangles.iter().any(|m| m.face_id == result.face_id);
        assert!(any_meta_with_id, "face_id not found in metadata");
        // t ≈ 4.0 (z=5 → z=1 top face).
        assert!((result.hit.t - 4.0).abs() < 1e-6, "t={}", result.hit.t);
    }

    /// A ray that completely misses should return None.
    #[test]
    fn miss_returns_none() {
        let mwm = make_box_mwm();
        let ray = Ray::new(
            Point3::new(100.0, 100.0, 5.0),
            Vec3::new(0.0, 0.0, -1.0),
        );
        assert!(pick_face(&mwm, &ray).is_none());
    }

    /// pick_face_from_many returns the closer of two boxes.
    #[test]
    fn pick_from_many_returns_closest() {
        let near = make_box_mwm(); // z ∈ [0, 1]

        // A "far" box: shift all vertices by +10 in Z.
        let mut far = make_box_mwm();
        for v in far.mesh.vertices.iter_mut() { v[2] += 10.0; }

        let ray = Ray::new(
            Point3::new(0.5, 0.5, 20.0),
            Vec3::new(0.0, 0.0, -1.0),
        );
        let result = pick_face_from_many([&near, &far].into_iter(), &ray)
            .expect("expected a hit");
        // Far box top face is at z=11, near box top face at z=1.
        // Ray from z=20: hits far at t≈9, near at t≈19 → closest is far.
        assert!(result.hit.t < 10.0, "expected far box (t≈9), got t={}", result.hit.t);
    }
}


