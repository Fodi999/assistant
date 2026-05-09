//! Primitive shape generators — square, rectangle, circle, triangle, cube, sphere, line.

use std::f32::consts::{PI, TAU};

use crate::infrastructure::geometry::kernel::extrude::{extrude_polygon, ExtrudeOptions, Point2};
use crate::infrastructure::geometry::kernel::{GeometryQuality, MeshBuilder};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

fn extrude_single(
    pts: &[Point2],
    depth: f32,
    color_hex: &str,
    group_name: &str,
    metalness: f32,
    roughness: f32,
) -> Mesh {
    let opts = ExtrudeOptions { depth, bevel: 0.0 };
    let [front, back, sides] = extrude_polygon(pts, &opts).expect("primitives: degenerate polygon");
    let color = hex_to_rgb(color_hex);
    let mut b = MeshBuilder::new();
    let g = b.add_group(
        Material::solid(group_name, color)
            .with_pbr(roughness, metalness)
            .with_class("opaque"),
    );
    b.add_part(g, &front);
    b.add_part(g, &back);
    b.add_part(g, &sides);
    b.build()
}

pub fn generate_square(color_hex: &str) -> Mesh {
    let h = 0.5_f32; // 1m × 1m, 5cm thick
    let pts = [
        Point2::new(-h, -h),
        Point2::new(h, -h),
        Point2::new(h, h),
        Point2::new(-h, h),
    ];
    extrude_single(&pts, 0.05, color_hex, "shape_square", 0.05, 0.55)
}

pub fn generate_rectangle(color_hex: &str) -> Mesh {
    let pts = [
        Point2::new(-0.8, -0.5),
        Point2::new(0.8, -0.5),
        Point2::new(0.8, 0.5),
        Point2::new(-0.8, 0.5),
    ];
    extrude_single(&pts, 0.05, color_hex, "shape_rectangle", 0.05, 0.55)
}

pub fn generate_triangle(color_hex: &str) -> Mesh {
    let r = 1.0_f32 / 3.0_f32.sqrt();
    let pts: Vec<Point2> = (0..3)
        .map(|i| {
            let a = PI / 2.0 + i as f32 * TAU / 3.0;
            Point2::new(r * a.cos(), r * a.sin())
        })
        .collect();
    extrude_single(&pts, 0.05, color_hex, "shape_triangle", 0.05, 0.55)
}

pub fn generate_circle(color_hex: &str, quality: GeometryQuality) -> Mesh {
    let segs: usize = match quality {
        GeometryQuality::Draft => 32,
        GeometryQuality::Standard => 48,
        GeometryQuality::High => 64,
        GeometryQuality::Ultra => 96,
    };
    let r = 0.6_f32;
    let pts: Vec<Point2> = (0..segs)
        .map(|i| {
            let a = i as f32 * TAU / segs as f32;
            Point2::new(r * a.cos(), r * a.sin())
        })
        .collect();
    extrude_single(&pts, 0.05, color_hex, "shape_circle", 0.05, 0.50)
}

/// Manual box mesh — 24 vertices (4 per face) for proper flat shading.
/// Avoids extrude_polygon entirely so we get a guaranteed-correct cube.
pub fn generate_cube(color_hex: &str) -> Mesh {
    generate_cube_grid(color_hex, 1, 0.0)
}

