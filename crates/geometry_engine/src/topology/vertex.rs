//! Topological vertex — zero-dimensional entity carrying a 3D point.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Point3;

#[derive(Debug, Clone)]
pub struct Vertex {
    /// 3D position in world-space metres.
    pub point: Point3,
    /// Optional name / label for debugging.
    pub name: Option<String>,
}

impl Vertex {
    pub fn new(point: Point3) -> Self {
        Self { point, name: None }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

impl Default for Vertex {
    fn default() -> Self { Self::new(Point3::ORIGIN) }
}

