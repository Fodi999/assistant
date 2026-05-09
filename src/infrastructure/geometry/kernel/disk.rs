//! Capped disk fans (jar bottom, jar lid top, sauce meniscus, plate underside).
//!
//! Builds a [`MeshPart`] consisting of a centre vertex plus a `segments`
//! ring at radius `r` lying flat at height `y`. Two helpers:
//!
//! * [`disk_fan_up`] — outward normal `+Y`, CCW seen from above.
//! * [`disk_fan_down`] — outward normal `-Y`, CCW seen from below.
//!
//! UVs are radial-polar: centre at `(0.5, 0.5)`, rim around the unit circle.

use std::f32::consts::PI;

use super::lathe::MeshPart;
use super::validate::GeometryError;

/// Disk facing `+Y` (top of liquid, top of lid).
pub fn disk_fan_up(radius: f32, y: f32, segments: usize) -> Result<MeshPart, GeometryError> {
    disk_fan(radius, y, segments, true)
}

/// Disk facing `-Y` (jar bottom, lid underside).
pub fn disk_fan_down(radius: f32, y: f32, segments: usize) -> Result<MeshPart, GeometryError> {
    disk_fan(radius, y, segments, false)
}

fn disk_fan(
    radius: f32,
    y: f32,
    segments: usize,
    face_up: bool,
) -> Result<MeshPart, GeometryError> {
    if segments < 3 {
        return Err(GeometryError::InvalidArgument(format!(
            "disk segments must be >= 3 (got {segments})"
        )));
    }
    if !radius.is_finite() || radius <= 0.0 {
        return Err(GeometryError::InvalidArgument(format!(
            "disk radius must be > 0 (got {radius})"
        )));
    }
    if !y.is_finite() {
        return Err(GeometryError::InvalidArgument(
            "disk y is not finite".into(),
        ));
    }

    let n = if face_up {
        [0.0_f32, 1.0, 0.0]
    } else {
        [0.0_f32, -1.0, 0.0]
    };

    let mut vertices = Vec::with_capacity(segments + 2);
    let mut normals = Vec::with_capacity(segments + 2);
    let mut uvs = Vec::with_capacity(segments + 2);

    // Centre vertex.
    vertices.push([0.0, y, 0.0]);
    normals.push(n);
    uvs.push([0.5, 0.5]);

    for s in 0..=segments {
        let t = s as f32 / segments as f32;
        let theta = t * 2.0 * PI;
        let cx = theta.cos();
        let sz = theta.sin();
        vertices.push([cx * radius, y, sz * radius]);
        normals.push(n);
        uvs.push([0.5 + cx * 0.5, 0.5 + sz * 0.5]);
    }

    let mut faces = Vec::with_capacity(segments);
    for s in 0..segments {
        let a = 0;
        let b = 1 + s;
        let c = 1 + s + 1;
        if face_up {
            faces.push([a, b, c]);
        } else {
            faces.push([a, c, b]);
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

    #[test]
    fn disk_fan_up_normals_point_up() {
        let p = disk_fan_up(0.05, 0.10, 24).unwrap();
        for n in &p.normals {
            assert!((n[1] - 1.0).abs() < 1e-5);
        }
        assert_eq!(p.faces.len(), 24);
    }

    #[test]
    fn disk_fan_down_normals_point_down() {
        let p = disk_fan_down(0.05, -0.10, 16).unwrap();
        for n in &p.normals {
            assert!((n[1] + 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn disk_rejects_bad_args() {
        assert!(disk_fan_up(0.05, 0.0, 2).is_err());
        assert!(disk_fan_up(0.0, 0.0, 16).is_err());
        assert!(disk_fan_up(f32::NAN, 0.0, 16).is_err());
    }

    #[test]
    fn disk_face_indices_in_range() {
        let p = disk_fan_up(0.04, 0.0, 32).unwrap();
        let n = p.vertices.len();
        for [a, b, c] in &p.faces {
            assert!(*a < n && *b < n && *c < n);
        }
    }
}
