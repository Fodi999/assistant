// ── Sketch validation — light structural checks ──────────────────────────
//
// Mirrors the assertions performed by frontend `__validateSketchJSON`.
// Returns a structured result so callers can surface issues without parsing.

use serde::{Deserialize, Serialize};

use super::sketch::{SketchGraph, WorkingPlane};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub ok: bool,
    pub issues: Vec<ValidationIssue>,
    pub open_ends: usize,
    pub isolated_points: usize,
    pub duplicate_edges: usize,
}

impl ValidationResult {
    pub fn empty_ok() -> Self {
        Self {
            ok: true,
            issues: Vec::new(),
            open_ends: 0,
            isolated_points: 0,
            duplicate_edges: 0,
        }
    }
}

/// Run the full structural validation pipeline on a SketchGraph.
pub fn validate(sketch: &SketchGraph) -> ValidationResult {
    let mut issues = Vec::new();

    // Working plane.
    let plane = WorkingPlane::parse(&sketch.working_plane);
    if plane.is_none() {
        issues.push(ValidationIssue {
            code: "bad_plane".into(),
            message: format!("Invalid workingPlane: {}", sketch.working_plane),
        });
    }

    if !(sketch.grid_size.is_finite() && sketch.grid_size > 0.0) {
        issues.push(ValidationIssue {
            code: "bad_grid_size".into(),
            message: format!("gridSize must be a positive finite number, got {}", sketch.grid_size),
        });
    }

    // Point id uniqueness + grid uniqueness + plane constraint.
    let mut seen_ids: std::collections::HashSet<&str> = Default::default();
    let mut seen_grid: std::collections::HashSet<(i32, i32, i32)> = Default::default();
    for p in &sketch.points {
        if !seen_ids.insert(p.id.as_str()) {
            issues.push(ValidationIssue {
                code: "dup_point_id".into(),
                message: format!("Duplicate point id: {}", p.id),
            });
        }
        if !seen_grid.insert((p.gx, p.gy, p.gz)) {
            issues.push(ValidationIssue {
                code: "dup_point_grid".into(),
                message: format!("Duplicate grid coords on point {}: ({},{},{})", p.id, p.gx, p.gy, p.gz),
            });
        }
        if let Some(pl) = plane {
            if !pl.accepts_grid(p.gx, p.gy, p.gz) {
                issues.push(ValidationIssue {
                    code: "off_plane".into(),
                    message: format!(
                        "Point {} off plane {}: ({},{},{})",
                        p.id, pl.as_str(), p.gx, p.gy, p.gz
                    ),
                });
            }
        }
    }

    // Edges.
    let mut edge_ids: std::collections::HashSet<&str> = Default::default();
    let mut edge_pairs: std::collections::HashSet<(String, String)> = Default::default();
    let mut duplicate_edges = 0usize;
    for e in &sketch.edges {
        if !edge_ids.insert(e.id.as_str()) {
            issues.push(ValidationIssue {
                code: "dup_edge_id".into(),
                message: format!("Duplicate edge id: {}", e.id),
            });
        }
        if !seen_ids.contains(e.a.as_str()) {
            issues.push(ValidationIssue {
                code: "edge_missing_pt".into(),
                message: format!("Edge {} references unknown point a={}", e.id, e.a),
            });
        }
        if !seen_ids.contains(e.b.as_str()) {
            issues.push(ValidationIssue {
                code: "edge_missing_pt".into(),
                message: format!("Edge {} references unknown point b={}", e.id, e.b),
            });
        }
        if e.a == e.b {
            issues.push(ValidationIssue {
                code: "self_loop".into(),
                message: format!("Edge {} is a self-loop", e.id),
            });
        }
        let (lo, hi) = if e.a <= e.b {
            (e.a.clone(), e.b.clone())
        } else {
            (e.b.clone(), e.a.clone())
        };
        if !edge_pairs.insert((lo, hi)) {
            duplicate_edges += 1;
            issues.push(ValidationIssue {
                code: "dup_edge".into(),
                message: format!("Duplicate edge endpoints on {}: ({}, {})", e.id, e.a, e.b),
            });
        }
    }

    // Degree map for open-ends + isolated-points stats.
    let mut degree: std::collections::HashMap<&str, usize> = Default::default();
    for p in &sketch.points {
        degree.insert(p.id.as_str(), 0);
    }
    for e in &sketch.edges {
        if let Some(v) = degree.get_mut(e.a.as_str()) {
            *v += 1;
        }
        if let Some(v) = degree.get_mut(e.b.as_str()) {
            *v += 1;
        }
    }
    let isolated_points = degree.values().filter(|&&d| d == 0).count();
    let open_ends = degree.values().filter(|&&d| d == 1).count();

    ValidationResult {
        ok: issues.is_empty(),
        issues,
        open_ends,
        isolated_points,
        duplicate_edges,
    }
}
