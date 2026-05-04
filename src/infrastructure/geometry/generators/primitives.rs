//! Primitive shape generators — square, rectangle, circle, triangle, cube, sphere, line.

use std::f32::consts::{PI, TAU};

use crate::infrastructure::geometry::kernel::extrude::{extrude_polygon, ExtrudeOptions, Point2};
use crate::infrastructure::geometry::kernel::{GeometryQuality, MeshBuilder};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

fn extrude_single(pts: &[Point2], depth: f32, color_hex: &str, group_name: &str, metalness: f32, roughness: f32) -> Mesh {
    let opts = ExtrudeOptions { depth, bevel: 0.0 };
    let [front, back, sides] = extrude_polygon(pts, &opts)
        .expect("primitives: degenerate polygon");
    let color = hex_to_rgb(color_hex);
    let mut b = MeshBuilder::new();
    let g = b.add_group(Material::solid(group_name, color).with_pbr(roughness, metalness).with_class("opaque"));
    b.add_part(g, &front);
    b.add_part(g, &back);
    b.add_part(g, &sides);
    b.build()
}

pub fn generate_square(color_hex: &str) -> Mesh {
    let h = 0.5_f32;  // 1m × 1m, 5cm thick
    let pts = [Point2::new(-h,-h), Point2::new(h,-h), Point2::new(h,h), Point2::new(-h,h)];
    extrude_single(&pts, 0.05, color_hex, "shape_square", 0.05, 0.55)
}

pub fn generate_rectangle(color_hex: &str) -> Mesh {
    let pts = [Point2::new(-0.8,-0.5), Point2::new(0.8,-0.5), Point2::new(0.8,0.5), Point2::new(-0.8,0.5)];
    extrude_single(&pts, 0.05, color_hex, "shape_rectangle", 0.05, 0.55)
}

pub fn generate_triangle(color_hex: &str) -> Mesh {
    let r = 1.0_f32 / 3.0_f32.sqrt();
    let pts: Vec<Point2> = (0..3).map(|i| {
        let a = PI / 2.0 + i as f32 * TAU / 3.0;
        Point2::new(r * a.cos(), r * a.sin())
    }).collect();
    extrude_single(&pts, 0.05, color_hex, "shape_triangle", 0.05, 0.55)
}

