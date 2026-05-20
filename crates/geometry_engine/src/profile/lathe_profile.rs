//! 2D профиль для операции вращения вокруг оси Y (lathe). Uses Real (f64).

use crate::math::Real;
use crate::mesh::GeometryError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LathePoint {
    pub radius: Real,
    pub y: Real,
}

impl LathePoint {
    #[inline]
    pub const fn new(radius: Real, y: Real) -> Self { Self { radius, y } }
}

#[derive(Debug, Clone)]
pub struct LatheProfile {
    pub points: Vec<LathePoint>,
}

impl LatheProfile {
    pub fn new(points: Vec<LathePoint>) -> Result<Self, GeometryError> {
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
                    "profile point {i} has negative radius {}", p.radius
                )));
            }
        }
        for w in points.windows(2) {
            if w[1].y < w[0].y - 1e-6 {
                return Err(GeometryError::InvalidProfile(format!(
                    "profile y is not monotonic: {} → {}", w[0].y, w[1].y
                )));
            }
            if w[0].radius == 0.0 && w[1].radius == 0.0 {
                return Err(GeometryError::InvalidProfile(
                    "two consecutive zero-radius points (degenerate)".into(),
                ));
            }
        }
        Ok(Self { points })
    }

    pub fn scaled(mut self, factor: Real) -> Self {
        for p in &mut self.points { p.radius *= factor; p.y *= factor; }
        self
    }

    pub fn translated_y(mut self, dy: Real) -> Self {
        for p in &mut self.points { p.y += dy; }
        self
    }

    pub fn max_radius(&self) -> Real {
        self.points.iter().map(|p| p.radius).fold(0.0_f64, f64::max)
    }
    pub fn min_y(&self) -> Real {
        self.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min)
    }
    pub fn max_y(&self) -> Real {
        self.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max)
    }
    pub fn len(&self) -> usize { self.points.len() }
    pub fn is_empty(&self) -> bool { self.points.is_empty() }
}
