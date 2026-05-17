// ── Profile Analyze + Repair (backend CAD engine) ────────────────────────
//
// Endpoints:
//   POST /api/matter/sketch/profile/analyze
//   POST /api/matter/sketch/profile/repair
//
// All coordinates in mm on input/output for frontend compatibility.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::sketch::{Point, SketchGraph};
use crate::profiles::Profile;

// ─── Request/Response types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ProfileAnalyzeRequest {
    pub sketch: SketchGraph,
    pub profile_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ProfileRepairRequest {
    pub sketch: SketchGraph,
    pub profile_id: String,
    pub repair_type: String, // "FIX_RECTANGLE" | "FIX_SQUARE" | "EQUALIZE_EDGES"
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileIssue {
    pub kind: String,           // "not_axis_aligned" | "length_mismatch" | "angle_not_90" | "not_closed"
    pub severity: String,       // "error" | "warn"
    pub edge_id: Option<String>,
    pub vertex_point_id: Option<String>,
    pub drift_mm: Option<f64>,
    pub actual_mm: Option<f64>,
    pub expected_mm: Option<f64>,
    pub angle_deg: Option<f64>,
    pub orient: Option<String>, // "H" | "V"
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ProfileAnalyzeResponse {
    pub ok: bool,
    pub profile_id: String,
    pub profile_type: String,  // "rectangle" | "square" | "open" | "polygon"
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub area_mm2: Option<f64>,
    pub perimeter_mm: Option<f64>,
    pub issues: Vec<ProfileIssue>,
    pub error_count: usize,
    pub warn_count: usize,
}

#[derive(Debug, Serialize)]
pub struct RepairedPoint {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_mm: f64,
}

#[derive(Debug, Serialize)]
pub struct ProfileRepairResponse {
    pub ok: bool,
    pub profile_id: String,
    pub repair_type: String,
    pub repaired_points: Vec<RepairedPoint>,
    pub avg_mm: Option<f64>,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub error: Option<String>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn point_map(sketch: &SketchGraph) -> HashMap<String, &Point> {
    sketch.points.iter().map(|p| (p.id.clone(), p)).collect()
}

/// Real-world length of an edge in mm.
fn edge_length_mm(a: &Point, b: &Point) -> f64 {
    let dx = (b.x - a.x) * 1000.0;
    let dy = (b.y - a.y) * 1000.0;
    let dz = (b.z - a.z) * 1000.0;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Find a profile by id in the sketch (re-detect).
fn find_profile(sketch: &SketchGraph, profile_id: &str) -> Option<Profile> {
    crate::profiles::detect_profiles(sketch)
        .into_iter()
        .find(|p| p.id == profile_id)
        // Fallback: if id doesn't match deterministically, take first profile
        .or_else(|| crate::profiles::detect_profiles(sketch).into_iter().next())
}

// ─── Analyze ─────────────────────────────────────────────────────────────

pub fn analyze_profile(req: ProfileAnalyzeRequest) -> ProfileAnalyzeResponse {
    let profile = match find_profile(&req.sketch, &req.profile_id) {
        Some(p) => p,
        None => {
            return ProfileAnalyzeResponse {
                ok: false,
                profile_id: req.profile_id.clone(),
                profile_type: "unknown".into(),
                width_mm: None,
                height_mm: None,
                area_mm2: None,
                perimeter_mm: None,
                issues: vec![ProfileIssue {
                    kind: "not_found".into(),
                    severity: "error".into(),
                    edge_id: None,
                    vertex_point_id: None,
                    drift_mm: None,
                    actual_mm: None,
                    expected_mm: None,
                    angle_deg: None,
                    orient: None,
                    message: format!("Profile '{}' not found in sketch", req.profile_id),
                }],
                error_count: 1,
                warn_count: 0,
            };
        }
    };

    let pm = point_map(&req.sketch);
    let mut issues: Vec<ProfileIssue> = Vec::new();
    let AXIS_TOL_MM: f64 = 0.01; // 0.01 mm tolerance for axis-alignment

    // Collect edge lengths and check axis alignment
    let mut lengths_mm: Vec<f64> = Vec::new();
    let mut min_x = f64::MAX; let mut max_x = f64::MIN;
    let mut min_y = f64::MAX; let mut max_y = f64::MIN;
    let mut min_z = f64::MAX; let mut max_z = f64::MIN;

    for edge_id in &profile.edge_ids {
        let edge = match req.sketch.edges.iter().find(|e| &e.id == edge_id) {
            Some(e) => e,
            None => continue,
        };
        let pa = match pm.get(&edge.a) { Some(p) => *p, None => continue };
        let pb = match pm.get(&edge.b) { Some(p) => *p, None => continue };

        let len = edge_length_mm(pa, pb);
        lengths_mm.push(len);

        // Update bounding box
        for pt in [pa, pb] {
            let x = pt.x * 1000.0;
            let y = pt.y * 1000.0;
            let z = pt.z * 1000.0;
            if x < min_x { min_x = x; } if x > max_x { max_x = x; }
            if y < min_y { min_y = y; } if y > max_y { max_y = y; }
            if z < min_z { min_z = z; } if z > max_z { max_z = z; }
        }

        // Check axis alignment (XZ plane: check if edge is H or V)
        let dx_mm = (pb.x - pa.x).abs() * 1000.0;
        let dz_mm = (pb.z - pa.z).abs() * 1000.0;
        let dy_mm = (pb.y - pa.y).abs() * 1000.0;

        let is_h = dz_mm < AXIS_TOL_MM && dy_mm < AXIS_TOL_MM; // purely X axis
        let is_v = dx_mm < AXIS_TOL_MM && dy_mm < AXIS_TOL_MM; // purely Z axis
        let is_y = dx_mm < AXIS_TOL_MM && dz_mm < AXIS_TOL_MM; // purely Y axis

        if !is_h && !is_v && !is_y {
            // Diagonal or skewed edge
            let drift_mm = dz_mm.min(dx_mm).min(dy_mm); // smallest non-zero component
            let orient = if dx_mm >= dz_mm.max(dy_mm) { "H" }
                        else if dz_mm >= dx_mm.max(dy_mm) { "V" }
                        else { "Y" };
            issues.push(ProfileIssue {
                kind: "not_axis_aligned".into(),
                severity: "error".into(),
                edge_id: Some(edge_id.clone()),
                vertex_point_id: None,
                drift_mm: Some(drift_mm),
                actual_mm: None,
                expected_mm: None,
                angle_deg: None,
                orient: Some(orient.into()),
                message: format!("Edge {} is not axis-aligned (drift {:.2} mm)", edge_id, drift_mm),
            });
        }
    }

    // Bounding box dimensions (XZ plane)
    let width_mm  = if max_x > min_x { Some(max_x - min_x) } else { None };
    let height_mm = if max_z > min_z { Some(max_z - min_z) } else { None };

    // Check if it looks like a rectangle (4 edges, alternating H/V)
    let n = profile.edge_ids.len();
    let is_rect_candidate = n == 4;
    if is_rect_candidate && lengths_mm.len() == 4 {
        // Opposite edges should have equal length
        let pairs = [(0usize, 2usize), (1, 3)];
        for (i, j) in pairs {
            let diff = (lengths_mm[i] - lengths_mm[j]).abs();
            if diff > AXIS_TOL_MM {
                issues.push(ProfileIssue {
                    kind: "length_mismatch".into(),
                    severity: "warn".into(),
                    edge_id: Some(profile.edge_ids[i].clone()),
                    vertex_point_id: None,
                    drift_mm: None,
                    actual_mm: Some(lengths_mm[i]),
                    expected_mm: Some(lengths_mm[j]),
                    angle_deg: None,
                    orient: None,
                    message: format!(
                        "Opposite edges differ: {:.2} vs {:.2} mm (diff {:.2})",
                        lengths_mm[i], lengths_mm[j], diff
                    ),
                });
            }
        }
    }

    // Perimeter
    let perimeter_mm: f64 = lengths_mm.iter().sum();

    // Area (shoelace on XZ plane using point_ids)
    let area_mm2 = shoelace_area_mm2(&profile.point_ids, &pm);

    // Profile type
    let profile_type = classify_profile(n, &issues, width_mm, height_mm);

    let error_count = issues.iter().filter(|i| i.severity == "error").count();
    let warn_count  = issues.iter().filter(|i| i.severity == "warn").count();

    ProfileAnalyzeResponse {
        ok: true,
        profile_id: profile.id,
        profile_type,
        width_mm,
        height_mm,
        area_mm2,
        perimeter_mm: Some(perimeter_mm),
        issues,
        error_count,
        warn_count,
    }
}

fn classify_profile(n: usize, issues: &[ProfileIssue], w: Option<f64>, h: Option<f64>) -> String {
    if n != 4 { return "polygon".into(); }
    let has_errors = issues.iter().any(|i| i.severity == "error");
    if has_errors { return "rectangle_skewed".into(); }
    match (w, h) {
        (Some(ww), Some(hh)) if (ww - hh).abs() < 0.01 => "square".into(),
        _ => "rectangle".into(),
    }
}

fn shoelace_area_mm2(point_ids: &[String], pm: &HashMap<String, &Point>) -> Option<f64> {
    if point_ids.len() < 3 { return None; }
    let pts: Vec<(f64, f64)> = point_ids.iter().filter_map(|id| {
        pm.get(id).map(|p| (p.x * 1000.0, p.z * 1000.0))
    }).collect();
    if pts.len() < 3 { return None; }
    let mut area = 0.0f64;
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += pts[i].0 * pts[j].1;
        area -= pts[j].0 * pts[i].1;
    }
    Some(area.abs() / 2.0)
}

// ─── Repair ──────────────────────────────────────────────────────────────

pub fn repair_profile(req: ProfileRepairRequest) -> ProfileRepairResponse {
    let profile = match find_profile(&req.sketch, &req.profile_id) {
        Some(p) => p,
        None => return ProfileRepairResponse {
            ok: false,
            profile_id: req.profile_id,
            repair_type: req.repair_type,
            repaired_points: vec![],
            avg_mm: None,
            width_mm: None,
            height_mm: None,
            error: Some("Profile not found".into()),
        },
    };

    match req.repair_type.as_str() {
        "FIX_RECTANGLE" => repair_rectangle(profile, &req.sketch, false),
        "FIX_SQUARE"    => repair_rectangle(profile, &req.sketch, true),
        "EQUALIZE_EDGES" => equalize_edges(profile, &req.sketch),
        other => ProfileRepairResponse {
            ok: false,
            profile_id: profile.id,
            repair_type: other.to_string(),
            repaired_points: vec![],
            avg_mm: None,
            width_mm: None,
            height_mm: None,
            error: Some(format!("Unknown repair_type: {}", other)),
        },
    }
}

/// Snap all profile points to a clean axis-aligned rectangle.
/// If `make_square = true`, use the longer side for both dimensions.
fn repair_rectangle(profile: Profile, sketch: &SketchGraph, make_square: bool) -> ProfileRepairResponse {
    let pm = point_map(sketch);

    // Collect profile points
    let pts: Vec<&Point> = profile.point_ids.iter()
        .filter_map(|id| pm.get(id).copied())
        .collect();

    if pts.len() < 3 {
        return ProfileRepairResponse {
            ok: false,
            profile_id: profile.id,
            repair_type: if make_square { "FIX_SQUARE" } else { "FIX_RECTANGLE" }.into(),
            repaired_points: vec![],
            avg_mm: None, width_mm: None, height_mm: None,
            error: Some("Need ≥3 points".into()),
        };
    }

    // Bounding box
    let mut min_x = f64::MAX; let mut max_x = f64::MIN;
    let mut min_z = f64::MAX; let mut max_z = f64::MIN;
    for p in &pts {
        let x = p.x * 1000.0; let z = p.z * 1000.0;
        if x < min_x { min_x = x; } if x > max_x { max_x = x; }
        if z < min_z { min_z = z; } if z > max_z { max_z = z; }
    }

    let mut w = max_x - min_x;
    let mut h = max_z - min_z;

    if make_square {
        let side = w.max(h);
        w = side; h = side;
        max_x = min_x + w;
        max_z = min_z + h;
    }

    // Snap each point to nearest corner of the clean rectangle
    let corners = [
        (min_x, min_z),
        (max_x, min_z),
        (max_x, max_z),
        (min_x, max_z),
    ];

    let mut repaired: Vec<RepairedPoint> = Vec::new();
    for p in &pts {
        let px = p.x * 1000.0; let pz = p.z * 1000.0;
        let (cx, cz) = *corners.iter().min_by(|(ax, az), (bx, bz)| {
            let da = (px - ax).powi(2) + (pz - az).powi(2);
            let db = (px - bx).powi(2) + (pz - bz).powi(2);
            da.partial_cmp(&db).unwrap()
        }).unwrap();

        repaired.push(RepairedPoint {
            id: p.id.clone(),
            x: cx / 1000.0,
            y: p.y,
            z: cz / 1000.0,
            x_mm: cx,
            y_mm: p.y * 1000.0,
            z_mm: cz,
        });
    }

    ProfileRepairResponse {
        ok: true,
        profile_id: profile.id,
        repair_type: if make_square { "FIX_SQUARE" } else { "FIX_RECTANGLE" }.into(),
        repaired_points: repaired,
        avg_mm: None,
        width_mm: Some(w),
        height_mm: Some(h),
        error: None,
    }
}

/// Set all edges to their average length.
fn equalize_edges(profile: Profile, sketch: &SketchGraph) -> ProfileRepairResponse {
    let pm = point_map(sketch);

    let lengths: Vec<f64> = profile.edge_ids.iter().filter_map(|eid| {
        let e = sketch.edges.iter().find(|e| &e.id == eid)?;
        let a = pm.get(&e.a)?;
        let b = pm.get(&e.b)?;
        Some(edge_length_mm(a, b))
    }).collect();

    if lengths.is_empty() {
        return ProfileRepairResponse {
            ok: false, profile_id: profile.id, repair_type: "EQUALIZE_EDGES".into(),
            repaired_points: vec![], avg_mm: None, width_mm: None, height_mm: None,
            error: Some("No edges".into()),
        };
    }

    let avg: f64 = lengths.iter().sum::<f64>() / lengths.len() as f64;

    // Scale each edge to avg length by moving the B endpoint along the edge direction
    let mut repaired: Vec<RepairedPoint> = Vec::new();
    for eid in &profile.edge_ids {
        let e = match sketch.edges.iter().find(|e| &e.id == eid) { Some(e) => e, None => continue };
        let pa = match pm.get(&e.a) { Some(p) => *p, None => continue };
        let pb = match pm.get(&e.b) { Some(p) => *p, None => continue };
        let cur = edge_length_mm(pa, pb);
        if cur < 1e-9 { continue; }
        let scale = avg / (cur * 1000.0); // avg in mm, cur in mm
        let nx = pa.x + (pb.x - pa.x) * (avg / (cur));
        let ny = pa.y + (pb.y - pa.y) * (avg / (cur));
        let nz = pa.z + (pb.z - pa.z) * (avg / (cur));
        repaired.push(RepairedPoint {
            id: pb.id.clone(),
            x: nx, y: ny, z: nz,
            x_mm: nx * 1000.0, y_mm: ny * 1000.0, z_mm: nz * 1000.0,
        });
    }

    ProfileRepairResponse {
        ok: true,
        profile_id: profile.id,
        repair_type: "EQUALIZE_EDGES".into(),
        repaired_points: repaired,
        avg_mm: Some(avg),
        width_mm: None, height_mm: None,
        error: None,
    }
}
