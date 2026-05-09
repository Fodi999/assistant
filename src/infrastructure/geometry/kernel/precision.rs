//! Precision tessellator — converts a domain `GeometricShell` (f64)
//! into an infrastructure `MeshPart` (f32) for GLB export.
//!
//! ## Parasolid correspondence
//! This is the equivalent of Parasolid's **facet module** (`spa_facet`):
//! it takes an exact B-Rep shell and produces a discrete triangle mesh
//! that approximates the smooth geometry within `tol.fitting` chord error.
//!
//! ## DDD layer boundary
//! This module lives in `infrastructure/geometry/kernel/` intentionally:
//!   - It *reads* the domain (`GeometricShell`, `Vertex`, `Tolerance`)
//!   - It *writes* the infrastructure type (`MeshPart`)
//!   - The domain itself never knows about `MeshPart` or f32
//!
//! ```text
//! domain::geometry::GeometricShell   ← precise, f64, B-Rep
//!         │
//!         │  precision::tessellate()
//!         ▼
//! infrastructure::geometry::kernel::MeshPart  ← f32, triangles, GLB-ready
//! ```
//!
//! ## Algorithm
//! 1. Fan-triangulate every face (no new vertices introduced for planar
//!    faces — safe because all our generator faces are already planar).
//! 2. Compute per-vertex normals as the area-weighted average of incident
//!    face normals (same algorithm as `normals::recalculate_smooth_normals`
//!    but running on the f64 domain values before the f32 cast).
//! 3. Assign UV coordinates: for each face, use the face normal to choose
//!    the best projection plane (XY / XZ / YZ) and normalise into [0,1].
//! 4. Cast all f64 values to f32 at the last possible moment.

use std::collections::HashMap;

use crate::domain::geometry::shell::GeometricShell;
use crate::domain::geometry::vertex::Vertex;