/// Subdivided cube with optional corner bevel — Plasticity-style.
///
/// * `subdivisions` — number of edge splits per face axis (1 = 2×2 quads/face,
///   2 = 3×3, 4 = 5×5). Default 1 is equivalent to the old flat cube.
/// * `bevel` — corner rounding strength 0.0 (sharp) … 1.0 (sphere).
///   Blends corner vertices toward the circumscribed sphere, producing
///   smooth chamfered edges exactly like the Plasticity "Bevel" handle.
pub fn generate_cube_grid(color_hex: &str, subdivisions: u32, bevel: f32) -> Mesh {
    let s = 0.5_f32; // half-extent → cube is 1 m³
    let n = subdivisions.max(1) as usize;
    let bevel = bevel.clamp(0.0, 1.0);
    let color = hex_to_rgb(color_hex);
    let mut b = MeshBuilder::new();
    let g = b.add_group(
        Material::solid("shape_cube", color)
            .with_pbr(0.45, 0.0)
            .with_class("opaque"),
    );

    // Each face defined by: corner origin + u_step + v_step + face normal.
    // Winding is CCW when viewed from *outside* the cube, i.e. the geometric
    // normal `u_axis × v_axis` MUST equal `face_normal`. Otherwise the face
    // gets back-face-culled and the cube renders as an open box.
    let face_defs: [([f32; 3], [f32; 3], [f32; 3], [f32; 3]); 6] = [
        // +Z front:  u=+X, v=+Y, u×v=+Z ✓
        (
            [-s, -s, s],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ),
        // -Z back:   u=-X, v=+Y, u×v=-Z ✓
        (
            [s, -s, -s],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0],
        ),
        // +X right:  u=+Y, v=+Z, u×v=+X ✓  (was u=+Z,v=+Y → -X bug)
        (
            [s, -s, -s],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
        ),
        // -X left:   u=+Y, v=-Z, u×v=-X ✓  (was u=-Z,v=+Y → +X bug)
        (
            [-s, -s, s],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0],
            [-1.0, 0.0, 0.0],
        ),
        // +Y top:    u=+X, v=-Z, u×v=+Y ✓
        (
            [-s, s, s],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
        ),
        // -Y bottom: u=+X, v=+Z, u×v=-Y ✓
        (
            [-s, -s, -s],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, -1.0, 0.0],
        ),
    ];

    let step = 1.0_f32 / n as f32;

    for (origin, u_axis, v_axis, face_normal) in &face_defs {
        // Build (n+1)×(n+1) vertex grid for this face
        let mut idx_grid: Vec<usize> = Vec::with_capacity((n + 1) * (n + 1));

        for vi in 0..=(n) {
            for ui in 0..=(n) {
                let tu = ui as f32 * step; // 0..1
                let tv = vi as f32 * step; // 0..1

                // Raw cube position
                let mut pos = [
                    origin[0] + u_axis[0] * tu * 2.0 * s + v_axis[0] * tv * 2.0 * s,
                    origin[1] + u_axis[1] * tu * 2.0 * s + v_axis[1] * tv * 2.0 * s,
                    origin[2] + u_axis[2] * tu * 2.0 * s + v_axis[2] * tv * 2.0 * s,
                ];

                // Normal: starts as face normal, blends toward sphere normal at
                // corners (bevel > 0).
                let mut normal = *face_normal;

                if bevel > 0.0 {
                    let (bp, bn) = bevel_vertex(pos, *face_normal, s, bevel);
                    pos = bp;
                    normal = bn;
                }

                idx_grid.push(b.add_vertex(pos, normal, [tu, tv]));
            }
        }

        // Emit quads (two triangles each)
        for vi in 0..n {
            for ui in 0..n {
                let i00 = idx_grid[vi * (n + 1) + ui];
                let i10 = idx_grid[vi * (n + 1) + ui + 1];
                let i11 = idx_grid[(vi + 1) * (n + 1) + ui + 1];
                let i01 = idx_grid[(vi + 1) * (n + 1) + ui];
                b.add_quad(g, i00, i10, i11, i01);
            }
        }
    }

    b.build()
}

