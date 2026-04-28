//! Decal / label primitives (PR #15).
//!
//! Two helpers:
//!
//! * [`cylindrical_band`] — a thin cylindrical strip wrapped around the
//!   rotation axis. Used for the front label of a sauce bottle. UVs span
//!   `u ∈ [0, 1]` around the circumference and `v ∈ [0, 1]` from `y_min`
//!   to `y_max`. The radius is offset slightly outward from the bottle
//!   wall (configurable) so the decal sits on top without z-fighting.
//!
//! * [`flat_patch`] — a flat rectangular quad sitting in front of a jar
//!   or any object. Centred at `(0, center_y, depth)`, lying in the
//!   XY plane, faces `+Z`. UVs `u ∈ [0, 1]` left→right, `v ∈ [0, 1]`
//!   bottom→top.
//!
//! Both produce a [`MeshPart`] that the caller drops into a label
//! material group via `MeshBuilder::add_part`.

use std::f32::consts::PI;

use super::lathe::MeshPart;
use super::validate::GeometryError;

/// Cylindrical label band: ring at `y_min` and ring at `y_max`, both at
/// `radius`, with `segments` slices around. Faces outward (`+r̂`).
///
/// `segments >= 8` to avoid faceted-looking labels.
pub fn cylindrical_band(
    radius: f32,
    y_min: f32,
    y_max: f32,
    segments: usize,
) -> Result<MeshPart, GeometryError> {
    if segments < 8 {
        return Err(GeometryError::InvalidArgument(format!(
            "label segments must be >= 8 (got {segments})"
        )));
    }
    if !radius.is_finite() || radius <= 0.0 {
        return Err(GeometryError::InvalidArgument(format!(
            "label radius must be > 0 (got {radius})"
        )));
    }
    if !(y_min.is_finite() && y_max.is_finite()) || y_max <= y_min {
        return Err(GeometryError::InvalidArgument(format!(
            "label requires y_max > y_min (got {y_min}..{y_max})"
        )));
    }

    let ring_size = segments + 1;
    let mut vertices = Vec::with_capacity(2 * ring_size);
    let mut normals = Vec::with_capacity(2 * ring_size);
    let mut uvs = Vec::with_capacity(2 * ring_size);

    for s in 0..=segments {
        let t = s as f32 / segments as f32;
        let theta = t * 2.0 * PI;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Ring at y_min (v=0).
        vertices.push([cos_t * radius, y_min, sin_t * radius]);
        normals.push([cos_t, 0.0, sin_t]);
        uvs.push([t, 0.0]);

        // Ring at y_max (v=1).
        vertices.push([cos_t * radius, y_max, sin_t * radius]);
        normals.push([cos_t, 0.0, sin_t]);
        uvs.push([t, 1.0]);
    }

    let mut faces = Vec::with_capacity(2 * segments);
    for s in 0..segments {
        let b0 = s * 2;
        let t0 = s * 2 + 1;
        let b1 = (s + 1) * 2;
        let t1 = (s + 1) * 2 + 1;
        // CCW seen from outside (outward normal +r̂).
        faces.push([b0, b1, t1]);
        faces.push([b0, t1, t0]);
    }

    Ok(MeshPart {
        vertices,
        normals,
        uvs,
        faces,
    })
}

/// Flat rectangular label patch on the `+Z` face of an object, centred at
/// `(0, center_y, depth)`. Width along X, height along Y.
pub fn flat_patch(
    width: f32,
    height: f32,
    center_y: f32,
    depth: f32,
) -> Result<MeshPart, GeometryError> {
    if !(width.is_finite() && height.is_finite() && depth.is_finite() && center_y.is_finite()) {
        return Err(GeometryError::InvalidArgument(
            "flat_patch arguments must be finite".into(),
        ));
    }
    if width <= 0.0 || height <= 0.0 {
        return Err(GeometryError::InvalidArgument(format!(
            "flat_patch width/height must be > 0 (got {width}x{height})"
        )));
    }

    let hw = width * 0.5;
    let hh = height * 0.5;
    let n = [0.0_f32, 0.0, 1.0];

    let vertices = vec![
        [-hw, center_y - hh, depth],
        [hw, center_y - hh, depth],
        [hw, center_y + hh, depth],
        [-hw, center_y + hh, depth],
    ];
    let normals = vec![n; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let faces = vec![[0, 1, 2], [0, 2, 3]];

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
    fn cylindrical_band_has_correct_vertex_count() {
        let p = cylindrical_band(0.030, -0.01, 0.01, 32).unwrap();
        assert_eq!(p.vertices.len(), 2 * 33);
        assert_eq!(p.faces.len(), 32 * 2);
    }

    #[test]
    fn cylindrical_band_normals_point_outward() {
        let p = cylindrical_band(0.030, -0.01, 0.01, 16).unwrap();
        for (v, n) in p.vertices.iter().zip(p.normals.iter()) {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            let outward = [v[0] / r, 0.0, v[2] / r];
            let dot = n[0] * outward[0] + n[1] * outward[1] + n[2] * outward[2];
            assert!(dot > 0.99);
        }
    }

    #[test]
    fn cylindrical_band_uvs_cover_unit_square() {
        let p = cylindrical_band(0.030, -0.01, 0.01, 16).unwrap();
        let (mut min_u, mut max_u, mut min_v, mut max_v) =
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        for [u, v] in &p.uvs {
            min_u = min_u.min(*u);
            max_u = max_u.max(*u);
            min_v = min_v.min(*v);
            max_v = max_v.max(*v);
        }
        assert!((min_u - 0.0).abs() < 1e-6);
        assert!((max_u - 1.0).abs() < 1e-6);
        assert!((min_v - 0.0).abs() < 1e-6);
        assert!((max_v - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cylindrical_band_rejects_bad_args() {
        assert!(cylindrical_band(0.030, 0.0, 0.0, 16).is_err());
        assert!(cylindrical_band(0.030, 0.01, -0.01, 16).is_err());
        assert!(cylindrical_band(-0.01, -0.01, 0.01, 16).is_err());
        assert!(cylindrical_band(0.030, -0.01, 0.01, 4).is_err());
    }

    #[test]
    fn flat_patch_has_4_verts_and_2_faces() {
        let p = flat_patch(0.04, 0.03, 0.0, 0.041).unwrap();
        assert_eq!(p.vertices.len(), 4);
        assert_eq!(p.faces.len(), 2);
        for n in &p.normals {
            assert!((n[2] - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn flat_patch_rejects_bad_args() {
        assert!(flat_patch(0.0, 0.03, 0.0, 0.041).is_err());
        assert!(flat_patch(0.04, -0.01, 0.0, 0.041).is_err());
        assert!(flat_patch(f32::NAN, 0.03, 0.0, 0.041).is_err());
    }
}
