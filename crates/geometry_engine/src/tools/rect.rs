// ── tools/rect.rs — Rectangle tool (native Rust) ─────────────────────────────
//
// Creates 4 corner points + 4 edges + HORIZONTAL/VERTICAL constraints.
// Grid coordinates in, SketchDelta out. Zero DOM, zero JS.
//
// Planes:
//   XZ: corners on Y=0, axes are gx (width) and gz (depth)
//   XY: corners on Z=0, axes are gx (width) and gy (height)
//   YZ: corners on X=0, axes are gy (height) and gz (depth)

use super::types::{SketchDelta, ToolConstraint, ToolEdge, ToolPoint};

/// Grid coords of the two diagonal corners and the working plane.
pub struct RectInput {
    pub gx1: i64,
    pub gy1: i64,
    pub gz1: i64,
    pub gx2: i64,
    pub gy2: i64,
    pub gz2: i64,
    pub plane: String,
    /// Starting point ID offset for unique ID generation.
    pub id_offset: u64,
}

/// Create a rectangle: 4 points + 4 edges + 4 H/V constraints.
pub fn create_rect(input: RectInput) -> SketchDelta {
    let RectInput { gx1, gy1, gz1, gx2, gy2, gz2, plane, id_offset } = input;

    // Validate: corners must differ in both working-plane axes
    let (valid, reason) = match plane.as_str() {
        "XY" => (gx1 != gx2 && gy1 != gy2, "XY: corners must differ in X and Y"),
        "YZ" => (gy1 != gy2 && gz1 != gz2, "YZ: corners must differ in Y and Z"),
        _    => (gx1 != gx2 && gz1 != gz2, "XZ: corners must differ in X and Z"),
    };
    if !valid {
        return SketchDelta::err(reason);
    }

    // Build 4 corner grid coords (clockwise: TL, TR, BR, BL viewed from +normal)
    let corners: [(i64, i64, i64); 4] = match plane.as_str() {
        "XY" => [
            (gx1, gy1, 0),
            (gx2, gy1, 0),
            (gx2, gy2, 0),
            (gx1, gy2, 0),
        ],
        "YZ" => [
            (0, gy1, gz1),
            (0, gy1, gz2),
            (0, gy2, gz2),
            (0, gy2, gz1),
        ],
        _ /* XZ */ => [
            (gx1, 0, gz1),
            (gx2, 0, gz1),
            (gx2, 0, gz2),
            (gx1, 0, gz2),
        ],
    };

    let mut delta = SketchDelta::empty();
    let base = id_offset;

    // 4 points
    let pt_ids: Vec<String> = (0..4)
        .map(|i| format!("rpt_{}", base + i as u64))
        .collect();

    for (i, (gx, gy, gz)) in corners.iter().enumerate() {
        delta.new_points.push(ToolPoint {
            id: pt_ids[i].clone(),
            gx: *gx,
            gy: *gy,
            gz: *gz,
        });
    }

    // 4 edges: 0→1 (top), 1→2 (right), 2→3 (bottom), 3→0 (left)
    let edge_ids: Vec<String> = (0..4)
        .map(|i| format!("re_{}", base + i as u64))
        .collect();

    for i in 0..4usize {
        delta.new_edges.push(ToolEdge {
            id: edge_ids[i].clone(),
            a: pt_ids[i].clone(),
            b: pt_ids[(i + 1) % 4].clone(),
            kind: "normal".into(),
        });
    }

    // 4 constraints: top=H, right=V, bottom=H, left=V
    let c_kinds = ["HORIZONTAL", "VERTICAL", "HORIZONTAL", "VERTICAL"];
    for i in 0..4usize {
        delta.new_constraints.push(ToolConstraint {
            id: format!("rc_{}", base + i as u64),
            kind: c_kinds[i].into(),
            target_type: "edge".into(),
            target_id: edge_ids[i].clone(),
            value: None,
        });
    }

    delta
}

/// Enforce square: all 4 sides equal to the shortest side.
/// Returns a delta with EQUAL_LENGTH + FIXED_LENGTH constraints
/// and corrected point positions.
pub struct MakeSquareInput {
    pub pt_ids: [String; 4],
    pub edge_ids: [String; 4],
    /// Current grid positions of the 4 points (same order as pt_ids).
    pub pts_gx: [i64; 4],
    pub pts_gy: [i64; 4],
    pub pts_gz: [i64; 4],
    pub plane: String,
    pub id_offset: u64,
}