use super::lathe::MeshPart;

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Tessellate a `GeometricShell` into a flat triangle `MeshPart`.
///
/// Normals are smooth (area-weighted per-vertex average).
/// UVs use best-fit planar projection per face.
///
/// Returns `None` if the shell has no faces (call `shell.validate()` first).
pub fn tessellate(shell: &GeometricShell) -> Option<MeshPart> {
    if shell.face_count() == 0 {
        return None;
    }

    let domain_verts = shell.vertices();
    let faces = shell.faces();

    // ── 1. Build flat triangle list from face loops ──────────────────────

    struct Triangle {
        indices: [usize; 3], // into domain_verts
        face_idx: usize,     // which face this triangle came from
    }

    let mut triangles: Vec<Triangle> = Vec::new();
    for (fi, face) in faces.iter().enumerate() {
        for tri in face.fan_triangles() {
            triangles.push(Triangle {
                indices: tri,
                face_idx: fi,
            });
        }
    }

    if triangles.is_empty() {
        return None;
    }

    // ── 2. Compute smooth normals (f64, area-weighted per domain vertex) ──

    let nv = domain_verts.len();
    let mut smooth_n = vec![[0.0_f64; 3]; nv];

    for tri in &triangles {
        let [a, b, c] = tri.indices;
        let pa = domain_verts[a];
        let pb = domain_verts[b];
        let pc = domain_verts[c];
        // Unnormalised cross → length = 2 × area (area-weighting built in).
        let fn_ = pa.cross_to(pb, pc);
        for k in 0..3 {
            smooth_n[a][k] += fn_[k];
            smooth_n[b][k] += fn_[k];
            smooth_n[c][k] += fn_[k];
        }
    }

    // Normalise accumulated normals.
    let smooth_n: Vec<[f32; 3]> = smooth_n
        .iter()
        .map(|n| {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 1e-12 {
                [
                    (n[0] / len) as f32,
                    (n[1] / len) as f32,
                    (n[2] / len) as f32,
                ]
            } else {
                [0.0, 1.0, 0.0] // degenerate → UP
            }
        })
        .collect();

    // ── 3. UV: best-fit planar projection per face ────────────────────────

    // For each face choose the projection axis based on the dominant normal
    // component:
    //   |nx| largest → project onto YZ plane (u=y, v=z)
    //   |ny| largest → project onto XZ plane (u=x, v=z)
    //   |nz| largest → project onto XY plane (u=x, v=y)
    //
    // Then normalise the projected coordinates into [0, 1] using the face's
    // own bounding box (Parasolid-style "fitted" UV).

    // domain_vertex_index → UV (f32)
    let mut uv_map: HashMap<usize, [f32; 2]> = HashMap::with_capacity(nv);

    for (fi, face) in faces.iter().enumerate() {
        let n = face.normal.unwrap_or([0.0, 1.0, 0.0]);
        let ax = n[0].abs();
        let ay = n[1].abs();
        let az = n[2].abs();

        let project = |v: Vertex| -> [f64; 2] {
            if ax >= ay && ax >= az {
                [v.y, v.z]
            } else if ay >= ax && ay >= az {
                [v.x, v.z]
            } else {
                [v.x, v.y]
            }
        };

        // Bounding box of projected coords for this face.
        let (mut min_u, mut max_u, mut min_v, mut max_v) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for &idx in &face.loop_ {
            let [pu, pv] = project(domain_verts[idx]);
            min_u = min_u.min(pu);
            max_u = max_u.max(pu);
            min_v = min_v.min(pv);
            max_v = max_v.max(pv);
        }
        let du = (max_u - min_u).max(1e-12);
        let dv = (max_v - min_v).max(1e-12);

        for &idx in &face.loop_ {
            let [pu, pv] = project(domain_verts[idx]);
            uv_map
                .entry(idx)
                .or_insert([((pu - min_u) / du) as f32, ((pv - min_v) / dv) as f32]);
        }
    }

    // ── 4. Assemble output MeshPart (f32) ────────────────────────────────

    // Re-use domain vertex indices directly — one output vertex per domain
    // vertex (no duplication). This is conservative for the UV seam case
    // but acceptable for our convex generators.

    let vertices: Vec<[f32; 3]> = domain_verts.iter().map(|v| v.to_f32()).collect();
    let normals: Vec<[f32; 3]> = smooth_n;
    let uvs: Vec<[f32; 2]> = (0..nv)
        .map(|i| uv_map.get(&i).copied().unwrap_or([0.0, 0.0]))
        .collect();

    let faces_out: Vec<[usize; 3]> = triangles.iter().map(|t| t.indices).collect();

    Some(MeshPart {
        vertices,
        normals,
        uvs,
        faces: faces_out,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::geometry::{shell::GeometricShell, vertex::Vertex};

    fn cube_shell() -> GeometricShell {
        let mut s = GeometricShell::default_precision();
        let v: Vec<usize> = [
            [-0.5_f64, -0.5, -0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
        ]
        .iter()
        .map(|&[x, y, z]| s.add_vertex_raw(Vertex::new(x, y, z)))
        .collect();

        for face in [
            [v[0], v[3], v[2], v[1]],
            [v[4], v[5], v[6], v[7]],
            [v[0], v[1], v[5], v[4]],
            [v[3], v[7], v[6], v[2]],
            [v[0], v[4], v[7], v[3]],
            [v[1], v[2], v[6], v[5]],
        ] {
            s.add_face(face.to_vec()).unwrap();
        }
        s
    }

    #[test]
    fn tessellate_cube_produces_valid_part() {
        let s = cube_shell();
        let part = tessellate(&s).unwrap();

        // 8 vertices, 12 triangles (6 faces × 2 tris each).
        assert_eq!(part.vertices.len(), 8);
        assert_eq!(part.faces.len(), 12);
        assert_eq!(part.normals.len(), 8);
        assert_eq!(part.uvs.len(), 8);
    }

    #[test]
    fn tessellate_normals_are_unit_length() {
        let s = cube_shell();
        let part = tessellate(&s).unwrap();
        for n in &part.normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "non-unit normal: {len}");
        }
    }

    #[test]
    fn tessellate_empty_returns_none() {
        let s = GeometricShell::default_precision();
        assert!(tessellate(&s).is_none());
    }

    #[test]
    fn tessellate_indices_in_bounds() {
        let s = cube_shell();
        let part = tessellate(&s).unwrap();
        let n = part.vertices.len();
        for [a, b, c] in &part.faces {
            assert!(*a < n && *b < n && *c < n);
        }
    }
}
