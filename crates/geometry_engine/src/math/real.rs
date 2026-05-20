//! Numeric precision types.
//!
//! # Rationale
//! CAD kernels require f64 precision internally to avoid:
//!   - micro-gaps between adjacent faces
//!   - vertex-weld false-positives / false-negatives
//!   - unstable CSG intersections
//!   - cumulative rounding drift in large assemblies
//!
//! `GpuReal` (f32) is used **only** in the final GPU-facing types (`GpuMesh`)
//! because WebGPU vertex buffers are 32-bit.

/// Internal precision for all geometry computations.
pub type Real = f64;

/// Precision used in GPU vertex/index buffers (WebGPU).
pub type GpuReal = f32;