pub fn make_square(input: MakeSquareInput) -> SketchDelta {
    let MakeSquareInput { pt_ids, edge_ids, pts_gx, pts_gy, pts_gz, plane, id_offset } = input;

    // Compute edge lengths (in grid units)
    let len = |i: usize| -> f64 {
        let j = (i + 1) % 4;
        let dx = (pts_gx[j] - pts_gx[i]) as f64;
        let dy = (pts_gy[j] - pts_gy[i]) as f64;
        let dz = (pts_gz[j] - pts_gz[i]) as f64;
        (dx * dx + dy * dy + dz * dz).sqrt()
    };

    let lengths: [f64; 4] = [len(0), len(1), len(2), len(3)];
    let target = lengths.iter().cloned().filter(|&l| l > 0.0)
        .fold(f64::INFINITY, f64::min);

    if !target.is_finite() || target <= 0.0 {
        return SketchDelta::err("make_square: degenerate edges");
    }

    let t = target.round() as i64;
    let base = id_offset;

    let mut delta = SketchDelta::empty();

    // Remove old EQUAL_LENGTH / FIXED_LENGTH for these edges (signal via removed_constraint_ids)
    // JS will filter constraints by targetId matching these edge_ids
    for eid in &edge_ids {
        delta.removed_constraint_ids.push(format!("EQUAL_LENGTH:{}", eid));
        delta.removed_constraint_ids.push(format!("FIXED_LENGTH:{}", eid));
    }

    // Add EQUAL_LENGTH between pairs 0-1, 1-2, 2-3
    for i in 0..3usize {
        delta.new_constraints.push(ToolConstraint {
            id: format!("sq_eq_{}", base + i as u64),
            kind: "EQUAL_LENGTH".into(),
            target_type: "edge".into(),
            target_id: format!("{},{}", edge_ids[i], edge_ids[i + 1]),
            value: None,
        });
    }

    // Fix first edge length as anchor
    delta.new_constraints.push(ToolConstraint {
        id: format!("sq_fix_{}", base),
        kind: "FIXED_LENGTH".into(),
        target_type: "edge".into(),
        target_id: edge_ids[0].clone(),
        value: Some(target),
    });

    // Force-correct geometry: p0 is anchor, build square from its position
    let (u_idx, v_idx): (usize, usize) = match plane.as_str() {
        "XY" => (0, 1), // gx, gy
        "YZ" => (1, 2), // gy, gz
        _    => (0, 2), // gx, gz  (XZ)
    };

    let get_uv = |i: usize| -> (i64, i64) {
        let coords = [pts_gx[i], pts_gy[i], pts_gz[i]];
        (coords[u_idx], coords[v_idx])
    };

    let (u0, v0) = get_uv(0);
    let (u1, v1) = get_uv(1);
    let (u3, v3) = get_uv(3);

    let du01 = (u1 - u0).signum();
    let dv01 = (v1 - v0).signum();
    let du03 = (u3 - u0).signum();
    let dv03 = (v3 - v0).signum();

    // Correct positions of p1, p2, p3 (p0 is anchor)
    let new_pts: [(i64, i64); 3] = [
        (u0 + du01 * t,           v0 + dv01 * t),
        (u0 + (du01 + du03) * t,  v0 + (dv01 + dv03) * t),
        (u0 + du03 * t,           v0 + dv03 * t),
    ];

    for (i, (nu, nv)) in new_pts.iter().enumerate() {
        let idx = i + 1; // p1, p2, p3
        let mut gx = pts_gx[idx];
        let mut gy = pts_gy[idx];
        let mut gz = pts_gz[idx];
        match plane.as_str() {
            "XY" => { gx = *nu; gy = *nv; gz = 0; }
            "YZ" => { gx = 0; gy = *nu; gz = *nv; }
            _    => { gx = *nu; gy = 0; gz = *nv; }
        }
        delta.new_points.push(ToolPoint { id: pt_ids[idx].clone(), gx, gy, gz });
    }

    delta
}
