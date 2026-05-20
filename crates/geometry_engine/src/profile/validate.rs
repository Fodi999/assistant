//! Валидация 3D профиля (список точек перед экструзией).

use crate::mesh::GeometryError;

/// Validate a flat list of 3D points (as [f64; 3]) that will be used as a
/// sketch profile. Returns the projected 2D points (f32) ready for extrude.
///
/// Checks:
///  * at least 3 points
///  * all coordinates finite
///  * no two consecutive duplicate points
pub fn validate_profile_3d(
    pts: &[[f64; 3]],
) -> Result<Vec<(f32, f32, f32)>, GeometryError> {
    if pts.len() < 3 {
        return Err(GeometryError::InvalidProfile(format!(
            "need ≥3 points, got {}", pts.len()
        )));
    }
    let mut out = Vec::with_capacity(pts.len());
    for (i, p) in pts.iter().enumerate() {
        if !p[0].is_finite() || !p[1].is_finite() || !p[2].is_finite() {
            return Err(GeometryError::InvalidProfile(format!(
                "point {i} has non-finite coordinate"
            )));
        }
        let fp = (p[0] as f32, p[1] as f32, p[2] as f32);
        if i > 0 {
            let prev: (f32, f32, f32) = out[i - 1];
            let dx = fp.0 - prev.0;
            let dy = fp.1 - prev.1;
            let dz = fp.2 - prev.2;
            if (dx*dx + dy*dy + dz*dz).sqrt() < 1e-9_f32 {
                return Err(GeometryError::InvalidProfile(format!(
                    "points {i} and {} are duplicate", i-1
                )));
            }
        }
        out.push(fp);
    }
    Ok(out)
}
