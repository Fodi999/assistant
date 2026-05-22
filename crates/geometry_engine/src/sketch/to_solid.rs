//! SketchGraph → 3D Solid — THE key bridge between 2D and 3D.
//!
//! This is the function that serious CAD tools center around:
//!
//!   sketch (points + edges + constraints)
//!     → solve constraints → detect closed profiles
//!       → select profile → extrude to solid mesh
//!         → tessellate → GpuMesh ready for WebGPU
//!
//! # Usage
//! ```ignore
//! let req = SketchExtrudeRequest {
//!     sketch: my_sketch,
//!     depth_m: 0.1,
//!     plane: "XZ".into(),
//!     bevel: None,
//!     profile_id: None,  // None = pick first closed profile
//! };
//! let result = sketch_extrude(req)?;
//! // result.positions / normals / face_ids / indices → GPU upload
//! ```

use serde::{Deserialize, Serialize};

use crate::sketch::profiles::detect_profiles;
use crate::sketch::solver::{solve_constraints, SolveConfig};
use crate::sketch::types::{SketchGraph, WorkingPlane};
use crate::ops::extrude::{extrude_polygon, ExtrudeOptions, Point2};
use crate::mesh::GeometryError;

// ── Request / Response ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SketchExtrudeRequest {
    pub sketch:    SketchGraph,
    /// Extrusion depth in metres.
    pub depth_m:   f64,
    /// Working plane ("XZ" | "XY" | "YZ").  Defaults to sketch.working_plane.
    #[serde(default)]
    pub plane:     Option<String>,
    /// Optional chamfer width in metres.
    #[serde(default)]
    pub bevel:     Option<f64>,
    /// ID of the profile to extrude.  None = first closed profile.
    #[serde(default)]
    pub profile_id: Option<String>,
    /// Run constraint solver before extrusion (default: true).
    #[serde(default = "default_true")]
    pub solve:     bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize)]
pub struct SketchExtrudeResult {
    pub ok:             bool,
    /// Flat f32 array: [x0,y0,z0, x1,y1,z1, …]
    pub positions:      Vec<f32>,
    pub normals:        Vec<f32>,
    /// Per-vertex face id (1-based, same across a face group).
    pub face_ids:       Vec<u32>,
    pub indices:        Vec<u32>,
    pub vertex_count:   usize,
    pub triangle_count: usize,
    pub face_count:     usize,
    pub profile_id:     String,
    pub kernel:         &'static str,
    pub error:          Option<String>,
    /// Post-solve sketch (points moved by solver, profiles detected).
    pub sketch:         SketchGraph,
}

impl SketchExtrudeResult {
    fn err(sketch: SketchGraph, msg: impl Into<String>) -> Self {
        Self {
            ok: false, positions: vec![], normals: vec![], face_ids: vec![],
            indices: vec![], vertex_count: 0, triangle_count: 0, face_count: 0,
            profile_id: String::new(), kernel: "geometry_engine",
            error: Some(msg.into()), sketch,
        }
    }
}

// ── Main entry point ──────────────────────────────────────────────────────

