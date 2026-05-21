//! WASM bindings for geometry_engine — extrude + rounded_rect → browser.
//!
//! Build: make wasm-geometry
//!
//! JS:
//!   import init, { extrude_json, rounded_rect_json } from '/wasm/geometry_engine/geometry_engine.js';
//!   await init();
//!   const mesh = JSON.parse(extrude_json(JSON.stringify({ plane:"XZ", depth:0.1, bevel:0, profile:[...] })));

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ops::extrude::{extrude_polygon, ExtrudeOptions, Point2};
use crate::ops::bevel::rounded_rect_points;
use crate::mesh::MeshPart;

fn err_json(msg: impl std::fmt::Display) -> String {
    serde_json::json!({ "ok": false, "error": msg.to_string() }).to_string()
}

#[derive(Deserialize)]
struct JsPoint3 { x: f64, y: f64, z: f64 }

#[derive(Serialize)]
struct GpuMeshOut {
    positions:      Vec<f32>,
    normals:        Vec<f32>,
    face_ids:       Vec<u32>,
    indices:        Vec<u32>,
    vertex_count:   usize,
    triangle_count: usize,
    face_count:     usize,
    kernel:         &'static str,
}

fn pts_to_2d(pts: &[JsPoint3], plane: &str) -> Vec<Point2> {
    pts.iter().map(|p| match plane {
        "XY" => Point2 { x: p.x, y: p.y },
        "YZ" => Point2 { x: p.y, y: p.z },
        _    => Point2 { x: p.x, y: p.z },
    }).collect()
}

fn parts_to_gpu(parts: &[MeshPart; 3], plane: &str) -> GpuMeshOut {
    let mut positions: Vec<f32> = Vec::new();
    let mut normals:   Vec<f32> = Vec::new();
    let mut face_ids:  Vec<u32> = Vec::new();
    let mut indices:   Vec<u32> = Vec::new();
    let mut off: u32            = 0;

    for (fi, part) in parts.iter().enumerate() {
        let fid = (fi + 1) as u32;
        // Kernel uses XZ plane: vertex coords are [x, z_depth, y_sketch] conceptually.
        // Mapping to world-space by plane:
        //   kernel [x, y, z] means: x=profile_x, y=profile_y (or z), z=depth_axis
        // extrude_polygon uses XY profile extruded along +Z:
        //   kernel x = profile.x, kernel y = profile.y, kernel z = depth
        // World remapping:
        //   XZ plane: world(x,y,z) = kernel(x, z, y)   y=depth goes up
        //   XY plane: world(x,y,z) = kernel(x, y, z)   z=depth
        //   YZ plane: world(x,y,z) = kernel(z, x, y)   x=depth
        for (vi, v) in part.vertices.iter().enumerate() {
            let (wx, wy, wz) = match plane {
                "XY" => (v[0] as f32, v[1] as f32, v[2] as f32),
                "YZ" => (v[2] as f32, v[0] as f32, v[1] as f32),
                _    => (v[0] as f32, v[2] as f32, v[1] as f32), // XZ default
            };
            positions.push(wx); positions.push(wy); positions.push(wz);
            if let Some(n) = part.normals.get(vi) {
                let (nx, ny, nz) = match plane {
                    "XY" => (n[0] as f32, n[1] as f32, n[2] as f32),
                    "YZ" => (n[2] as f32, n[0] as f32, n[1] as f32),
                    _    => (n[0] as f32, n[2] as f32, n[1] as f32),
                };
                normals.push(nx); normals.push(ny); normals.push(nz);
            } else {
                normals.push(0.0); normals.push(1.0); normals.push(0.0);
            }
            face_ids.push(fid);
        }
        for tri in &part.faces {
            indices.push(tri[0] as u32 + off);
            indices.push(tri[1] as u32 + off);
            indices.push(tri[2] as u32 + off);
        }
        off += part.vertices.len() as u32;
    }
    let vc = positions.len() / 3;
    let tc = indices.len() / 3;
    GpuMeshOut { positions, normals, face_ids, indices, vertex_count: vc, triangle_count: tc, face_count: 3, kernel: "geometry_engine_wasm" }
}

#[wasm_bindgen]
pub fn extrude_json(input: &str) -> String {
    #[derive(Deserialize)]
    struct Req { plane: Option<String>, depth: f64, bevel: Option<f64>, profile: Vec<JsPoint3> }
    let req: Req = match serde_json::from_str(input) { Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")) };
    if req.profile.len() < 3 { return err_json("profile must have >= 3 points"); }
    if req.depth <= 0.0      { return err_json("depth must be > 0"); }
    let plane = req.plane.as_deref().unwrap_or("XZ");
    let pts2d = pts_to_2d(&req.profile, plane);
    let opts  = ExtrudeOptions { depth: req.depth, bevel: req.bevel.unwrap_or(0.0), ..Default::default() };
    match extrude_polygon(&pts2d, &opts) {
        Ok(parts) => serde_json::to_string(&parts_to_gpu(&parts, plane)).unwrap_or_else(|e| err_json(e)),
        Err(e)    => err_json(e),
    }
}

#[wasm_bindgen]
pub fn rounded_rect_json(input: &str) -> String {
    #[derive(Deserialize)]
    struct Req { plane: Option<String>, depth: f64, width: f64, height: f64, corner_radius: f64, segments: Option<u32> }
    let req: Req = match serde_json::from_str(input) { Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")) };
    if req.corner_radius > req.width.min(req.height) * 0.5 + 1e-9 {
        return err_json(format!("InvalidRadius: {:.6} > half min(w,h)", req.corner_radius));
    }
    if req.depth <= 0.0 { return err_json("depth must be > 0"); }
    let plane = req.plane.as_deref().unwrap_or("XZ");
    let pts2d = rounded_rect_points(req.width, req.height, req.corner_radius, req.segments.unwrap_or(8) as usize);
    let opts  = ExtrudeOptions { depth: req.depth, bevel: 0.0, ..Default::default() };
    match extrude_polygon(&pts2d, &opts) {
        Ok(parts) => serde_json::to_string(&parts_to_gpu(&parts, plane)).unwrap_or_else(|e| err_json(e)),
        Err(e)    => err_json(e),
    }
}

#[wasm_bindgen]
pub fn geometry_engine_info() -> String {
    serde_json::json!({
        "name": "geometry_engine", "version": env!("CARGO_PKG_VERSION"),
        "wasm_ops": ["extrude_json", "rounded_rect_json"],
        "server_ops": ["boolean", "export"],
        "note": "heavy ops route to /api/matter/geometry/*"
    }).to_string()
}
