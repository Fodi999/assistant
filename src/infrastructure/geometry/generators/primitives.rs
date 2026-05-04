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

pub fn generate_cube(color_hex: &str) -> Mesh {
    let h = 0.5_f32;  // 1m × 1m × 1m — like Blender default cube
    let pts = [Point2::new(-h,-h), Point2::new(h,-h), Point2::new(h,h), Point2::new(-h,h)];
    let opts = ExtrudeOptions { depth: h * 2.0, bevel: 0.02 };
    let [front, back, sides] = extrude_polygon(&pts, &opts).expect("cube extrude failed");
    let color = hex_to_rgb(color_hex);
    let mut b = MeshBuilder::new();
    let g = b.add_group(Material::solid("shape_cube", color).with_pbr(0.45, 0.0).with_class("opaque"));
    b.add_part(g, &front); b.add_part(g, &back); b.add_part(g, &sides);
    b.build()
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
}
