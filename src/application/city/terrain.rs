//! Backend terrain generator.
//!
//! Produces a `CityTerrain` (mesh + metadata) for a CityMap.
//! Frontend uploads `terrain.mesh` directly to a `BufferGeometry` — zero
//! geometry math on the client.
//!
//! Pipeline:
//!   terrain_height(x, z, seed)  →  build_terrain_mesh()  →  CityTerrain
//!
//! The terrain is intentionally soft (≈ ±1.2m) so districts/roads/buildings
//! that still sit on `y = 0` look natural over it. Later we can flatten
//! pads under districts and roads in `terrain_height_with_flatten`.

use crate::domain::city::{CityMesh, CityTerrain};

// ─────────────────────────────────────────────────────────────────────────────
// Height field
// ─────────────────────────────────────────────────────────────────────────────

/// Smooth, deterministic terrain height in metres.
///
/// Returns roughly `-1.2 ..= +1.2`. No real Perlin/Simplex — just a sum of
/// trigonometric waves seeded by tenant_id. Cheap, allocation-free, stable.
#[inline]
pub fn terrain_height(x: f32, z: f32, seed: u64) -> f32 {
    // Convert seed to a small phase offset so different tenants get different
    // (but still smooth) landscapes.
    let s = (seed as f32) * 0.000_001;

    let large = ((x * 0.018 + s).sin() * 0.6)
              + ((z * 0.014 + s * 1.7).cos() * 0.5);

    let small = ((x * 0.055 + z * 0.025 + s).sin() * 0.18)
              + ((z * 0.045 - x * 0.018 + s).cos() * 0.12);

    (large + small) * 0.9
}

// ─────────────────────────────────────────────────────────────────────────────
// Mesh builder
// ─────────────────────────────────────────────────────────────────────────────