/// Apply Plasticity-style corner bevel to a cube vertex.
///
/// Corners are identified by their proximity to the cube diagonal:
/// the closer a vertex is to a corner the more it blends toward the
/// circumscribed sphere (radius = s√3).
///
/// Returns `(new_position, new_normal)`.
fn bevel_vertex(pos: [f32; 3], face_normal: [f32; 3], s: f32, bevel: f32) -> ([f32; 3], [f32; 3]) {
    // Distance from origin (cube center)
    let len = (pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt();
    let max_len = s * 1.732_f32; // √3 — distance to a cube corner

    // "Corner factor": 0 at face centre, 1 at pure corner vertex
    // Face-centre vertices have one coordinate = ±s and the other two = 0.
    // Corner vertices have all three coordinates = ±s.
    let corner_factor = ((len - s) / (max_len - s)).clamp(0.0, 1.0);
    let t = bevel * corner_factor;

    if t < 1e-6 {
        return (pos, face_normal);
    }

    // Sphere position: same direction, radius = s (inscribed sphere is s,
    // but we blend to circumscribed = s√3 normalised back to s to keep
    // cube size).
    let sphere_pos = if len > 1e-9 {
        [
            pos[0] / len * s * 1.732_f32 / 1.732_f32, // = pos/len * s
            pos[1] / len * s,
            pos[2] / len * s,
        ]
    } else {
        pos
    };

    let new_pos = [
        pos[0] * (1.0 - t) + sphere_pos[0] * t,
        pos[1] * (1.0 - t) + sphere_pos[1] * t,
        pos[2] * (1.0 - t) + sphere_pos[2] * t,
    ];

    // Normal: blend face normal → sphere (radial) normal
    let sphere_normal = if len > 1e-9 {
        [pos[0] / len, pos[1] / len, pos[2] / len]
    } else {
        face_normal
    };
    let blended = [
        face_normal[0] * (1.0 - t) + sphere_normal[0] * t,
        face_normal[1] * (1.0 - t) + sphere_normal[1] * t,
        face_normal[2] * (1.0 - t) + sphere_normal[2] * t,
    ];
    let nlen = (blended[0] * blended[0] + blended[1] * blended[1] + blended[2] * blended[2])
        .sqrt()
        .max(1e-9);
    let new_normal = [blended[0] / nlen, blended[1] / nlen, blended[2] / nlen];

    (new_pos, new_normal)
}

pub fn generate_sphere(color_hex: &str, quality: GeometryQuality) -> Mesh {
    use crate::infrastructure::geometry::generators::hard_surface::organic_sphere::{
        generate_organic_sphere, OrganicSphereSpec,
    };
    generate_organic_sphere(&OrganicSphereSpec {
        radius: 0.6,
        color_hex: color_hex.to_string(),
        ..OrganicSphereSpec::with_quality(quality)
    })
}

pub fn generate_line(color_hex: &str) -> Mesh {
    let pts = [
        Point2::new(-1.0, -0.03),
        Point2::new(1.0, -0.03),
        Point2::new(1.0, 0.03),
        Point2::new(-1.0, 0.03),
    ];
    extrude_single(&pts, 0.03, color_hex, "shape_line", 0.0, 0.6)
}

// ─────────────────────────────────────────────────────────────────────────────
// Parasolid-style cylinder / cone / torus
//
// All three follow the same contract:
//   • Watertight topology (caps + side meet, no T-junctions).
//   • Split normals at hard edges (cap/side share position but not normal),
//     so flat caps stay flat and curved sides stay smooth.
//   • UVs follow the natural surface parameterisation (u = angle / 2π).
//   • Dimensions in metres, centred at origin, +Y up.
// ─────────────────────────────────────────────────────────────────────────────

/// Capped right cylinder centred at origin, axis = +Y.
///
/// * `radius`  — base radius in metres (default 0.5)
/// * `height`  — full height in metres (default 1.0)
/// * `quality` — drives radial segment count
pub fn generate_cylinder(
    color_hex: &str,
    radius: f32,
    height: f32,
    quality: GeometryQuality,
) -> Mesh {
    let r = radius.max(1e-4);
    let h = height.max(1e-4);
    let segs = quality.radial_segments().max(8);
    let half = h * 0.5;
    let color = hex_to_rgb(color_hex);
    let mut b = MeshBuilder::new();
    let g = b.add_group(
        Material::solid("shape_cylinder", color)
            .with_pbr(0.45, 0.0)
            .with_class("opaque"),
    );

    // ── Side (smooth radial normals, split from caps) ──
    let mut side_top: Vec<usize> = Vec::with_capacity(segs + 1);
    let mut side_bot: Vec<usize> = Vec::with_capacity(segs + 1);
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let a = t * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        let n = [ca, 0.0, sa];
        side_top.push(b.add_vertex([r * ca, half, r * sa], n, [t, 1.0]));
        side_bot.push(b.add_vertex([r * ca, -half, r * sa], n, [t, 0.0]));
    }
    for i in 0..segs {
        b.add_quad(
            g,
            side_bot[i],
            side_bot[i + 1],
            side_top[i + 1],
            side_top[i],
        );
    }

    // ── Top cap (+Y normal, separate vertex ring) ──
    let top_center = b.add_vertex([0.0, half, 0.0], [0.0, 1.0, 0.0], [0.5, 0.5]);
    let mut top_rim: Vec<usize> = Vec::with_capacity(segs + 1);
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let a = t * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        top_rim.push(b.add_vertex(
            [r * ca, half, r * sa],
            [0.0, 1.0, 0.0],
            [0.5 + 0.5 * ca, 0.5 + 0.5 * sa],
        ));
    }
    for i in 0..segs {
        b.add_triangle(g, top_center, top_rim[i], top_rim[i + 1]);
    }

    // ── Bottom cap (-Y normal) ──
    let bot_center = b.add_vertex([0.0, -half, 0.0], [0.0, -1.0, 0.0], [0.5, 0.5]);
    let mut bot_rim: Vec<usize> = Vec::with_capacity(segs + 1);
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let a = t * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        bot_rim.push(b.add_vertex(
            [r * ca, -half, r * sa],
            [0.0, -1.0, 0.0],
            [0.5 + 0.5 * ca, 0.5 - 0.5 * sa],
        ));
    }
    for i in 0..segs {
        // CCW when viewed from below (= -Y)
        b.add_triangle(g, bot_center, bot_rim[i + 1], bot_rim[i]);
    }

    b.build()
}

