//! Integer grid ↔ f64 метры.
//!
//! sketch_engine хранит точки как `gx: i32` + `x: f64 = gx * GRID_SIZE`.
//! Этот модуль содержит ту же константу и хелперы для конвертации.

/// 1 grid unit = 0.00001 метра = 0.01 мм = 10 микрон.
pub const GRID_SIZE: f64 = 0.00001;

/// Конвертировать integer grid coordinate → метры (f64).
#[inline]
pub fn grid_to_f64(g: i32) -> f64 {
    g as f64 * GRID_SIZE
}

/// Конвертировать integer grid coordinate → метры (f32) для GPU upload.
/// Используй `grid_to_f64` для CAD вычислений.
#[inline]
pub fn grid_to_f32(g: i32) -> f32 {
    (g as f64 * GRID_SIZE) as f32
}