/// Build an indexed grid mesh sampling `terrain_height`.
///
/// Layout:
///   - Centred at origin (XZ ∈ [-w/2, +w/2] × [-d/2, +d/2])
///   - `cell_size` controls density (smaller = more triangles)
///   - Normals are computed via cross-product of triangle edges, then averaged
///     per-vertex (smooth shading).
pub fn build_terrain_mesh(width: f32, depth: f32, cell_size: f32, seed: u64) -> CityMesh {
    let cols = (width / cell_size).ceil() as usize;
    let rows = (depth / cell_size).ceil() as usize;

    let stride = cols + 1;
    let vert_count = stride * (rows + 1);

    let mut positions: Vec<f32> = Vec::with_capacity(vert_count * 3);
    let mut normals:   Vec<f32> = vec![0.0; vert_count * 3];
    let mut uvs:       Vec<f32> = Vec::with_capacity(vert_count * 2);
    let mut indices:   Vec<u32> = Vec::with_capacity(rows * cols * 6);

    let half_w = width * 0.5;
    let half_d = depth * 0.5;

    // 1. Vertex positions + UVs
    for row in 0..=rows {
        for col in 0..=stride - 1 {
            let x = -half_w + (col as f32) * cell_size;
            let z = -half_d + (row as f32) * cell_size;
            let y = terrain_height(x, z, seed);

            positions.push(x);
            positions.push(y);
            positions.push(z);

            uvs.push((col as f32) / (cols as f32));
            uvs.push((row as f32) / (rows as f32));
        }
    }

    // 2. Triangle indices (two per cell, CCW when viewed from +Y)
    for row in 0..rows {
        for col in 0..cols {
            let a = (row * stride + col) as u32;
            let b = (row * stride + col + 1) as u32;
            let c = ((row + 1) * stride + col) as u32;
            let d = ((row + 1) * stride + col + 1) as u32;

            // Triangle 1: a → c → b
            indices.push(a);
            indices.push(c);
            indices.push(b);
            // Triangle 2: b → c → d
            indices.push(b);
            indices.push(c);
            indices.push(d);
        }
    }

    // 3. Smooth per-vertex normals — accumulate triangle face normals.
    for tri in indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;

        let p0 = [positions[i0 * 3], positions[i0 * 3 + 1], positions[i0 * 3 + 2]];
        let p1 = [positions[i1 * 3], positions[i1 * 3 + 1], positions[i1 * 3 + 2]];
        let p2 = [positions[i2 * 3], positions[i2 * 3 + 1], positions[i2 * 3 + 2]];

        let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

        // Cross product e1 × e2
        let nx = e1[1] * e2[2] - e1[2] * e2[1];
        let ny = e1[2] * e2[0] - e1[0] * e2[2];
        let nz = e1[0] * e2[1] - e1[1] * e2[0];

        for &i in &[i0, i1, i2] {
            normals[i * 3]     += nx;
            normals[i * 3 + 1] += ny;
            normals[i * 3 + 2] += nz;
        }
    }

    // Normalise each accumulated normal.
    for n in normals.chunks_exact_mut(3) {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 1e-6 {
            n[0] /= len;
            n[1] /= len;
            n[2] /= len;
        } else {
            n[0] = 0.0;
            n[1] = 1.0;
            n[2] = 0.0;
        }
    }

    CityMesh {
        positions,
        normals,
        uvs,
        indices,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// High-level helper used by CityEngine
// ─────────────────────────────────────────────────────────────────────────────

/// Build a complete `CityTerrain` ready to attach to `CityMap.terrain`.
pub fn build_city_terrain(width: f32, depth: f32, cell_size: f32, seed: u64) -> CityTerrain {
    let mesh = build_terrain_mesh(width, depth, cell_size, seed);

    // Sample y values quickly to get true min/max for HUD/colour ramps.
    let mut min_h = f32::INFINITY;
    let mut max_h = f32::NEG_INFINITY;
    for chunk in mesh.positions.chunks_exact(3) {
        let y = chunk[1];
        if y < min_h { min_h = y; }
        if y > max_h { max_h = y; }
    }
    if !min_h.is_finite() { min_h = -1.5; }
    if !max_h.is_finite() { max_h =  1.5; }

    CityTerrain {
        mesh,
        width,
        depth,
        cell_size,
        min_height: min_h,
        max_height: max_h,
        color: "#5f6f52".into(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_height_deterministic() {
        assert_eq!(terrain_height(1.0, 2.0, 42), terrain_height(1.0, 2.0, 42));
    }

    #[test]
    fn terrain_height_within_bounds() {
        for i in -50..=50 {
            for j in -50..=50 {
                let h = terrain_height(i as f32, j as f32, 12345);
                assert!(h.abs() < 2.0, "height {h} out of soft range at ({i},{j})");
            }
        }
    }

    #[test]
    fn terrain_mesh_shapes() {
        let mesh = build_terrain_mesh(40.0, 30.0, 5.0, 7);

        // cols = 8, rows = 6  → (cols+1)*(rows+1) = 63 verts
        let cols = (40.0_f32 / 5.0).ceil() as usize;
        let rows = (30.0_f32 / 5.0).ceil() as usize;
        let expected_verts = (cols + 1) * (rows + 1);

        assert_eq!(mesh.positions.len(), expected_verts * 3);
        assert_eq!(mesh.normals.len(),   expected_verts * 3);
        assert_eq!(mesh.uvs.len(),       expected_verts * 2);
        assert_eq!(mesh.indices.len(),   rows * cols * 6);
    }

    #[test]
    fn terrain_mesh_normals_unit_and_up() {
        let mesh = build_terrain_mesh(20.0, 20.0, 4.0, 99);
        for n in mesh.normals.chunks_exact(3) {
            let len = (n[0]*n[0] + n[1]*n[1] + n[2]*n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-3, "non-unit normal: {len}");
            // Soft terrain → normal Y should be strongly positive.
            assert!(n[1] > 0.5, "normal not pointing up enough: {:?}", n);
        }
    }

    #[test]
    fn city_terrain_min_max_consistent() {
        let t = build_city_terrain(60.0, 60.0, 3.0, 1);
        assert!(t.min_height <= t.max_height);
        assert!(t.min_height >= -2.0);
        assert!(t.max_height <=  2.0);
    }
}
