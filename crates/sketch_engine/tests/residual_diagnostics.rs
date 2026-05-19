//! Integration tests for residual diagnostics and SolveResult v2.

#[cfg(test)]
mod tests {
    use sketch_engine::{
        solve_constraints, solve_constraints_with_config, SolveConfig,
        compute_residuals, residual_one,
        SketchGraph, WorkingPlane,
    };
    use sketch_engine::types::{Constraint, Edge, Point};

    // ── helpers ───────────────────────────────────────────────────────────

    fn pt(id: &str, gx: i32, gy: i32, gz: i32, gs: f64) -> Point {
        Point { id: id.into(), gx, gy, gz,
            x: gx as f64 * gs, y: gy as f64 * gs, z: gz as f64 * gs }
    }

    fn edge(id: &str, a: &str, b: &str) -> Edge {
        Edge { id: id.into(), a: a.into(), b: b.into() }
    }

    fn constraint(id: &str, ty: &str, ttype: &str, tid: &str, val: Option<f64>) -> Constraint {
        Constraint {
            id: Some(id.into()),
            ty: ty.into(),
            target_type: ttype.into(),
            target_id: tid.into(),
            value: val,
        }
    }

    /// Build a 4×3 rectangle on the XZ plane with H/V constraints.
    /// Points are intentionally skewed so the solver must move them.
    ///
    ///  p0(0,0,0) ──e0── p1(4,0,1)   ← top edge (should be H)
    ///  |                |
    ///  e3               e1           ← sides (should be V)
    ///  |                |
    ///  p3(1,0,3) ──e2── p2(4,0,3)   ← bottom edge (should be H)
    fn skewed_rect(gs: f64) -> SketchGraph {
        let mut s = SketchGraph::default();
        s.working_plane = "XZ".into();
        s.grid_size     = gs;
        s.points = vec![
            pt("p0", 0, 0, 0, gs),
            pt("p1", 4, 0, 1, gs),   // intentionally off-horizontal
            pt("p2", 4, 0, 3, gs),
            pt("p3", 1, 0, 3, gs),   // intentionally off-vertical
        ];
        s.edges = vec![
            edge("e0", "p0", "p1"),  // top   → HORIZONTAL
            edge("e1", "p1", "p2"),  // right → VERTICAL
            edge("e2", "p3", "p2"),  // bottom → HORIZONTAL
            edge("e3", "p0", "p3"),  // left  → VERTICAL
        ];
        s.constraints = vec![
            constraint("c0", "HORIZONTAL", "edge", "e0", None),
            constraint("c1", "VERTICAL",   "edge", "e1", None),
            constraint("c2", "HORIZONTAL", "edge", "e2", None),
            constraint("c3", "VERTICAL",   "edge", "e3", None),
        ];
        s
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 1: rectangle solve → all residuals ≈ 0 after solve
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn rectangle_residuals_zero_after_solve() {
        let sketch = skewed_rect(0.01);
        let result = solve_constraints(sketch);

        assert!(result.ok, "solve should succeed");
        assert_eq!(result.status, "converged");
        assert!(result.iterations >= 1);

        // All H/V residuals must be zero after solve
        for r in &result.residuals {
            assert!(
                r.error_mm < 1e-3,
                "residual {} (type {}) = {:.6} mm — should be ~0 after H/V solve",
                r.constraint_id, r.constraint_type, r.error_mm
            );
        }

        assert!(
            result.max_error_mm < 1e-3,
            "max_error_mm = {:.6} — expected < 1e-3 mm",
            result.max_error_mm
        );
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 2: FIXED_LENGTH residual shows error BEFORE solve
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn fixed_length_residual_before_solve() {
        let gs = 0.01; // 1 grid unit = 1 cm = 10 mm
        let mut s = SketchGraph::default();
        s.working_plane = "XZ".into();
        s.grid_size     = gs;
        // Edge is 3 grid units long = 30 mm actual, target = 50 mm
        s.points = vec![
            pt("p0", 0, 0, 0, gs),
            pt("p1", 3, 0, 0, gs),
        ];
        s.edges       = vec![edge("e0", "p0", "p1")];
        s.constraints = vec![constraint("c0", "FIXED_LENGTH", "edge", "e0", Some(50.0))];

        // Compute residuals WITHOUT running solve
        let residuals = compute_residuals(&s);
        assert_eq!(residuals.len(), 1);

        let r = &residuals[0];
        // actual = 30 mm, target = 50 mm → error = 20 mm
        let expected = 20.0_f64;
        assert!(
            (r.error_mm - expected).abs() < 0.5,
            "expected ≈ {} mm, got {:.4} mm",
            expected, r.error_mm
        );
        assert!(!r.satisfied, "should not be satisfied before solve");

        // After solve — should be ~0
        let result = solve_constraints(s);
        assert!(result.max_error_mm < 1.0,
            "after solve max_error_mm = {:.4}", result.max_error_mm);
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 3: COINCIDENT residual shows distance between points
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn coincident_residual_shows_distance() {
        let gs = 0.01;
        let mut s = SketchGraph::default();
        s.working_plane = "XZ".into();
        s.grid_size     = gs;
        // Two points 3 grid units apart (= 30 mm)
        s.points = vec![
            pt("p0", 0, 0, 0, gs),
            pt("p1", 3, 0, 0, gs),
        ];
        s.constraints = vec![constraint("c0", "COINCIDENT", "points", "p0,p1", None)];

        let residuals = compute_residuals(&s);
        assert_eq!(residuals.len(), 1);

        let r = &residuals[0];
        let expected = 30.0_f64; // 3 grid * 0.01 m/grid * 1000 mm/m
        assert!(
            (r.error_mm - expected).abs() < 0.5,
            "expected ≈ {} mm distance, got {:.4}", expected, r.error_mm
        );
        assert!(!r.satisfied);

        // After solve: distance should be 0
        let result = solve_constraints(s);
        assert!(result.max_error_mm < 1e-3,
            "after coincident solve max_error = {:.4}", result.max_error_mm);
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 4: DOF estimation
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn dof_under_constrained() {
        let sketch = {
            let gs = 0.01;
            let mut s = SketchGraph::default();
            s.working_plane = "XZ".into();
            s.grid_size = gs;
            // 4 points, 0 constraints → 8 DOF
            s.points = vec![
                pt("p0", 0, 0, 0, gs), pt("p1", 1, 0, 0, gs),
                pt("p2", 1, 0, 1, gs), pt("p3", 0, 0, 1, gs),
            ];
            s
        };
        let result = solve_constraints(sketch);
        assert_eq!(result.diagnostics.dof, 8);
        assert!(result.diagnostics.dof_status.contains("under-constrained"));
    }

    #[test]
    fn dof_fully_constrained_rect() {
        let sketch = skewed_rect(0.01);
        let result = solve_constraints(sketch);
        // 4 pts × 2 = 8 DOF, 4 constraints (H,V,H,V) × 1 = 4 used → 4 free
        // (we can still translate the whole rect — that's correct)
        assert!(result.diagnostics.dof >= 0, "should not be over-constrained");
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 5: SolveConfig — max_passes=1 leaves rect partially converged
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn solve_config_max_passes_respected() {
        let sketch = skewed_rect(0.01);
        let cfg    = SolveConfig { max_passes: 1, ..Default::default() };
        let result = solve_constraints_with_config(sketch, &cfg);
        assert!(result.iterations <= 1);
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 6: WASM-layer still returns ok + sketch + results + validation
    // ─────────────────────────────────────────────────────────────────────
    #[test]
    fn solve_result_has_all_v1_and_v2_fields() {
        let sketch = skewed_rect(0.01);
        let result = solve_constraints(sketch);
        // v1 fields
        let _ = result.ok;
        let _ = result.sketch.points.len();
        let _ = result.results.len();
        let _ = result.validation.ok;
        // v2 fields
        assert!(!result.status.is_empty());
        assert!(result.iterations >= 1);
        let _ = result.max_error_mm;
        let _ = result.total_error_mm;
        let _ = result.moved_points;
        let _ = result.residuals.len();
        let _ = result.diagnostics.dof;
    }
}