pub fn generate_circle(color_hex: &str, quality: GeometryQuality) -> Mesh {
    let segs: usize = match quality { GeometryQuality::Draft => 32, GeometryQuality::Standard => 48, GeometryQuality::High => 64, GeometryQuality::Ultra => 96 };
    let r = 0.6_f32;
    let pts: Vec<Point2> = (0..segs).map(|i| { let a = i as f32 * TAU / segs as f32; Point2::new(r * a.cos(), r * a.sin()) }).collect();
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

    // Each face defined by: corner origin + u_step + v_step + face normal
    // Winding is CCW when viewed from outside.
    let face_defs: [([f32;3], [f32;3], [f32;3], [f32;3]); 6] = [
        // +Z front
        ([-s, -s,  s], [ 1.0, 0.0, 0.0], [0.0,  1.0, 0.0], [0.0, 0.0,  1.0]),
        // -Z back  (flipped u so winding stays CCW)
        ([ s, -s, -s], [-1.0, 0.0, 0.0], [0.0,  1.0, 0.0], [0.0, 0.0, -1.0]),
        // +X right
        ([ s, -s, -s], [0.0, 0.0,  1.0], [0.0,  1.0, 0.0], [ 1.0, 0.0, 0.0]),
        // -X left
        ([-s, -s,  s], [0.0, 0.0, -1.0], [0.0,  1.0, 0.0], [-1.0, 0.0, 0.0]),
        // +Y top
        ([-s,  s,  s], [ 1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0,  1.0, 0.0]),
        // -Y bottom
        ([-s, -s, -s], [ 1.0, 0.0, 0.0], [0.0, 0.0,  1.0], [0.0, -1.0, 0.0]),
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
                let i00 = idx_grid[vi       * (n + 1) + ui    ];
                let i10 = idx_grid[vi       * (n + 1) + ui + 1];
                let i11 = idx_grid[(vi + 1) * (n + 1) + ui + 1];
                let i01 = idx_grid[(vi + 1) * (n + 1) + ui    ];
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
    let len = (pos[0]*pos[0] + pos[1]*pos[1] + pos[2]*pos[2]).sqrt();
    let max_len = s * 1.732_f32; // √3 — distance to a cube corner

    // "Corner factor": 0 at face centre, 1 at pure corner vertex
    // Face-centre vertices have one coordinate = ±s and the other two = 0.
    // Corner vertices have all three coordinates = ±s.
    let corner_factor = ((len - s) / (max_len - s)).clamp(0.0, 1.0);
    let t = bevel * corner_factor;

    if t < 1e-6 { return (pos, face_normal); }

    // Sphere position: same direction, radius = s (inscribed sphere is s,
    // but we blend to circumscribed = s√3 normalised back to s to keep
    // cube size).
    let sphere_pos = if len > 1e-9 {
        [pos[0] / len * s * 1.732_f32 / 1.732_f32,   // = pos/len * s
         pos[1] / len * s,
         pos[2] / len * s]
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
    let nlen = (blended[0]*blended[0] + blended[1]*blended[1] + blended[2]*blended[2]).sqrt().max(1e-9);
    let new_normal = [blended[0]/nlen, blended[1]/nlen, blended[2]/nlen];

    (new_pos, new_normal)
}

pub fn generate_sphere(color_hex: &str, quality: GeometryQuality) -> Mesh {
    use crate::infrastructure::geometry::generators::hard_surface::organic_sphere::{generate_organic_sphere, OrganicSphereSpec};
    generate_organic_sphere(&OrganicSphereSpec { radius: 0.6, color_hex: color_hex.to_string(), ..OrganicSphereSpec::with_quality(quality) })
}

pub fn generate_line(color_hex: &str) -> Mesh {
    let pts = [Point2::new(-1.0,-0.03), Point2::new(1.0,-0.03), Point2::new(1.0,0.03), Point2::new(-1.0,0.03)];
    extrude_single(&pts, 0.03, color_hex, "shape_line", 0.0, 0.6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::kernel::validate::validate_mesh;
    #[test] fn square_valid()    { validate_mesh(&generate_square("#38BDF8")).unwrap(); }
    #[test] fn rectangle_valid() { validate_mesh(&generate_rectangle("#A78BFA")).unwrap(); }
    #[test] fn triangle_valid()  { validate_mesh(&generate_triangle("#FB923C")).unwrap(); }
    #[test] fn circle_valid()    { validate_mesh(&generate_circle("#34D399", GeometryQuality::Draft)).unwrap(); }
    #[test] fn cube_valid()      { validate_mesh(&generate_cube("#F472B6")).unwrap(); }
    #[test] fn sphere_valid()    { validate_mesh(&generate_sphere("#FACC15", GeometryQuality::Draft)).unwrap(); }
    #[test] fn line_valid()      { validate_mesh(&generate_line("#94A3B8")).unwrap(); }

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
            assert!(p.iter().all(|c| c.is_finite()),
                "position has NaN/Inf: {:?}", p);
            // Beveled vertex must stay inside the unit cube (corners pull inward).
            assert!(p.iter().all(|c: &f32| c.abs() <= 0.5 + 1e-4),
                "beveled vertex outside cube bounds: {:?}", p);
        }
        for n in &m.normals {
            assert!(n.iter().all(|c| c.is_finite()),
                "normal has NaN/Inf: {:?}", n);
        }
    }

    #[test]
    fn cube_grid_bevel_corners_pull_inward() {
        // A pure corner vertex (±0.5, ±0.5, ±0.5) on an unbeveled cube has
        // length √(3)/2 ≈ 0.866. With bevel=1 it should be pulled to length s=0.5.
        let sharp = generate_cube_grid("#F472B6", 1, 0.0);
        let smooth = generate_cube_grid("#F472B6", 1, 1.0);

        let max_len = |m: &Mesh| -> f32 {
            m.vertices.iter()
                .map(|p| (p[0].powi(2) + p[1].powi(2) + p[2].powi(2)).sqrt())
                .fold(0.0_f32, f32::max)
        };
        let max_sharp = max_len(&sharp);
        let max_smooth = max_len(&smooth);

        assert!((max_sharp - 0.866).abs() < 0.01, "sharp corner len ≈ √3/2, got {max_sharp}");
        assert!(max_smooth < max_sharp - 0.1,
            "bevel=1 must pull corners inward (got {max_smooth} vs {max_sharp})");
    }
}
