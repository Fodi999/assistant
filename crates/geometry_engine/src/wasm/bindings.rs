//! WASM bindings for geometry_engine — full 2D+3D CAD API for the browser.
//!
//! Build: make wasm-geometry
//!
//! JS usage:
//!   import init, { extrude_json, sketch_solve_json, sketch_extrude_json, … }
//!     from '/wasm/geometry_engine/geometry_engine.js';
//!   await init();
//!
//!   // 3D extrude from raw polygon:
//!   const mesh = JSON.parse(extrude_json(JSON.stringify({ plane:"XZ", depth:0.1, profile:[...] })));
//!
//!   // 2D constraint solve:
//!   const solved = JSON.parse(sketch_solve_json(JSON.stringify({ sketch: { points, edges, constraints } })));
//!
//!   // Full sketch → solid (THE main path):
//!   const solid = JSON.parse(sketch_extrude_json(JSON.stringify({
//!     sketch: { points, edges, constraints },
//!     depth_m: 0.1, plane: "XZ"
//!   })));

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ops::extrude::{extrude_polygon, ExtrudeOptions, Point2};
use crate::ops::bevel::rounded_rect_points;
use crate::mesh::MeshPart;
use crate::sketch::types::WorkingPlane;
use crate::sketch::{
    apply_add_point, apply_add_edge, apply_move_point,
    solve_constraints, validate,
    AddPointRequest, AddEdgeRequest, MovePointRequest,
};
use crate::sketch::solver::{apply_constraint_once, SolveConstraintsRequest, SolveConfig};
use crate::sketch::to_solid::{sketch_extrude, SketchExtrudeRequest};

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
    edge_indices:   Vec<u32>,   // line-list: pairs of global vertex indices forming face boundary edges
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
    let mut edge_indices: Vec<u32> = Vec::new();
    let mut off: u32            = 0;

    for (fi, part) in parts.iter().enumerate() {
        let fid = (fi + 1) as u32;
        for (vi, v) in part.vertices.iter().enumerate() {
            let (wx, wy, wz) = match plane {
                "XY" => (v[0] as f32, v[1] as f32, v[2] as f32),
                "YZ" => (v[2] as f32, v[0] as f32, v[1] as f32),
                _    => (v[0] as f32, v[2] as f32, v[1] as f32),
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

        // ── Extract boundary edges of this face (edges used by exactly 1 triangle) ──
        use std::collections::HashMap;
        let mut edge_count: HashMap<(u32, u32), u32> = HashMap::new();
        for tri in &part.faces {
            let a = tri[0] as u32;
            let b = tri[1] as u32;
            let c = tri[2] as u32;
            for (p, q) in [(a,b),(b,c),(c,a)] {
                let key = if p < q { (p, q) } else { (q, p) };
                *edge_count.entry(key).or_insert(0) += 1;
            }
        }
        for ((a, b), count) in edge_count {
            if count == 1 {
                // boundary edge — add as line-list pair with global offset
                edge_indices.push(a + off);
                edge_indices.push(b + off);
            }
        }

        off += part.vertices.len() as u32;
    }
    let vc = positions.len() / 3;
    let tc = indices.len() / 3;
    GpuMeshOut { positions, normals, face_ids, indices, edge_indices, vertex_count: vc, triangle_count: tc, face_count: 3, kernel: "geometry_engine_wasm" }
}

// ── 3D: raw polygon extrude ───────────────────────────────────────────────

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

// ── 2D: sketch commands ───────────────────────────────────────────────────

#[wasm_bindgen]
pub fn wasm_add_point(json: &str) -> String {
    let req: AddPointRequest = match serde_json::from_str(json) { Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")) };
    serde_json::to_string(&apply_add_point(req)).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_add_edge(json: &str) -> String {
    let req: AddEdgeRequest = match serde_json::from_str(json) { Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")) };
    serde_json::to_string(&apply_add_edge(req)).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_move_point(json: &str) -> String {
    let req: MovePointRequest = match serde_json::from_str(json) { Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")) };
    serde_json::to_string(&apply_move_point(req)).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_solve_constraints(json: &str) -> String {
    let req: SolveConstraintsRequest = match serde_json::from_str(json) { Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")) };
    let cfg = req.config.clone().unwrap_or_default();
    let result = if let Some(ref c) = req.constraint {
        apply_constraint_once(req.sketch, c)
    } else {
        crate::sketch::solver::solve_constraints_with_config(req.sketch, &cfg)
    };
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_validate_sketch(json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Req { sketch: crate::sketch::types::SketchGraph }
    let req: Req = match serde_json::from_str(json) { Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")) };
    serde_json::to_string(&validate(&req.sketch)).unwrap_or_else(|e| err_json(e.to_string()))
}

// ── 2D→3D: THE main path ─────────────────────────────────────────────────

/// Full pipeline: sketch (points+edges+constraints) → solve → extrude → GpuMesh.
/// This is the single canonical path that replaces both old WASM + server fallback.
///
/// Input JSON: { sketch: SketchGraph, depth_m: f64, plane?: string, bevel?: f64, profile_id?: string }
/// Output JSON: SketchExtrudeResult { ok, positions, normals, face_ids, indices, vertex_count, … }
#[wasm_bindgen]
pub fn sketch_extrude_json(json: &str) -> String {
    let req: SketchExtrudeRequest = match serde_json::from_str(json) {
        Ok(v)  => v,
        Err(e) => return err_json(format!("bad json: {e}")),
    };
    serde_json::to_string(&sketch_extrude(req)).unwrap_or_else(|e| err_json(e.to_string()))
}

// ── Info ──────────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn geometry_engine_info() -> String {
    serde_json::json!({
        "name": "geometry_engine",
        "version": env!("CARGO_PKG_VERSION"),
        "wasm_ops": [
            "extrude_json",
            "rounded_rect_json",
            "wasm_add_point",
            "wasm_add_edge",
            "wasm_move_point",
            "wasm_solve_constraints",
            "wasm_validate_sketch",
            "sketch_extrude_json",
            "wasm_tool_rect",
            "wasm_tool_circle",
            "wasm_tool_copy",
            "wasm_tool_edge_extrude",
            "wasm_tool_click",
            "wasm_tool_activate",
            "wasm_make_square"
        ],
        "sketch_ops": "2D constraint solver + profile detection built-in",
        "tools": "rect, circle, copy-connect, edge-extrude — all native Rust",
        "note": "Single crate for all 2D+3D CAD operations"
    }).to_string()
}

// ── Tools API ─────────────────────────────────────────────────────────────
//
// wasm_tool_rect(json)         → SketchDelta (4 pts + 4 edges + H/V constraints)
// wasm_tool_circle(json)       → SketchDelta (N-gon points + edges)
// wasm_tool_copy(json)         → SketchDelta (copied pts + edges + connectors)
// wasm_tool_edge_extrude(json) → EdgeExtrudeResult (delta + wall surfaces)
// wasm_tool_click(json)        → ToolClickResult  (FSM: 1st/2nd click dispatch)
// wasm_tool_activate(json)     → void  (set active tool + plane + grid_size)
// wasm_make_square(json)       → SketchDelta (EQUAL_LENGTH + forced geometry)

use crate::tools::{
    rect::{create_rect, RectInput, make_square, MakeSquareInput},
    circle::{create_circle, CircleInput},
    copy::{copy_selection, CopyInput, CopySourcePoint, CopySourceEdge},
    edge_extrude::{edge_extrude, EdgeExtrudeInput, ExtrudeSourcePoint, ExtrudeSourceEdge},
    tool_state::{ToolState, ActiveTool, tool_click},
};
use std::cell::RefCell;

thread_local! {
    static TOOL_STATE: RefCell<ToolState> = RefCell::new(ToolState::new());
}

/// Create a rectangle: 4 points + 4 edges + HORIZONTAL/VERTICAL constraints.
/// Input: { gx1, gy1, gz1, gx2, gy2, gz2, plane, id_offset? }
#[wasm_bindgen]
pub fn wasm_tool_rect(json: &str) -> String {
    #[derive(Deserialize)]
    struct Req {
        gx1: i64, gy1: i64, gz1: i64,
        gx2: i64, gy2: i64, gz2: i64,
        plane: Option<String>,
        id_offset: Option<u64>,
    }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    let delta = create_rect(RectInput {
        gx1: req.gx1, gy1: req.gy1, gz1: req.gz1,
        gx2: req.gx2, gy2: req.gy2, gz2: req.gz2,
        plane: req.plane.unwrap_or_else(|| "XZ".into()),
        id_offset: req.id_offset.unwrap_or(1),
    });
    serde_json::to_string(&delta).unwrap_or_else(|e| err_json(e.to_string()))
}

/// Create a circle N-gon: points + closed edges.
/// Input: { center_gx, center_gy, center_gz, radius, plane, segments?, id_offset? }
#[wasm_bindgen]
pub fn wasm_tool_circle(json: &str) -> String {
    #[derive(Deserialize)]
    struct Req {
        center_gx: i64, center_gy: i64, center_gz: i64,
        radius: f64,
        plane: Option<String>,
        segments: Option<usize>,
        id_offset: Option<u64>,
    }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    let delta = create_circle(CircleInput {
        center_gx: req.center_gx, center_gy: req.center_gy, center_gz: req.center_gz,
        radius: req.radius,
        plane: req.plane.unwrap_or_else(|| "XZ".into()),
        segments: req.segments.unwrap_or(32),
        id_offset: req.id_offset.unwrap_or(1),
    });
    serde_json::to_string(&delta).unwrap_or_else(|e| err_json(e.to_string()))
}

/// Copy-connect: duplicate points+edges with a world-space offset, add connectors.
/// Input: { points: [{id,x,y,z}], edges: [{a,b,kind}], dx, dy, dz, grid_size?, id_offset? }
#[wasm_bindgen]
pub fn wasm_tool_copy(json: &str) -> String {
    #[derive(Deserialize)]
    struct Req {
        points: Vec<CopySourcePoint>,
        edges: Vec<CopySourceEdge>,
        dx: f64, dy: f64, dz: f64,
        grid_size: Option<f64>,
        id_offset: Option<u64>,
    }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    let delta = copy_selection(CopyInput {
        points: req.points, edges: req.edges,
        dx: req.dx, dy: req.dy, dz: req.dz,
        grid_size: req.grid_size.unwrap_or(0.01),
        id_offset: req.id_offset.unwrap_or(1),
    });
    serde_json::to_string(&delta).unwrap_or_else(|e| err_json(e.to_string()))
}

/// Edge extrude: create top points + top edge + vertical edges + wall surfaces.
/// Input: { edges: [{id,a,b}], points: [{id,x,y,z,gx,gy,gz}], height_m, plane, grid_size?, id_offset? }
#[wasm_bindgen]
pub fn wasm_tool_edge_extrude(json: &str) -> String {
    #[derive(Deserialize)]
    struct Req {
        edges: Vec<ExtrudeSourceEdge>,
        points: Vec<ExtrudeSourcePoint>,
        height_m: f64,
        plane: Option<String>,
        grid_size: Option<f64>,
        id_offset: Option<u64>,
    }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    let result = edge_extrude(EdgeExtrudeInput {
        edges: req.edges, points: req.points,
        height_m: req.height_m,
        plane: req.plane.unwrap_or_else(|| "XZ".into()),
        grid_size: req.grid_size.unwrap_or(0.01),
        id_offset: req.id_offset.unwrap_or(1),
    });
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

/// Activate a tool in the WASM FSM.
/// Input: { tool: "rect"|"circle"|"grab"|"edge_extrude"|"none", plane?, grid_size?, segments? }
#[wasm_bindgen]
pub fn wasm_tool_activate(json: &str) -> String {
    #[derive(Deserialize)]
    struct Req {
        tool: String,
        plane: Option<String>,
        grid_size: Option<f64>,
        segments: Option<usize>,
    }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    let tool = match req.tool.as_str() {
        "rect"          => ActiveTool::Rect,
        "circle"        => ActiveTool::Circle { segments: req.segments.unwrap_or(32) },
        "grab"          => ActiveTool::Grab,
        "edge_extrude"  => ActiveTool::EdgeExtrude,
        "solid_extrude" => ActiveTool::SolidExtrude,
        "copy"          => ActiveTool::CopyConnect,
        _               => ActiveTool::None,
    };
    let plane = req.plane.unwrap_or_else(|| "XZ".into());
    let gs    = req.grid_size.unwrap_or(0.01);
    TOOL_STATE.with(|s| s.borrow_mut().activate(tool, plane, gs));
    serde_json::json!({ "ok": true }).to_string()
}

/// Dispatch a click at (gx, gy, gz) to the active tool FSM.
/// Input: { gx, gy, gz }
/// Output: ToolClickResult { ok, phase, status, delta?, still_active }
#[wasm_bindgen]
pub fn wasm_tool_click(json: &str) -> String {
    #[derive(Deserialize)]
    struct Req { gx: i64, gy: i64, gz: i64 }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    let result = TOOL_STATE.with(|s| {
        let mut state = s.borrow_mut();
        tool_click(&mut state, req.gx, req.gy, req.gz)
    });
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

/// Make square: force all 4 sides equal length via EQUAL_LENGTH + forced geometry.
/// Input: { pt_ids: [4 strings], edge_ids: [4 strings],
///          pts_gx: [4], pts_gy: [4], pts_gz: [4], plane, id_offset? }
#[wasm_bindgen]
pub fn wasm_make_square(json: &str) -> String {
    #[derive(Deserialize)]
    struct Req {
        pt_ids: [String; 4],
        edge_ids: [String; 4],
        pts_gx: [i64; 4],
        pts_gy: [i64; 4],
        pts_gz: [i64; 4],
        plane: Option<String>,
        id_offset: Option<u64>,
    }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    let delta = make_square(MakeSquareInput {
        pt_ids: req.pt_ids, edge_ids: req.edge_ids,
        pts_gx: req.pts_gx, pts_gy: req.pts_gy, pts_gz: req.pts_gz,
        plane: req.plane.unwrap_or_else(|| "XZ".into()),
        id_offset: req.id_offset.unwrap_or(1),
    });
    serde_json::to_string(&delta).unwrap_or_else(|e| err_json(e.to_string()))
}

