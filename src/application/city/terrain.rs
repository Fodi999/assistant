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
/// Returns roughly `-10 .. +10` m. Three-octave sum of trigonometric waves
/// — large macro shape, medium ridges, fine detail — seeded by tenant_id.
/// Cheap, allocation-free, stable. Tuned for ~1 km city extents.
#[inline]
pub fn terrain_height(x: f32, z: f32, seed: u64) -> f32 {
    let s = (seed as f32) * 0.000_001;
    let macro_shape = ((x * 0.004  + s).sin() * 5.0)
                    + ((z * 0.0035 + s * 1.7).cos() * 4.0);
    let mid = ((x * 0.012 + z * 0.008 + s).sin() * 1.8)
            + ((z * 0.015 - x * 0.009 + s).cos() * 1.4);
    let detail = ((x * 0.05 + z * 0.03 + s).sin() * 0.35)
               + ((z * 0.06 - x * 0.02 + s).cos() * 0.25);
    macro_shape + mid + detail
}

// ─────────────────────────────────────────────────────────────────────────────
// Colour mapping
// ─────────────────────────────────────────────────────────────────────────────

/// Map a terrain height (metres) to a linear-RGB triple `[r, g, b]` in 0..1.
///
/// Colour stops (blended with smooth Hermite interpolation):
///
/// | height (m) | colour          | feel        |
/// |------------|-----------------|-------------|
/// | ≤ −8       | `#253d18`       | dark wetland|
/// | −4         | `#35571e`       | wet grass   |
/// |  0         | `#4a6830`       | grass       |
/// |  4         | `#6a6a40`       | dry plateau |
/// |  8         | `#7c7060`       | dusty earth |
/// | ≥ 12       | `#8e8880`       | rocky grey  |
fn height_to_rgb(h: f32) -> [f32; 3] {
    // Each entry: (threshold, [r,g,b])
    const STOPS: &[(f32, [f32; 3])] = &[
        (-8.0,  [0.145, 0.239, 0.094]),  // dark wetland
        (-4.0,  [0.208, 0.341, 0.118]),  // wet grass
        ( 0.0,  [0.290, 0.408, 0.188]),  // normal grass
        ( 4.0,  [0.416, 0.416, 0.251]),  // dry plateau
        ( 8.0,  [0.486, 0.439, 0.376]),  // dusty earth
        (12.0,  [0.557, 0.533, 0.502]),  // rocky grey
    ];

    if h <= STOPS[0].0 { return STOPS[0].1; }
    if h >= STOPS[STOPS.len() - 1].0 { return STOPS[STOPS.len() - 1].1; }

    for i in 1..STOPS.len() {
        let (t0, c0) = STOPS[i - 1];
        let (t1, c1) = STOPS[i];
        if h <= t1 {
            let t = (h - t0) / (t1 - t0);
            // Smoothstep (Hermite)
            let s = t * t * (3.0 - 2.0 * t);
            return [
                c0[0] + (c1[0] - c0[0]) * s,
                c0[1] + (c1[1] - c0[1]) * s,
                c0[2] + (c1[2] - c0[2]) * s,
            ];
        }
    }
    STOPS[STOPS.len() - 1].1
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge taper
// ─────────────────────────────────────────────────────────────────────────────

/// Returns a 0..1 fade factor (1 = interior, 0 = border).
/// Vertices near the border are lerped toward a deep underground y so the
/// terrain edge sinks below the city ground plane instead of forming a
/// visible cliff.
#[inline]
fn edge_fade(col: usize, row: usize, cols: usize, rows: usize, fade_cells: usize) -> f32 {
    let fx = (col.min(cols.saturating_sub(col)) as f32 / fade_cells as f32).min(1.0);
    let fz = (row.min(rows.saturating_sub(row)) as f32 / fade_cells as f32).min(1.0);
    let f  = fx.min(fz);
    // Smoothstep for a softer taper curve.
    f * f * (3.0 - 2.0 * f)
}

// ─────────────────────────────────────────────────────────────────────────────
// Mesh builder
// ─────────────────────────────────────────────────────────────────────────────

/// Build an indexed grid mesh sampling `terrain_height`.
///
/// Extras over a plain grid:
///   - **Vertex colours** (`colors` buffer, flat RGB 0..1) — height-based
///     colour ramp so the frontend can use `vertexColors: true` without any
///     extra texture.
///   - **Edge taper** — border vertices are sunk to `EDGE_SINK_Y` (well below
///     city ground level) via a smooth Hermite fade so the terrain edge
///     disappears underground rather than forming a hard visible cliff.
///   - **Smooth normals** — per-vertex normals from averaged face normals.
pub fn build_terrain_mesh(width: f32, depth: f32, cell_size: f32, seed: u64) -> CityMesh {
    /// How many cells from each edge fade to underground.
    const FADE_CELLS: usize = 14;
    /// Y value edge vertices sink to (below deepest visible ground).
    const EDGE_SINK_Y: f32 = -30.0;

    let cols = (width  / cell_size).ceil() as usize;
    let rows = (depth  / cell_size).ceil() as usize;

    let stride = cols + 1;
    let vert_count = stride * (rows + 1);

    let mut positions: Vec<f32> = Vec::with_capacity(vert_count * 3);
    let mut normals:   Vec<f32> = vec![0.0; vert_count * 3];
    let mut uvs:       Vec<f32> = Vec::with_capacity(vert_count * 2);
    let mut colors:    Vec<f32> = Vec::with_capacity(vert_count * 3);
    let mut indices:   Vec<u32> = Vec::with_capacity(rows * cols * 6);

    let half_w = width  * 0.5;
    let half_d = depth  * 0.5;

    // 1. Vertex positions, UVs, vertex colours.
    for row in 0..=rows {
        for col in 0..cols + 1 {
            let x = -half_w + (col as f32) * cell_size;
            let z = -half_d + (row as f32) * cell_size;

            let h_raw = terrain_height(x, z, seed);

            // Smoothly sink edge vertices underground.
            let fade = edge_fade(col, row, cols, rows, FADE_CELLS);
            let y = h_raw * fade + EDGE_SINK_Y * (1.0 - fade);

            positions.push(x);
            positions.push(y);
            positions.push(z);

            uvs.push((col as f32) / (cols as f32));
            uvs.push((row as f32) / (rows as f32));

            // Colour based on raw height (before taper) so edge colour
            // blends to the darkest wetland stop naturally.
            let rgb = height_to_rgb(h_raw * fade);
            colors.push(rgb[0]);
            colors.push(rgb[1]);
            colors.push(rgb[2]);
        }
    }

    // 2. Triangle indices (two per cell, CCW from +Y).
    for row in 0..rows {
        for col in 0..cols {
            let a = (row * stride + col) as u32;
            let b = (row * stride + col + 1) as u32;
            let c = ((row + 1) * stride + col) as u32;
            let d = ((row + 1) * stride + col + 1) as u32;
            indices.push(a); indices.push(c); indices.push(b);
            indices.push(b); indices.push(c); indices.push(d);
        }
    }

    // 3. Smooth per-vertex normals via accumulated face normals.
    for tri in indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        let p = |i: usize| [positions[i*3], positions[i*3+1], positions[i*3+2]];
        let p0 = p(i0); let p1 = p(i1); let p2 = p(i2);
        let e1 = [p1[0]-p0[0], p1[1]-p0[1], p1[2]-p0[2]];
        let e2 = [p2[0]-p0[0], p2[1]-p0[1], p2[2]-p0[2]];
        let nx = e1[1]*e2[2] - e1[2]*e2[1];
        let ny = e1[2]*e2[0] - e1[0]*e2[2];
        let nz = e1[0]*e2[1] - e1[1]*e2[0];
        for &i in &[i0, i1, i2] {
            normals[i*3]     += nx;
            normals[i*3 + 1] += ny;
            normals[i*3 + 2] += nz;
        }
    }
    for n in normals.chunks_exact_mut(3) {
        let len = (n[0]*n[0] + n[1]*n[1] + n[2]*n[2]).sqrt();
        if len > 1e-6 { n[0]/=len; n[1]/=len; n[2]/=len; }
        else { n[0]=0.0; n[1]=1.0; n[2]=0.0; }
    }

    CityMesh { positions, normals, uvs, indices, colors }
}

