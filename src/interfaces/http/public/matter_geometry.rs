//! Geometry-engine operations exposed as HTTP endpoints.
//!
//! # Endpoints
//!
//! ## POST /api/matter/geometry/boolean
//! Run a CSG boolean operation on two             let fid = tri_meta.map(|m| m.face_id.id.0 as u32).unwrap_or(1);xtruded solids.
//!
//! ```json
//! {
//!   "op": "subtract",            // "union" | "subtract" | "intersect"
//!   "a": {
//!     "plane": "XZ", "depth": 0.1,
//!     "profile": [{"x":0,"y":0,"z":0}, ...]
//!   },
//!   "b": {
//!     "plane": "XZ", "depth": 0.05,
//!     "profile": [{"x":0.02,"y":0,"z":0.02}, ...]
//!   }
//! }
//! ```
//! Response: same GPU-ready arrays as `/api/matter/sketch/extrude`.
//!
//! ## POST /api/matter/geometry/tessellate
//! Re-tessellate a B-Rep model that was previously produced by a boolean op
//! (pass back `brep_json` from a prior boolean response, or just re-extrude
//! with different options — this endpoint is intentionally a superset).

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

use geometry_engine::{
    extrude_polygon_brep, ExtrudeBrepResult, ExtrudeOptions, Point2,
    boolean::{boolean_subtract, boolean_intersect},
    tessellation::{tessellate_body, TessOptions},
};

// ── Shared types ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct ProfilePoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Specification for one extruded solid (same schema as sketch/extrude).
#[derive(Deserialize, Clone)]
pub struct SolidSpec {
    pub plane: String,
    pub depth: f64,
    pub profile: Vec<ProfilePoint>,
    #[serde(default)]
    pub bevel: Option<f64>,
}

