//! Константы точности и эпсилон-сравнения.

/// Минимальная длина ребра для CAD операций (метры).
pub const EDGE_EPS: f32 = 1e-8;

/// Эпсилон для нормалей.
pub const NORMAL_EPS: f32 = 1e-6;

/// Эпсилон для площади полигона.
pub const AREA_EPS: f32 = 1e-10;

/// Эпсилон для weld вершин (2 точки считаются одной если ближе этого расстояния).
pub const WELD_EPS: f32 = 1e-5;

/// Вспомогательная структура — хранит параметры точности как значение.
#[derive(Debug, Clone, Copy)]
pub struct Tolerance {
    /// Tessellation chord tolerance.
    pub chord: f32,
    /// Vertex weld distance.
    pub weld: f32,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self { chord: 0.01, weld: WELD_EPS }
    }
}