// ─────────────────────────────────────────────────────────────────────────────
// High-level helper used by CityEngine
// ─────────────────────────────────────────────────────────────────────────────

/// Build a complete `CityTerrain` ready to attach to `CityMap.terrain`.
pub fn build_city_terrain(width: f32, depth: f32, cell_size: f32, seed: u64) -> CityTerrain {
    let mesh = build_terrain_mesh(width, depth, cell_size, seed);

    // min/max reflect the *raw* height field (before edge taper) so HUD and
    // camera logic see meaningful landscape values, not the -30 m sink value.
    let mut min_h = f32::INFINITY;
    let mut max_h = f32::NEG_INFINITY;
    let half_w = width  * 0.5;
    let half_d = depth  * 0.5;
    // Sample on a coarse interior grid (skip edge strip covered by the taper).
    let step = cell_size * 4.0;
    let mut z = -half_d + step * 2.0;
    while z < half_d - step {
        let mut x = -half_w + step * 2.0;
        while x < half_w - step {
            let h = terrain_height(x, z, seed);
            if h < min_h { min_h = h; }
            if h > max_h { max_h = h; }
            x += step;
        }
        z += step;
    }
    if !min_h.is_finite() { min_h = -12.0; }
    if !max_h.is_finite() { max_h =  12.0; }

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
        // Theoretical max amplitude: 5 + 4 + 1.8 + 1.4 + 0.35 + 0.25 = 12.8
        for i in -500..=500 {
            for j in -500..=500 {
                let h = terrain_height((i * 2) as f32, (j * 2) as f32, 12345);
                assert!(h.abs() < 15.0, "height {h} out of range at ({i},{j})");
            }
        }
    }

    #[test]
    fn terrain_mesh_shapes() {
        let mesh = build_terrain_mesh(160.0, 120.0, 8.0, 7);

        let cols = (160.0_f32 / 8.0).ceil() as usize;
        let rows = (120.0_f32 / 8.0).ceil() as usize;
        let expected_verts = (cols + 1) * (rows + 1);

        assert_eq!(mesh.positions.len(), expected_verts * 3);
        assert_eq!(mesh.normals.len(),   expected_verts * 3);
        assert_eq!(mesh.uvs.len(),       expected_verts * 2);
        assert_eq!(mesh.indices.len(),   rows * cols * 6);
    }

    #[test]
    fn terrain_mesh_normals_unit_and_up() {
        // Sample a city-scale chunk so normals reflect the gentle slopes.
        let mesh = build_terrain_mesh(400.0, 400.0, 8.0, 99);
        for n in mesh.normals.chunks_exact(3) {
            let len = (n[0]*n[0] + n[1]*n[1] + n[2]*n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-3, "non-unit normal: {len}");
            // Soft terrain → max slope ≈ 0.06 → cos(slope) > 0.99 → normal.y > 0.5 always
            assert!(n[1] > 0.5, "normal not pointing up enough: {:?}", n);
        }
    }

    #[test]
    fn city_terrain_min_max_consistent() {
        let t = build_city_terrain(1000.0, 1000.0, 8.0, 1);
        assert!(t.min_height <= t.max_height);
        assert!(t.min_height >= -15.0);
        assert!(t.max_height <=  15.0);
        // On a 1 km square we should actually exercise both signs.
        assert!(t.min_height <  0.0, "expected some negative terrain, got {}", t.min_height);
        assert!(t.max_height >  0.0, "expected some positive terrain, got {}", t.max_height);
    }
}
