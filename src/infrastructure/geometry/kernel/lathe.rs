//! Lathe / revolve operation.
//!
//! Takes a 2D [`Profile`] and revolves it around the Y axis into a 3D side
//! wall. The result is returned as a [`MeshPart`] — a self-contained vertex
//! / index block that the caller can drop into a [`MeshBuilder`] under any
//! material group.
//!
//! Algorithm:
//!   * For each profile point (`P` of them), emit `segments + 1` vertices
//!     around the Y axis (the extra one closes UVs cleanly without
//!     duplicating geometry).
//!   * For each adjacent pair of rings, emit `segments` quads → 2·segments
//!     triangles.
//!
//! Normals are computed analytically from the **local profile slope**:
//! the outward normal of a point at radius `r` and slope `dr/dy` is
//! `(cos θ, -dr/dy, sin θ)` normalised. This is exact for revolved
//! frustums and good enough for shading on smooth profiles.
//!
//! UV layout:
//!   * `u = segment / segments`  (0..1 around the rotation)
//!   * `v = (cumulative arc length up the profile) / total arc length`
//!     — gives uniformly-stretched textures regardless of where the
//!     profile narrows or widens.

use std::f32::consts::PI;

use super::math::Vec3;
use super::profile::Profile;
use super::validate::GeometryError;

/// One self-contained block of revolved geometry. Vertices/normals/uvs are
/// in **local** space (just like `Mesh`), and `faces` references local
/// indices `0..vertices.len()`.
#[derive(Debug, Clone)]
pub struct MeshPart {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub faces: Vec<[usize; 3]>,
}

impl MeshPart {
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Return a copy with **all face windings reversed** and **all normals
    /// negated**. Used for inner walls of bowls / hollow shells where the
    /// surface should face the rotation axis instead of away from it.
    pub fn flipped(&self) -> Self {
        let normals = self.normals.iter().map(|n| [-n[0], -n[1], -n[2]]).collect();
        let faces = self.faces.iter().map(|f| [f[0], f[2], f[1]]).collect();
        Self {
            vertices: self.vertices.clone(),
            normals,
            uvs: self.uvs.clone(),
            faces,
        }
    }
}