/// Cone / frustum centred at origin, axis = +Y.
///
/// * `radius_bottom` — bottom radius in metres
/// * `radius_top`    — top radius in metres (0.0 → pure cone)
/// * `height`        — full height
///
/// Side normals are tilted by the cone half-angle so shading is correct.
pub fn generate_cone(
    color_hex: &str,
    radius_bottom: f32,
    radius_top: f32,
    height: f32,
    quality: GeometryQuality,
) -> Mesh {
    let r0 = radius_bottom.max(0.0);
    let r1 = radius_top.max(0.0);
    let h = height.max(1e-4);
    let segs = quality.radial_segments().max(8);
    let half = h * 0.5;
    let color = hex_to_rgb(color_hex);
    let mut b = MeshBuilder::new();
    let g = b.add_group(
        Material::solid("shape_cone", color)
            .with_pbr(0.45, 0.0)
            .with_class("opaque"),
    );

    // Slope vector for normal tilt: dr/dy in the meridian plane.
    // Side normal at angle a is (cos·cosθ, sinθ, sin·cosθ) where
    // tanθ = (r0 − r1) / h (cone leans inward at the top when r1 < r0).
    let dr = r0 - r1;
    let slope_len = (dr * dr + h * h).sqrt().max(1e-9);
    let n_y = dr / slope_len; // +Y component (positive when narrowing toward top)
    let n_xz = h / slope_len; // radial component

    // ── Side ──
    let mut top: Vec<usize> = Vec::with_capacity(segs + 1);
    let mut bot: Vec<usize> = Vec::with_capacity(segs + 1);
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let a = t * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        let n = [ca * n_xz, n_y, sa * n_xz];
        top.push(b.add_vertex([r1 * ca, half, r1 * sa], n, [t, 1.0]));
        bot.push(b.add_vertex([r0 * ca, -half, r0 * sa], n, [t, 0.0]));
    }
    for i in 0..segs {
        if r1 < 1e-5 {
            // Pure tip → emit triangle (skip degenerate quad)
            b.add_triangle(g, bot[i], bot[i + 1], top[i]);
        } else {
            b.add_quad(g, bot[i], bot[i + 1], top[i + 1], top[i]);
        }
    }

    // ── Bottom cap (always present) ──
    if r0 > 1e-5 {
        let center = b.add_vertex([0.0, -half, 0.0], [0.0, -1.0, 0.0], [0.5, 0.5]);
        let mut rim: Vec<usize> = Vec::with_capacity(segs + 1);
        for i in 0..=segs {
            let t = i as f32 / segs as f32;
            let a = t * TAU;
            let (sa, ca) = (a.sin(), a.cos());
            rim.push(b.add_vertex(
                [r0 * ca, -half, r0 * sa],
                [0.0, -1.0, 0.0],
                [0.5 + 0.5 * ca, 0.5 - 0.5 * sa],
            ));
        }
        for i in 0..segs {
            b.add_triangle(g, center, rim[i + 1], rim[i]);
        }
    }

    // ── Top cap (only for frustum) ──
    if r1 > 1e-5 {
        let center = b.add_vertex([0.0, half, 0.0], [0.0, 1.0, 0.0], [0.5, 0.5]);
        let mut rim: Vec<usize> = Vec::with_capacity(segs + 1);
        for i in 0..=segs {
            let t = i as f32 / segs as f32;
            let a = t * TAU;
            let (sa, ca) = (a.sin(), a.cos());
            rim.push(b.add_vertex(
                [r1 * ca, half, r1 * sa],
                [0.0, 1.0, 0.0],
                [0.5 + 0.5 * ca, 0.5 + 0.5 * sa],
            ));
        }
        for i in 0..segs {
            b.add_triangle(g, center, rim[i], rim[i + 1]);
        }
    }

    b.build()
}

