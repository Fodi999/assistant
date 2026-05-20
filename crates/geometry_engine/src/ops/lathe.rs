//! Lathe / revolve operation — sweep LatheProfile around Y axis.

use std::f32::consts::PI;

use crate::math::Vec3;
use crate::mesh::{GeometryError, MeshPart};
use crate::profile::LatheProfile as Profile;

/// Revolve `profile` around the Y axis with `segments` slices (>= 3).
pub fn lathe_profile(profile: &Profile, segments: usize) -> Result<MeshPart, GeometryError> {
    if segments < 3 {
        return Err(GeometryError::InvalidArgument(format!(
            "lathe segments must be >= 3 (got {segments})"
        )));
    }
    let pts = &profile.points;
    let p_count = pts.len();
    let ring_size = segments + 1;

    let mut arc: Vec<f32> = Vec::with_capacity(p_count);
    arc.push(0.0);
    for i in 1..p_count {
        let dr = pts[i].radius - pts[i-1].radius;
        let dy = pts[i].y     - pts[i-1].y;
        arc.push(arc[i-1] + (dr*dr + dy*dy).sqrt());
    }
    let total_arc = arc.last().copied().unwrap_or(1.0).max(1e-6);

    let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(p_count * ring_size);
    let mut normals:  Vec<[f32; 3]> = Vec::with_capacity(p_count * ring_size);
    let mut uvs:      Vec<[f32; 2]> = Vec::with_capacity(p_count * ring_size);

    for (pi, p) in pts.iter().enumerate() {
        let (dr, dy) = if pi == 0 {
            (pts[1].radius - p.radius, pts[1].y - p.y)
        } else if pi == p_count - 1 {
            (p.radius - pts[pi-1].radius, p.y - pts[pi-1].y)
        } else {
            (pts[pi+1].radius - pts[pi-1].radius, pts[pi+1].y - pts[pi-1].y)
        };
        let n_len = (dy*dy + dr*dr).sqrt().max(1e-8);
        let nr = dy / n_len;
        let ny = -dr / n_len;
        let v = arc[pi] / total_arc;

        for s in 0..=segments {
            let theta = (s as f32 / segments as f32) * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();
            vertices.push([cos_t * p.radius, p.y, sin_t * p.radius]);
            let n3 = Vec3::new(cos_t * nr, ny, sin_t * nr).normalized();
            normals.push(n3.to_array());
            uvs.push([s as f32 / segments as f32, v]);
        }
    }

    let mut faces: Vec<[usize; 3]> = Vec::with_capacity(2 * segments * (p_count-1));
    for i in 0..p_count-1 {
        for s in 0..segments {
            let a = i * ring_size + s;
            let b = (i+1) * ring_size + s;
            let c = (i+1) * ring_size + s + 1;
            let d = i * ring_size + s + 1;
            faces.push([a, b, c]);
            faces.push([a, c, d]);
        }
    }

    Ok(MeshPart { vertices, normals, uvs, faces })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::LathePoint;

    fn cylinder() -> Profile {
        Profile::new(vec![
            LathePoint::new(0.05, -0.05),
            LathePoint::new(0.05,  0.05),
        ]).unwrap()
    }

    #[test]
    fn rejects_low_segments() {
        assert!(lathe_profile(&cylinder(), 2).is_err());
    }

    #[test]
    fn cylinder_vertex_count() {
        let mp = lathe_profile(&cylinder(), 16).unwrap();
        assert_eq!(mp.vertex_count(), 2 * 17);
        assert_eq!(mp.face_count(), 16 * 2);
    }

    #[test]
    fn cylinder_radius_correct() {
        let mp = lathe_profile(&cylinder(), 16).unwrap();
        for v in &mp.vertices {
            let r = (v[0]*v[0] + v[2]*v[2]).sqrt();
            assert!((r - 0.05).abs() < 1e-5);
        }
    }
}
