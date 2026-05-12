// ── Closed-profile detection ──────────────────────────────────────────────
//
// Finds simple closed loops in the sketch graph. A profile is a cyclic chain
// of edges where every interior vertex has exactly degree 2. Only the smallest
// cycles are returned (one profile per fundamental cycle).
//
// Heuristic — sufficient for current phase (only rectangles / closed polylines
// from line tool). Full planar face extraction is deferred.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::sketch::SketchGraph;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    #[serde(rename = "pointIds")]
    pub point_ids: Vec<String>,
    #[serde(rename = "edgeIds")]
    pub edge_ids: Vec<String>,
    pub plane: String,
    pub closed: bool,
}

/// Find all closed profiles (cycles) in the sketch graph.
pub fn detect_profiles(sketch: &SketchGraph) -> Vec<Profile> {
    // Adjacency: point_id -> Vec<(neighbour_id, edge_id)>
    let mut adj: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for p in &sketch.points {
        adj.insert(p.id.clone(), Vec::new());
    }
    for e in &sketch.edges {
        if e.a == e.b {
            continue;
        }
        adj.entry(e.a.clone())
            .or_default()
            .push((e.b.clone(), e.id.clone()));
        adj.entry(e.b.clone())
            .or_default()
            .push((e.a.clone(), e.id.clone()));
    }

    let mut profiles: Vec<Profile> = Vec::new();
    let mut used_edges: HashSet<String> = HashSet::new();
    let mut counter: u32 = 0;

    // DFS cycle finder anchored on each point with degree >= 2.
    for start in sketch.points.iter().map(|p| p.id.clone()) {
        if adj.get(&start).map(|v| v.len()).unwrap_or(0) < 2 {
            continue;
        }
        if let Some(cycle) = find_cycle_from(&adj, &start, &used_edges) {
            for eid in &cycle.edge_ids {
                used_edges.insert(eid.clone());
            }
            counter += 1;
            profiles.push(Profile {
                id: format!("profile_{}", counter),
                point_ids: cycle.point_ids,
                edge_ids: cycle.edge_ids,
                plane: sketch.working_plane.clone(),
                closed: true,
            });
        }
    }

    profiles
}

struct CycleHit {
    point_ids: Vec<String>,
    edge_ids: Vec<String>,
}

/// DFS from `start`, finding the shortest cycle that does not reuse `forbidden_edges`.
fn find_cycle_from(
    adj: &HashMap<String, Vec<(String, String)>>,
    start: &str,
    forbidden_edges: &HashSet<String>,
) -> Option<CycleHit> {
    // Stack frames: (current_point, parent_edge, path_points, path_edges)
    let mut best: Option<CycleHit> = None;
    let max_depth = 64usize;

    fn dfs(
        adj: &HashMap<String, Vec<(String, String)>>,
        start: &str,
        cur: &str,
        forbidden: &HashSet<String>,
        path_pts: &mut Vec<String>,
        path_eds: &mut Vec<String>,
        visited_pts: &mut HashSet<String>,
        best: &mut Option<CycleHit>,
        max_depth: usize,
    ) {
        if path_pts.len() > max_depth {
            return;
        }
        if let Some(neigh) = adj.get(cur) {
            for (n, eid) in neigh {
                if forbidden.contains(eid) {
                    continue;
                }
                if path_eds.last().map_or(false, |last| last == eid) {
                    continue;
                }
                if n == start && path_pts.len() >= 3 {
                    // Found a cycle.
                    let candidate = CycleHit {
                        point_ids: path_pts.clone(),
                        edge_ids: {
                            let mut v = path_eds.clone();
                            v.push(eid.clone());
                            v
                        },
                    };
                    if best
                        .as_ref()
                        .map_or(true, |b| b.edge_ids.len() > candidate.edge_ids.len())
                    {
                        *best = Some(candidate);
                    }
                    continue;
                }
                if visited_pts.contains(n) {
                    continue;
                }
                visited_pts.insert(n.clone());
                path_pts.push(n.clone());
                path_eds.push(eid.clone());
                dfs(adj, start, n, forbidden, path_pts, path_eds, visited_pts, best, max_depth);
                path_pts.pop();
                path_eds.pop();
                visited_pts.remove(n);
            }
        }
    }

    let mut path_pts = vec![start.to_string()];
    let mut path_eds: Vec<String> = Vec::new();
    let mut visited_pts: HashSet<String> = HashSet::new();
    visited_pts.insert(start.to_string());
    dfs(
        adj,
        start,
        start,
        forbidden_edges,
        &mut path_pts,
        &mut path_eds,
        &mut visited_pts,
        &mut best,
        max_depth,
    );
    best
}