/// Torus centred at origin, axis = +Y.
///
/// * `major_radius` — distance from origin to ring centre
/// * `minor_radius` — tube radius
/// * `quality`      — major segments from quality, minor = ⌈major / 3⌉
pub fn generate_torus(
    color_hex: &str,
    major_radius: f32,
    minor_radius: f32,
    quality: GeometryQuality,
) -> Mesh {
    let r_major = major_radius.max(1e-4);
    let r_minor = minor_radius.max(1e-4).min(r_major - 1e-4);
    let major_segs = quality.radial_segments().max(12);
    let minor_segs = (major_segs / 3).max(8);
    let color = hex_to_rgb(color_hex);
    let mut b = MeshBuilder::new();
    let g = b.add_group(
        Material::solid("shape_torus", color)
            .with_pbr(0.42, 0.05)
            .with_class("opaque"),
    );

    // Build (major+1)×(minor+1) grid (closed torus needs duplicate rim for UV seam).
    let mut grid: Vec<usize> = Vec::with_capacity((major_segs + 1) * (minor_segs + 1));
    for i in 0..=major_segs {
        let u = i as f32 / major_segs as f32;
        let a = u * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        let ring_center = [r_major * ca, 0.0, r_major * sa];
        for j in 0..=minor_segs {
            let v = j as f32 / minor_segs as f32;
            let p = v * TAU;
            let (sp, cp) = (p.sin(), p.cos());
            // Position: ring_center + minor_radius · (cp · radial + sp · up)
            let pos = [
                ring_center[0] + r_minor * cp * ca,
                ring_center[1] + r_minor * sp,
                ring_center[2] + r_minor * cp * sa,
            ];
            // Smooth normal points from the ring centre outward through pos.
            let nx = cp * ca;
            let ny = sp;
            let nz = cp * sa;
            grid.push(b.add_vertex(pos, [nx, ny, nz], [u, v]));
        }
    }

    let stride = minor_segs + 1;
    for i in 0..major_segs {
        for j in 0..minor_segs {
            let i00 = grid[i * stride + j];
            let i10 = grid[(i + 1) * stride + j];
            let i11 = grid[(i + 1) * stride + j + 1];
            let i01 = grid[i * stride + j + 1];
            b.add_quad(g, i00, i10, i11, i01);
        }
    }

    b.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::kernel::validate::validate_mesh;
    #[test]
    fn square_valid() {
        validate_mesh(&generate_square("#38BDF8")).unwrap();
    }
    #[test]
    fn rectangle_valid() {
        validate_mesh(&generate_rectangle("#A78BFA")).unwrap();
    }
    #[test]
    fn triangle_valid() {
        validate_mesh(&generate_triangle("#FB923C")).unwrap();
    }
    #[test]
    fn circle_valid() {
        validate_mesh(&generate_circle("#34D399", GeometryQuality::Draft)).unwrap();
    }
    #[test]
    fn cube_valid() {
        validate_mesh(&generate_cube("#F472B6")).unwrap();
    }
    #[test]
    fn sphere_valid() {
        validate_mesh(&generate_sphere("#FACC15", GeometryQuality::Draft)).unwrap();
    }
    #[test]
    fn line_valid() {
        validate_mesh(&generate_line("#94A3B8")).unwrap();
    }
    #[test]
    fn cylinder_valid() {
        validate_mesh(&generate_cylinder(
            "#38BDF8",
            0.5,
            1.0,
            GeometryQuality::Draft,
        ))
        .unwrap();
    }
    #[test]
    fn cone_valid() {
        validate_mesh(&generate_cone(
            "#FB923C",
            0.5,
            0.0,
            1.0,
            GeometryQuality::Draft,
        ))
        .unwrap();
    }
    #[test]
    fn frustum_valid() {
        validate_mesh(&generate_cone(
            "#FB923C",
            0.5,
            0.25,
            1.0,
            GeometryQuality::Draft,
        ))
        .unwrap();
    }
    #[test]
    fn torus_valid() {
        validate_mesh(&generate_torus(
            "#A78BFA",
            0.5,
            0.15,
            GeometryQuality::Draft,
        ))
        .unwrap();
    }

    // ── Plasticity-style cube tests ──────────────────────────────────────────
    #[test]
    fn cube_grid_sharp_topology() {
        // n=1 → 6 faces × 4 vertices = 24 vertices, 12 triangles.
        let m = generate_cube_grid("#F472B6", 1, 0.0);
        validate_mesh(&m).unwrap();
        assert_eq!(m.vertices.len(), 24, "sharp cube should have 24 verts");
        assert_eq!(m.faces.len(), 12, "sharp cube should have 12 triangles");
    }

    #[test]
    fn cube_grid_subdivided_topology() {
        // n=4 → (5×5)×6 = 150 verts, (4×4×2)×6 = 192 triangles.
        let m = generate_cube_grid("#F472B6", 4, 0.0);
        validate_mesh(&m).unwrap();
        assert_eq!(m.vertices.len(), 5 * 5 * 6);
        assert_eq!(m.faces.len(), 4 * 4 * 2 * 6);
    }

    #[test]
    fn cube_grid_beveled_no_nan() {
        // Heavy bevel + subdivisions — all coords / normals must be finite.
        let m = generate_cube_grid("#F472B6", 4, 0.7);
        validate_mesh(&m).unwrap();
        for p in &m.vertices {
            assert!(
                p.iter().all(|c| c.is_finite()),
                "position has NaN/Inf: {:?}",
                p
            );
            // Beveled vertex must stay inside the unit cube (corners pull inward).
            assert!(
                p.iter().all(|c: &f32| c.abs() <= 0.5 + 1e-4),
                "beveled vertex outside cube bounds: {:?}",
                p
            );
        }
        for n in &m.normals {
            assert!(
                n.iter().all(|c| c.is_finite()),
                "normal has NaN/Inf: {:?}",
                n
            );
        }
    }

    #[test]
    fn cube_grid_bevel_corners_pull_inward() {
        // A pure corner vertex (±0.5, ±0.5, ±0.5) on an unbeveled cube has
        // length √(3)/2 ≈ 0.866. With bevel=1 it should be pulled to length s=0.5.
        let sharp = generate_cube_grid("#F472B6", 1, 0.0);
        let smooth = generate_cube_grid("#F472B6", 1, 1.0);

        let max_len = |m: &Mesh| -> f32 {
            m.vertices
                .iter()
                .map(|p| (p[0].powi(2) + p[1].powi(2) + p[2].powi(2)).sqrt())
                .fold(0.0_f32, f32::max)
        };
        let max_sharp = max_len(&sharp);
        let max_smooth = max_len(&smooth);

        assert!(
            (max_sharp - 0.866).abs() < 0.01,
            "sharp corner len ≈ √3/2, got {max_sharp}"
        );
        assert!(
            max_smooth < max_sharp - 0.1,
            "bevel=1 must pull corners inward (got {max_smooth} vs {max_sharp})"
        );
    }

    /// Regression: every triangle's geometric normal (from CCW winding) must
    /// agree with its declared vertex normal. If any face is wound CW the dot
    /// product flips sign and back-face culling makes the cube look like an
    /// open box. (Bug discovered on 2026-05-04 when +X / -X side faces had
    /// `u_axis × v_axis = -face_normal`.)
    #[test]
    fn cube_faces_wound_outward() {
        for &(n_sub, label) in &[(1u32, "sharp"), (3u32, "subdivided")] {
            let m = generate_cube_grid("#F472B6", n_sub, 0.0);
            for (i, tri) in m.faces.iter().enumerate() {
                let p0 = m.vertices[tri[0]];
                let p1 = m.vertices[tri[1]];
                let p2 = m.vertices[tri[2]];
                let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
                let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
                // geometric normal = e1 × e2
                let geo = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                let n0 = m.normals[tri[0]];
                let dot = geo[0] * n0[0] + geo[1] * n0[1] + geo[2] * n0[2];
                assert!(
                    dot > 0.0,
                    "{label} cube tri #{i} ({:?}) wound inward: geo·normal = {dot}",
                    tri
                );
            }
        }
    }

    // ── Parasolid-style cylinder / cone / torus ──────────────────────────────
    #[test]
    fn cylinder_topology() {
        // Draft → 32 segments. Side: 2·(segs+1) split-seam verts; caps: 2 centres + 2·(segs+1) rim.
        let m = generate_cylinder("#38BDF8", 0.5, 1.0, GeometryQuality::Draft);
        validate_mesh(&m).unwrap();
        let segs = 32_usize;
        assert_eq!(m.vertices.len(), 2 * (segs + 1) + 2 + 2 * (segs + 1));
        assert_eq!(m.faces.len(), 4 * segs);
    }

    #[test]
    fn cylinder_watertight_height() {
        let m = generate_cylinder("#38BDF8", 0.5, 2.0, GeometryQuality::Draft);
        let max_y = m.vertices.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        let min_y = m.vertices.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        assert!(
            (max_y - 1.0).abs() < 1e-4,
            "top y must be +half, got {max_y}"
        );
        assert!(
            (min_y + 1.0).abs() < 1e-4,
            "bottom y must be -half, got {min_y}"
        );
    }

    #[test]
    fn cone_apex_collapses() {
        let m = generate_cone("#FB923C", 0.5, 0.0, 1.0, GeometryQuality::Draft);
        validate_mesh(&m).unwrap();
        let half = 0.5_f32;
        let apex_count = m
            .vertices
            .iter()
            .filter(|p| p[0].abs() < 1e-5 && p[2].abs() < 1e-5 && (p[1] - half).abs() < 1e-5)
            .count();
        assert!(
            apex_count >= 32,
            "cone tip must collapse to (0,0) in xz, got {apex_count}"
        );
    }

    #[test]
    fn cone_normals_tilted_for_pure_cone() {
        let m = generate_cone("#FB923C", 0.5, 0.0, 1.0, GeometryQuality::Draft);
        let positive_y_normals = m.normals.iter().filter(|n| n[1] > 0.1).count();
        assert!(
            positive_y_normals > 0,
            "cone side normals must tilt upward (n.y > 0) when r0 > r1"
        );
    }

    #[test]
    fn frustum_has_two_caps() {
        let m = generate_cone("#FB923C", 0.5, 0.25, 1.0, GeometryQuality::Draft);
        validate_mesh(&m).unwrap();
        let up = m
            .normals
            .iter()
            .filter(|n| (n[1] - 1.0).abs() < 1e-3)
            .count();
        let down = m
            .normals
            .iter()
            .filter(|n| (n[1] + 1.0).abs() < 1e-3)
            .count();
        assert!(up > 0, "frustum must have +Y normals (top cap)");
        assert!(down > 0, "frustum must have -Y normals (bottom cap)");
    }

    #[test]
    fn torus_topology() {
        let m = generate_torus("#A78BFA", 0.5, 0.15, GeometryQuality::Draft);
        validate_mesh(&m).unwrap();
        let major = 32_usize;
        let minor = (major / 3).max(8); // 10
        assert_eq!(m.vertices.len(), (major + 1) * (minor + 1));
        assert_eq!(m.faces.len(), major * minor * 2);
    }

    #[test]
    fn torus_vertices_lie_on_surface() {
        let r_major = 0.5_f32;
        let r_minor = 0.15_f32;
        let m = generate_torus("#A78BFA", r_major, r_minor, GeometryQuality::Draft);
        for p in &m.vertices {
            let radial = (p[0] * p[0] + p[2] * p[2]).sqrt();
            assert!(
                radial >= r_major - r_minor - 1e-3 && radial <= r_major + r_minor + 1e-3,
                "torus vertex outside tube: radial={radial}"
            );
            assert!(
                p[1].abs() <= r_minor + 1e-3,
                "torus vertex y outside tube: y={}",
                p[1]
            );
        }
    }
}