pub fn sketch_extrude(req: SketchExtrudeRequest) -> SketchExtrudeResult {
    let depth = req.depth_m;
    if depth <= 0.0 {
        return SketchExtrudeResult::err(req.sketch, format!("depth must be > 0, got {depth}"));
    }

    // 1. Optionally run the constraint solver.
    let mut sketch = if req.solve {
        let cfg = SolveConfig { compute_diagnostics: false, ..Default::default() };
        solve_constraints_with_config_inner(req.sketch, &cfg)
    } else {
        let mut s = req.sketch;
        s.profiles = detect_profiles(&s);
        s
    };

    // 2. Pick the target profile.
    let plane_str = req.plane
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&sketch.working_plane)
        .to_string();

    let profile = match req.profile_id.as_deref() {
        Some(pid) => sketch.profiles.iter().find(|p| p.id == pid).cloned(),
        None      => sketch.profiles.iter().find(|p| p.closed).cloned(),
    };
    let profile = match profile {
        Some(p) => p,
        None    => return SketchExtrudeResult::err(sketch, "No closed profile found in sketch"),
    };
    let profile_id = profile.id.clone();

    // 3. Extract 2D points from profile → working plane coords.
    let wp = match WorkingPlane::parse(&plane_str) {
        Some(p) => p,
        None    => return SketchExtrudeResult::err(sketch, format!("Unknown plane: {plane_str}")),
    };

    let mut pts2d: Vec<Point2> = Vec::with_capacity(profile.point_ids.len());
    for pid in &profile.point_ids {
        let p = match sketch.find_point(pid) {
            Some(p) => p,
            None    => return SketchExtrudeResult::err(sketch, format!("Profile point {pid} not found")),
        };
        let (u, v) = match wp {
            WorkingPlane::XZ => (p.x, p.z),
            WorkingPlane::XY => (p.x, p.y),
            WorkingPlane::YZ => (p.y, p.z),
        };
        pts2d.push(Point2 { x: u, y: v });
    }
    if pts2d.len() < 3 {
        return SketchExtrudeResult::err(sketch, "Profile has fewer than 3 points");
    }

    // 4. Extrude.
    let opts = ExtrudeOptions { depth, bevel: req.bevel.unwrap_or(0.0), ..Default::default() };
    let parts = match extrude_polygon(&pts2d, &opts) {
        Ok(p)  => p,
        Err(e) => return SketchExtrudeResult::err(sketch, format!("Extrude failed: {e}")),
    };

    // 5. Flatten mesh parts → GPU arrays (plane remapping matches bindings.rs).
    let mut positions: Vec<f32> = Vec::new();
    let mut normals:   Vec<f32> = Vec::new();
    let mut face_ids:  Vec<u32> = Vec::new();
    let mut indices:   Vec<u32> = Vec::new();
    let mut offset: u32 = 0;

    for (fi, part) in parts.iter().enumerate() {
        let fid = (fi + 1) as u32;
        for (vi, v) in part.vertices.iter().enumerate() {
            let (wx, wy, wz) = remap_vertex(v[0], v[1], v[2], wp);
            positions.push(wx); positions.push(wy); positions.push(wz);
            if let Some(n) = part.normals.get(vi) {
                let (nx, ny, nz) = remap_vertex(n[0], n[1], n[2], wp);
                normals.push(nx); normals.push(ny); normals.push(nz);
            } else {
                normals.push(0.0); normals.push(1.0); normals.push(0.0);
            }
            face_ids.push(fid);
        }
        for tri in &part.faces {
            indices.push(tri[0] as u32 + offset);
            indices.push(tri[1] as u32 + offset);
            indices.push(tri[2] as u32 + offset);
        }
        offset += part.vertices.len() as u32;
    }

    let vc = positions.len() / 3;
    let tc = indices.len() / 3;

    SketchExtrudeResult {
        ok: true, positions, normals, face_ids, indices,
        vertex_count: vc, triangle_count: tc, face_count: 3,
        profile_id, kernel: "geometry_engine", error: None, sketch,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn remap_vertex(x: f64, y: f64, z: f64, plane: WorkingPlane) -> (f32, f32, f32) {
    match plane {
        WorkingPlane::XY => (x as f32, y as f32, z as f32),
        WorkingPlane::YZ => (z as f32, x as f32, y as f32),
        WorkingPlane::XZ => (x as f32, z as f32, y as f32), // XZ default
    }
}

/// Internal: run solver + detect profiles, return updated sketch.
fn solve_constraints_with_config_inner(sketch: SketchGraph, cfg: &SolveConfig) -> SketchGraph {
    use crate::sketch::solver::solve_constraints_with_config;
    solve_constraints_with_config(sketch, cfg).sketch
}
