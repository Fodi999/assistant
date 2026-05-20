//! Константы точности и эпсилон-сравнения.
//! Все значения в единицах Real (f64) — метры.

use crate::math::Real;

/// Минимальная длина ребра для CAD операций (метры).
pub const EDGE_EPS:   Real = 1e-12;

/// Эпсилон для нормалей.
pub const NORMAL_EPS: Real = 1e-10;

/// Эпсилон для площади полигона.
pub const AREA_EPS:   Real = 1e-14;

/// Эпсилон для weld вершин. f64 позволяет различать точки на расстоянии 1 нм.
pub const WELD_EPS:   Real = 1e-9;

/// Вспомогательная структура — хранит параметры точности как значение.
#[derive(Debug, Clone, Copy)]
pub struct Tolerance {
    /// Tessellation chord tolerance (metres).
    pub chord: Real,
    /// Vertex weld distance (metres).
    pub weld: Real,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self { chord: 0.01, weld: WELD_EPS }
    }
}