/// Revolve `profile` around the Y axis with `segments` slices.
///
/// `segments` must be `>= 3`.
pub fn lathe_profile(profile: &Profile, segments: usize) -> Result<MeshPart, GeometryError> {
    if segments < 3 {
        return Err(GeometryError::InvalidArgument(format!(
            "lathe segments must be >= 3 (got {segments})"
        )));
    }
    let pts = &profile.points;
    let p_count = pts.len();
    let ring_size = segments + 1;

    // Pre-compute cumulative arc length up the profile for V coordinates.
    let mut arc: Vec<f32> = Vec::with_capacity(p_count);
    arc.push(0.0);
    for i in 1..p_count {
        let dr = pts[i].radius - pts[i - 1].radius;
        let dy = pts[i].y - pts[i - 1].y;
        arc.push(arc[i - 1] + (dr * dr + dy * dy).sqrt());
    }
    let total_arc = arc.last().copied().unwrap_or(1.0).max(1e-6);

    let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(p_count * ring_size);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(p_count * ring_size);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(p_count * ring_size);

    for (pi, p) in pts.iter().enumerate() {
        // Local profile slope (dr/dy). At endpoints, copy the neighbour's slope.
        let (dr, dy) = if pi == 0 {
            (pts[1].radius - p.radius, pts[1].y - p.y)
        } else if pi == p_count - 1 {
            (p.radius - pts[pi - 1].radius, p.y - pts[pi - 1].y)
        } else {
            (
                pts[pi + 1].radius - pts[pi - 1].radius,
                pts[pi + 1].y - pts[pi - 1].y,
            )
        };
        // Outward 2D normal in (r, y) space is (dy, -dr) / |.|
        let n_len = (dy * dy + dr * dr).sqrt().max(1e-8);
        let nr = dy / n_len;
        let ny = -dr / n_len;

        let v = arc[pi] / total_arc;

        for s in 0..=segments {
            let t = s as f32 / segments as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            vertices.push([cos_t * p.radius, p.y, sin_t * p.radius]);

            // Outward 3D normal: rotate (nr, ny) around Y.
            let n3 = Vec3::new(cos_t * nr, ny, sin_t * nr).normalized();
            normals.push(n3.to_array());

            uvs.push([t, v]);
        }
    }

    // Triangulate the side wall. For ring `i` and segment `s`:
    //   a = i*ring_size + s
    //   b = (i+1)*ring_size + s
    //   c = (i+1)*ring_size + s + 1
    //   d = i*ring_size + s + 1
    // Two triangles: (a, b, c) and (a, c, d) — CCW seen from outside.
    let mut faces: Vec<[usize; 3]> = Vec::with_capacity(2 * segments * (p_count - 1));
    for i in 0..p_count - 1 {
        for s in 0..segments {
            let a = i * ring_size + s;
            let b = (i + 1) * ring_size + s;
            let c = (i + 1) * ring_size + s + 1;
            let d = i * ring_size + s + 1;
            faces.push([a, b, c]);
            faces.push([a, c, d]);
        }
    }

    Ok(MeshPart {
        vertices,
        normals,
        uvs,
        faces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::kernel::profile::ProfilePoint;

    #[test]
    fn lathe_rejects_low_segment_count() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.03, 0.0),
            ProfilePoint::new(0.03, 0.10),
        ])
        .unwrap();
        assert!(lathe_profile(&p, 2).is_err());
    }

    #[test]
    fn lathe_2_point_profile_is_a_cylinder() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.05, -0.05),
            ProfilePoint::new(0.05, 0.05),
        ])
        .unwrap();
        let mp = lathe_profile(&p, 16).unwrap();

        // 2 rings × 17 verts each
        assert_eq!(mp.vertex_count(), 2 * 17);
        // 16 segments × 2 triangles
        assert_eq!(mp.face_count(), 16 * 2);

        // Every vertex must be at radius 0.05 from the Y axis.
        for v in &mp.vertices {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            assert!((r - 0.05).abs() < 1e-5, "got r={r}");
        }
    }

    #[test]
    fn lathe_normals_point_outward_for_cylinder() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.05, -0.05),
            ProfilePoint::new(0.05, 0.05),
        ])
        .unwrap();
        let mp = lathe_profile(&p, 8).unwrap();

        for (i, v) in mp.vertices.iter().enumerate() {
            // Outward direction in XZ plane.
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt().max(1e-6);
            let outward = [v[0] / r, 0.0_f32, v[2] / r];
            let n = mp.normals[i];
            let dot = n[0] * outward[0] + n[1] * outward[1] + n[2] * outward[2];
            assert!(dot > 0.99, "normal not outward enough: dot={dot}");
        }
    }

    #[test]
    fn lathe_face_indices_are_in_range() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.02, -0.02),
            ProfilePoint::new(0.04, 0.00),
            ProfilePoint::new(0.04, 0.05),
        ])
        .unwrap();
        let mp = lathe_profile(&p, 12).unwrap();

        for [a, b, c] in &mp.faces {
            assert!(*a < mp.vertices.len());
            assert!(*b < mp.vertices.len());
            assert!(*c < mp.vertices.len());
        }
    }

    #[test]
    fn lathe_normals_are_unit_length() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.02, 0.0),
            ProfilePoint::new(0.05, 0.05),
        ])
        .unwrap();
        let mp = lathe_profile(&p, 16).unwrap();
        for n in &mp.normals {
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((l - 1.0).abs() < 1e-4, "normal not unit: {l}");
        }
    }

    #[test]
    fn lathe_frustum_normals_tilt_with_slope() {
        // 1 cm radius at bottom, 5 cm radius at top, 10 cm tall — clear taper.
        let p = Profile::new(vec![
            ProfilePoint::new(0.01, 0.0),
            ProfilePoint::new(0.05, 0.10),
        ])
        .unwrap();
        let mp = lathe_profile(&p, 16).unwrap();
        // For a widening frustum, normals must have a *negative* Y component.
        for n in &mp.normals {
            assert!(
                n[1] < 0.0,
                "frustum normal Y should be negative, got {}",
                n[1]
            );
        }
    }

    #[test]
    fn flipped_inverts_normals_and_winding() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.05, -0.05),
            ProfilePoint::new(0.05, 0.05),
        ])
        .unwrap();
        let mp = lathe_profile(&p, 8).unwrap();
        let flipped = mp.flipped();

        assert_eq!(flipped.vertices.len(), mp.vertices.len());
        assert_eq!(flipped.faces.len(), mp.faces.len());

        // Normals must point inward now: dot with outward < 0.
        for (i, v) in flipped.vertices.iter().enumerate() {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt().max(1e-6);
            let outward = [v[0] / r, 0.0_f32, v[2] / r];
            let n = flipped.normals[i];
            let dot = n[0] * outward[0] + n[1] * outward[1] + n[2] * outward[2];
            assert!(dot < -0.99, "flipped normal should be inward: dot={dot}");
        }

        // Windings reversed: original (a,b,c) → (a,c,b).
        for (orig, flip) in mp.faces.iter().zip(flipped.faces.iter()) {
            assert_eq!(flip[0], orig[0]);
            assert_eq!(flip[1], orig[2]);
            assert_eq!(flip[2], orig[1]);
        }
    }
}
