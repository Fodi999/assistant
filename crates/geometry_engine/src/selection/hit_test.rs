//! Ray–triangle and ray–mesh intersection using Möller–Trumbore.
//!
//! The algorithm is the classic double-sided variant:
//!   1. Compute edge vectors `e1 = v1−v0`, `e2 = v2−v0`.
//!   2. `h = direction × e2`;  `det = e1 · h`.
//!      If |det| < ε the ray is parallel → no hit.
//!   3. Compute barycentric coordinates `(u, v)`.
//!      Both must be in `[0, 1]` with `u + v ≤ 1`.
//!   4. `t = (e2 · q) / det`; must be positive (in front of origin).
//!
//! Returns the *closest* hit (smallest positive `t`).

#![allow(dead_code, unused_imports)]
use crate::math::{Point3, Ray, Real, Vec3};
use crate::mesh::Mesh;
use crate::tessellation::MeshWithMetadata;
use crate::topology::FaceId;

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// A confirmed ray–triangle intersection.
#[derive(Debug, Clone)]
pub struct Hit {
    /// World-space intersection point.
    pub point: Point3,
    /// Ray parameter: `hit.point = ray.origin + t * ray.direction`.
    pub t: Real,
    /// Index into `mesh.faces` of the triangle that was hit.
    pub triangle_index: usize,
    /// Barycentric coordinates `(u, v)` — `w = 1 − u − v`.
    pub uv: (Real, Real),
    /// Optional: B-Rep face this triangle belongs to (only set when the
    /// mesh was created from [`MeshWithMetadata`]).
    pub face_id: Option<FaceId>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Core: Möller–Trumbore
// ─────────────────────────────────────────────────────────────────────────────

const EPSILON: Real = 1e-10;

/// Test a single triangle `(v0, v1, v2)` against `ray`.
/// Returns `Some((t, u, v))` if the ray hits, `None` otherwise.
/// Works for both front- and back-facing triangles.
#[inline]
pub fn ray_triangle(
    ray: &Ray,
    v0: [Real; 3], v1: [Real; 3], v2: [Real; 3],
) -> Option<(Real, Real, Real)> {
    let p = |a: [Real; 3]| Vec3::new(a[0], a[1], a[2]);
    let (v0, v1, v2) = (p(v0), p(v1), p(v2));

    let e1 = v1 - v0;
    let e2 = v2 - v0;

    let h = ray.direction.cross(e2);
    let det = e1.dot(h);

    if det.abs() < EPSILON {
        return None; // parallel
    }

    let inv_det = 1.0 / det;
    let o = Vec3::new(
        ray.origin.x - v0.x,
        ray.origin.y - v0.y,
        ray.origin.z - v0.z,
    );

    let u = o.dot(h) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = o.cross(e1);
    let v = ray.direction.dot(q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = e2.dot(q) * inv_det;
    if t < EPSILON {
        return None; // behind origin
    }

    Some((t, u, v))
}

// ─────────────────────────────────────────────────────────────────────────────
// Mesh-level traversal (brute-force; BVH acceleration comes later)
// ─────────────────────────────────────────────────────────────────────────────

/// Find the closest triangle in `mesh` that `ray` hits.
pub fn hit_mesh(mesh: &Mesh, ray: &Ray) -> Option<Hit> {
    let mut closest: Option<Hit> = None;

    for (tri_idx, tri) in mesh.faces.iter().enumerate() {
        let v0 = mesh.vertices[tri[0]];
        let v1 = mesh.vertices[tri[1]];
        let v2 = mesh.vertices[tri[2]];

        if let Some((t, u, v)) = ray_triangle(ray, v0, v1, v2) {
            let is_closer = closest.as_ref().map_or(true, |c| t < c.t);
            if is_closer {
                closest = Some(Hit {
                    point: ray.at(t),
                    t,
                    triangle_index: tri_idx,
                    uv: (u, v),
                    face_id: None,
                });
            }
        }
    }
    closest
}

/// Find the closest hit in `mesh_with_metadata`, returning `face_id` from
/// the corresponding [`TriangleMeta`].
pub fn hit_mesh_with_metadata(
    mwm: &MeshWithMetadata,
    ray: &Ray,
) -> Option<Hit> {
    let mut hit = hit_mesh(&mwm.mesh, ray)?;
    if let Some(meta) = mwm.triangles.get(hit.triangle_index) {
        hit.face_id = Some(meta.face_id);
    }
    Some(hit)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Point3, Vec3};

    fn ray_z() -> Ray {
        Ray::new(Point3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0))
    }

    /// A triangle in the Z = 0 plane — should be hit by the +Z ray.
    #[test]
    fn hits_front_facing_triangle() {
        let v0 = [-1.0, -1.0, 0.0];
        let v1 = [ 1.0, -1.0, 0.0];
        let v2 = [ 0.0,  1.0, 0.0];
        let r = ray_z();
        let hit = ray_triangle(&r, v0, v1, v2);
        assert!(hit.is_some(), "expected hit");
        let (t, _, _) = hit.unwrap();
        assert!((t - 5.0).abs() < 1e-9, "t={}", t);
    }

    /// Ray shot to the side should miss.
    #[test]
    fn misses_parallel_ray() {
        let v0 = [-1.0, -1.0, 0.0];
        let v1 = [ 1.0, -1.0, 0.0];
        let v2 = [ 0.0,  1.0, 0.0];
        let r = Ray::new(Point3::new(10.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(ray_triangle(&r, v0, v1, v2).is_none());
    }

    /// Ray behind the triangle should not produce a hit (t < 0).
    #[test]
    fn misses_behind_origin() {
        let v0 = [-1.0, -1.0, 0.0];
        let v1 = [ 1.0, -1.0, 0.0];
        let v2 = [ 0.0,  1.0, 0.0];
        // Origin is in front of the triangle, direction points away.
        let r = Ray::new(Point3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(ray_triangle(&r, v0, v1, v2).is_none());
    }

    /// hit_mesh must return the closest of two overlapping triangles.
    #[test]
    fn hit_mesh_returns_closest() {
        use crate::math::Real;
        use crate::mesh::{Material, Mesh};

        // Two triangles at z=0 and z=2 — same XY coverage.
        let verts: Vec<[Real; 3]> = vec![
            [-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0], // tri 0
            [-1.0, -1.0, 2.0], [1.0, -1.0, 2.0], [0.0, 1.0, 2.0], // tri 1
        ];
        let norms = vec![[0.0, 0.0, 1.0_f64]; 6];
        let uvs   = vec![[0.0, 0.0]; 6];
        let faces = vec![[0, 1, 2], [3, 4, 5]];
        let mesh  = Mesh::new(verts, norms, uvs, faces, Material::default());

        let r   = Ray::new(Point3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let hit = hit_mesh(&mesh, &r).expect("expected hit");
        assert_eq!(hit.triangle_index, 0, "expected closer triangle (z=0)");
        assert!((hit.t - 5.0).abs() < 1e-9, "t={}", hit.t);
    }
}