#[derive(Serialize)]
pub struct GeometryMeshResponse {
    pub ok: bool,
    pub op: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub face_count: usize,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub face_ids: Vec<u32>,
    pub kernel: &'static str,
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

type AppResult<T> = std::result::Result<T, (StatusCode, Json<ApiError>)>;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn bad(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (StatusCode::BAD_REQUEST, Json(ApiError { error: msg.into() }))
}

fn unprocessable(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (StatusCode::UNPROCESSABLE_ENTITY, Json(ApiError { error: msg.into() }))
}

fn signed_area_2d(pts: &[(f64, f64)]) -> f64 {
    let n = pts.len();
    let mut s = 0.0_f64;
    for i in 0..n {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[(i + 1) % n];
        s += ax * by - bx * ay;
    }
    s * 0.5
}

/// Build a `BrepModel` from a `SolidSpec`.
fn spec_to_brep(spec: &SolidSpec, label: &str) -> AppResult<ExtrudeBrepResult> {
    if spec.profile.len() < 3 {
        return Err(bad(format!("{label}: profile needs >=3 points, got {}", spec.profile.len())));
    }
    if !(spec.depth.is_finite() && spec.depth > 0.0) {
        return Err(bad(format!("{label}: depth must be positive finite, got {}", spec.depth)));
    }

    let plane = spec.plane.to_uppercase();

    let mut pts2: Vec<(f64, f64)> = spec.profile.iter().map(|p| match plane.as_str() {
        "XY" => (p.x, p.y),
        "YZ" => (p.y, p.z),
        _    => (p.x, p.z), // XZ default
    }).collect();

    if signed_area_2d(&pts2) < 0.0 { pts2.reverse(); }

    // Remove duplicate closing vertex
    if pts2.len() > 3 {
        let first = pts2[0];
        let last  = *pts2.last().unwrap();
        if (first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9 {
            pts2.pop();
        }
    }

    if pts2.len() < 3 {
        return Err(bad(format!("{label}: profile collapsed to <3 unique points")));
    }

    let kernel_pts: Vec<Point2> = pts2.iter().map(|(u, v)| Point2::new(*u, *v)).collect();
    let bevel = spec.bevel.unwrap_or(0.0);

    extrude_polygon_brep(&kernel_pts, &ExtrudeOptions { depth: spec.depth, bevel })
        .map_err(|e| unprocessable(format!("{label}: extrude_polygon_brep: {e:?}")))
}

// ── Boolean endpoint ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BooleanRequest {
    pub op: String,
    pub a: SolidSpec,
    pub b: SolidSpec,
}

pub async fn boolean_endpoint(
    Json(req): Json<BooleanRequest>,
) -> AppResult<Json<GeometryMeshResponse>> {
    let op = req.op.to_lowercase();
    tracing::info!(
        "🔧 geometry/boolean: op={op}, plane_a={}, depth_a={:.4}m, pts_a={}, plane_b={}, depth_b={:.4}m, pts_b={}",
        req.a.plane, req.a.depth, req.a.profile.len(),
        req.b.plane, req.b.depth, req.b.profile.len(),
    );

    // ── 1. Build both BrepModels ─────────────────────────────────────────
    let brep_a = spec_to_brep(&req.a, "solid_a")?;
    let brep_b = spec_to_brep(&req.b, "solid_b")?;

    // ── 2. Run boolean op ────────────────────────────────────────────────
    let result_model = match op.as_str() {
        "subtract"  => boolean_subtract(&brep_a.model, &brep_b.model),
        "intersect" => boolean_intersect(&brep_a.model, &brep_b.model),
        "union"     => {
            // union not yet fully implemented — fall back to solid A for now
            tracing::warn!("geometry/boolean: union not yet fully implemented, returning solid A");
            // Re-extrude solid A as stand-in result
            let fallback = spec_to_brep(&req.a, "union_fallback")?;
            fallback.model
        }
        other => return Err(bad(format!("unknown op '{other}'; expected union|subtract|intersect"))),
    };

    // ── 3. Tessellate result ─────────────────────────────────────────────
    let body_id = result_model
        .store
        .bodies
        .keys()
        .next()
        .copied()
        .ok_or_else(|| unprocessable("boolean produced an empty model (no bodies)"))?;

    let opts     = TessOptions::default();
    let mesh_meta = tessellate_body(&result_model, body_id, &opts);
    let mesh      = &mesh_meta.mesh;

    // ── 4. Flatten to GPU arrays ─────────────────────────────────────────
    let plane = req.a.plane.to_uppercase();
    let plane_s = plane.as_str();

    let mut positions: Vec<f32> = Vec::with_capacity(mesh.vertices.len() * 3);
    let mut normals:   Vec<f32> = Vec::with_capacity(mesh.normals.len() * 3);
    let mut indices:   Vec<u32> = Vec::with_capacity(mesh.faces.len() * 3);
    let mut face_ids:  Vec<u32> = Vec::with_capacity(mesh.vertices.len());

    for v in &mesh.vertices {
        let (wx, wy, wz): (f64, f64, f64) = match plane_s {
            "XY" => (v[0], v[1], v[2]),
            "YZ" => (v[2], v[0], v[1]),
            _    => (v[0], v[2], v[1]),
        };
        positions.push(wx as f32);
        positions.push(wy as f32);
        positions.push(wz as f32);
        face_ids.push(1); // per-vertex: simplified (all face 1 for now)
    }

    // Per-vertex face_id from triangle metadata (each triangle → 3 vertices)
    // Rebuild correctly: one face_id per vertex = face_id of the triangle it belongs to
    {
        let vc = positions.len() / 3;
        face_ids.clear();
        face_ids.resize(vc, 1u32);
        for (tri_idx, tri) in mesh.faces.iter().enumerate() {
            let fid = mesh_meta.triangles
                .get(tri_idx)
                .map(|m| m.face_id.id.0 as u32)
                .unwrap_or(1);
            face_ids[tri[0]] = fid;
            face_ids[tri[1]] = fid;
            face_ids[tri[2]] = fid;
        }
    }

    for n in &mesh.normals {
        let (nx, ny, nz): (f64, f64, f64) = match plane_s {
            "XY" => (n[0], n[1], n[2]),
            "YZ" => (n[2], n[0], n[1]),
            _    => (n[0], n[2], n[1]),
        };
        normals.push(nx as f32);
        normals.push(ny as f32);
        normals.push(nz as f32);
    }

    for tri in &mesh.faces {
        indices.push(tri[0] as u32);
        indices.push(tri[1] as u32);
        indices.push(tri[2] as u32);
    }

    let vc = positions.len() / 3;
    let tc = indices.len() / 3;

    // Count unique face IDs
    let mut unique_faces: Vec<u32> = face_ids.clone();
    unique_faces.sort_unstable();
    unique_faces.dedup();
    let fc = unique_faces.len();

    tracing::info!(
        "✅ geometry/boolean[{op}]: {vc} verts, {tc} tris, {fc} faces"
    );

    Ok(Json(GeometryMeshResponse {
        ok: true,
        op,
        vertex_count:   vc,
        triangle_count: tc,
        face_count:     fc,
        positions,
        normals,
        indices,
        face_ids,
        kernel: "geometry-engine-boolean",
    }))
}
