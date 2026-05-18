//! Closed-profile (cycle) detection in the sketch graph.

use std::collections::{HashMap, HashSet};
use crate::types::{Profile, SketchGraph};

/// Find all closed profiles (simple cycles) in the sketch graph.
pub fn detect_profiles(sketch: &SketchGraph) -> Vec<Profile> {
    let mut adj: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for p in &sketch.points {
        adj.insert(p.id.clone(), Vec::new());
    }
    for e in &sketch.edges {
        if e.a == e.b { continue; }
        adj.entry(e.a.clone()).or_default().push((e.b.clone(), e.id.clone()));
        adj.entry(e.b.clone()).or_default().push((e.a.clone(), e.id.clone()));
    }

    let mut profiles: Vec<Profile> = Vec::new();
    let mut used_edges: HashSet<String> = HashSet::new();
    let mut counter: u32 = 0;

    for start in sketch.points.iter().map(|p| p.id.clone()) {
        if adj.get(&start).map(|v| v.len()).unwrap_or(0) < 2 { continue; }
        if let Some(cycle) = find_cycle_from(&adj, &start, &used_edges) {
            for eid in &cycle.edge_ids { used_edges.insert(eid.clone()); }
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

// ── Internal ──────────────────────────────────────────────────────────────

struct CycleHit {
    point_ids: Vec<String>,
    edge_ids:  Vec<String>,
}

fn find_cycle_from(
    adj: &HashMap<String, Vec<(String, String)>>,
    start: &str,
    forbidden_edges: &HashSet<String>,
) -> Option<CycleHit> {
    let mut best: Option<CycleHit> = None;
    const MAX_DEPTH: usize = 64;

    fn dfs(
        adj: &HashMap<String, Vec<(String, String)>>,
        start: &str,
        cur: &str,
        forbidden: &HashSet<String>,
        path_pts: &mut Vec<String>,
        path_eds: &mut Vec<String>,
        visited: &mut HashSet<String>,
        best: &mut Option<CycleHit>,
    ) {
        if path_pts.len() > MAX_DEPTH { return; }
        if let Some(neigh) = adj.get(cur) {
            for (n, eid) in neigh {
                if forbidden.contains(eid) { continue; }
                if path_eds.last().map_or(false, |l| l == eid) { continue; }
                if n == start && path_pts.len() >= 3 {
                    let candidate = CycleHit {
                        point_ids: path_pts.clone(),
                        edge_ids: { let mut v = path_eds.clone(); v.push(eid.clone()); v },
                    };
                    if best.as_ref().map_or(true, |b| b.edge_ids.len() > candidate.edge_ids.len()) {
                        *best = Some(candidate);
                    }
                    continue;
                }
                if visited.contains(n) { continue; }
                visited.insert(n.clone());
                path_pts.push(n.clone());
                path_eds.push(eid.clone());
                dfs(adj, start, n, forbidden, path_pts, path_eds, visited, best);
                path_pts.pop();
                path_eds.pop();
                visited.remove(n);
            }
        }
    }

    let mut path_pts = vec![start.to_string()];
    let mut path_eds: Vec<String> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(start.to_string());
    dfs(adj, start, start, forbidden_edges, &mut path_pts, &mut path_eds, &mut visited, &mut best, );
    best
}
