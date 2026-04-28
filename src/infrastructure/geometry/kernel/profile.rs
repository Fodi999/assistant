//! 2D revolve profile.
//!
//! A `Profile` is an ordered list of `(radius, y)` points. When fed to
//! [`super::lathe::lathe_profile`] it is revolved around the Y axis to
//! produce a watertight side wall (cylinder, frustum, vase, bottle, …).
//!
//! Conventions:
//!   * `radius >= 0`. A profile may **start or end** at radius 0 (apex) but
//!     two consecutive zero-radius points are rejected as degenerate.
//!   * `y` is monotonically non-decreasing. Profiles that go up then down
//!     would need a different topology (lid + skirt) and are rejected here
//!     to keep the kernel small. Generators that need that just stack two
//!     profiles back-to-back via the `MeshBuilder`.
//!   * Profiles must contain at least 2 points (one segment).

use super::validate::GeometryError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfilePoint {
    pub radius: f32,
    pub y: f32,
}

impl ProfilePoint {
    pub const fn new(radius: f32, y: f32) -> Self {
        Self { radius, y }
    }
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub points: Vec<ProfilePoint>,
}

impl Profile {
    /// Build a `Profile`, returning [`GeometryError::InvalidProfile`] if any
    /// of the contract above is violated.
    pub fn new(points: Vec<ProfilePoint>) -> Result<Self, GeometryError> {
        if points.len() < 2 {
            return Err(GeometryError::InvalidProfile(
                "profile must have at least 2 points".into(),
            ));
        }
        for (i, p) in points.iter().enumerate() {
            if !p.radius.is_finite() || !p.y.is_finite() {
                return Err(GeometryError::InvalidProfile(format!(
                    "profile point {i} has non-finite values"
                )));
            }
            if p.radius < 0.0 {
                return Err(GeometryError::InvalidProfile(format!(
                    "profile point {i} has negative radius {}",
                    p.radius
                )));
            }
        }
        // Y must be non-decreasing.
        for w in points.windows(2) {
            if w[1].y < w[0].y - 1e-6 {
                return Err(GeometryError::InvalidProfile(format!(
                    "profile y is not monotonic: {} → {}",
                    w[0].y, w[1].y
                )));
            }
            if w[0].radius == 0.0 && w[1].radius == 0.0 {
                return Err(GeometryError::InvalidProfile(
                    "two consecutive zero-radius points (degenerate slice)".into(),
                ));
            }
        }
        Ok(Self { points })
    }

    /// Uniformly scale every point in the profile by `factor`.
    pub fn scaled(mut self, factor: f32) -> Self {
        for p in &mut self.points {
            p.radius *= factor;
            p.y *= factor;
        }
        self
    }

    /// Translate every point in the profile along Y by `dy`.
    pub fn translated_y(mut self, dy: f32) -> Self {
        for p in &mut self.points {
            p.y += dy;
        }
        self
    }

    pub fn max_radius(&self) -> f32 {
        self.points.iter().map(|p| p.radius).fold(0.0_f32, f32::max)
    }

    pub fn min_y(&self) -> f32 {
        self.points
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min)
    }

    pub fn max_y(&self) -> f32 {
        self.points
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cylinder_profile_is_valid() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.03, -0.05),
            ProfilePoint::new(0.03, 0.05),
        ])
        .unwrap();
        assert_eq!(p.len(), 2);
        assert!((p.max_radius() - 0.03).abs() < 1e-6);
        assert!((p.min_y() + 0.05).abs() < 1e-6);
        assert!((p.max_y() - 0.05).abs() < 1e-6);
    }

    #[test]
    fn rejects_single_point() {
        let err = Profile::new(vec![ProfilePoint::new(0.03, 0.0)]).unwrap_err();
        assert!(matches!(err, GeometryError::InvalidProfile(_)));
    }

    #[test]
    fn rejects_negative_radius() {
        let err = Profile::new(vec![
            ProfilePoint::new(-0.01, 0.0),
            ProfilePoint::new(0.03, 0.05),
        ])
        .unwrap_err();
        assert!(matches!(err, GeometryError::InvalidProfile(_)));
    }

    #[test]
    fn rejects_non_monotonic_y() {
        let err = Profile::new(vec![
            ProfilePoint::new(0.03, 0.05),
            ProfilePoint::new(0.03, 0.0), // goes back down
        ])
        .unwrap_err();
        assert!(matches!(err, GeometryError::InvalidProfile(_)));
    }

    #[test]
    fn rejects_two_consecutive_zero_radius() {
        let err = Profile::new(vec![
            ProfilePoint::new(0.0, 0.0),
            ProfilePoint::new(0.0, 0.05),
        ])
        .unwrap_err();
        assert!(matches!(err, GeometryError::InvalidProfile(_)));
    }

    #[test]
    fn rejects_nan_values() {
        let err = Profile::new(vec![
            ProfilePoint::new(f32::NAN, 0.0),
            ProfilePoint::new(0.03, 0.05),
        ])
        .unwrap_err();
        assert!(matches!(err, GeometryError::InvalidProfile(_)));
    }

    #[test]
    fn scaled_and_translated() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.02, 0.0),
            ProfilePoint::new(0.02, 0.10),
        ])
        .unwrap()
        .scaled(2.0)
        .translated_y(0.05);
        assert!((p.points[0].radius - 0.04).abs() < 1e-6);
        assert!((p.points[0].y - 0.05).abs() < 1e-6);
        assert!((p.points[1].y - 0.25).abs() < 1e-6);
    }
}
