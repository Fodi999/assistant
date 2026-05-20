//! Миллиметры ↔ метры.

#[inline]
pub fn mm_to_m(mm: f32) -> f32 { mm * 0.001 }

#[inline]
pub fn m_to_mm(m: f32) -> f32 { m * 1000.0 }

#[inline]
pub fn mm_to_m_f64(mm: f64) -> f64 { mm * 0.001 }

#[inline]
pub fn m_to_mm_f64(m: f64) -> f64 { m * 1000.0 }
